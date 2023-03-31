use cortex_m::asm::{delay, nop};
use cortex_m::delay::Delay;
use defmt::info;
use heapless::spsc::Queue;
use rp2040_pac::Interrupt::IO_IRQ_BANK0;
use rp2040_pac::{IO_BANK0, PPB, TIMER};
use crate::hal::gpio;
use crate::hal::gpio::{PIN_CNT_18, PIN_CNT_18_DIR, PIN_CNT_3, PIN_CNT_4, PIN_CNT_5, PIN_CNT_6, PIN_CNT_7, PIN_DETECT, PIN_DISPLAY_STROBE3};
use crate::replaycore::{ReplayState, VERITAS_MODE, VeritasMode};
use crate::VTABLE0;

/// Buffered list of controller inputs. 
pub static mut INPUT_BUFFER: Queue<[u8; 2], 1024> = Queue::new();
pub static mut REPLAY_STATE: ReplayState = ReplayState::new();

static mut LATCH_FILTER_US: u32 = 8000;
//static mut LATCH_FILTER_US: u32 = 6;
static mut OVERREAD: u8 = 1;

static mut LAST_LATCH: u64 = 0;
static mut LATCHED_INPUT: [u8; 2] = [0xFF, 0xFF];
static mut WORKING_INPUT: [u8; 2] = [0xFF, 0xFF];

const SER: [usize; 2] = [PIN_CNT_5, PIN_CNT_4];
const CLK: [usize; 2] = [PIN_CNT_7, PIN_CNT_6];
const LAT: usize = PIN_CNT_3;
//const RST: usize = PIN_CON_RESET;
const RST: usize = PIN_CNT_18;
/// set HIGH to enable
const RST_EN: usize = PIN_CNT_18_DIR;

/// Prepares the device to replay a TAS.
pub fn initialize() {
    gpio::set_as_output(PIN_DISPLAY_STROBE3, false, false);
    gpio::set_low(PIN_DISPLAY_STROBE3);
    
    gpio::set_low(PIN_DETECT);
    gpio::set_as_input(PIN_DETECT, false, true);
    
    for pin in SER { // Player 1 and 2 serial
        gpio::set_as_output(pin, false, false);
        gpio::set_high(pin);
    }
    
    for pin in CLK { // Player 1 and 2 clock
        gpio::set_as_input(pin, false, false);
    }
    
    gpio::set_as_input(LAT, false, false); // Shared latch
    
    gpio::set_as_output(RST, true, false); // Console reset (active-high)
    gpio::set_low(RST);
    
    gpio::set_high(RST_EN);
}

fn enable_interrupts() {
    unsafe {
        VTABLE0.register_handler(IO_IRQ_BANK0 as usize, io_irq_bank0_handler);
        
        (*IO_BANK0::ptr()).intr[1].write(|w| w.gpio7_edge_low().bit(true));
        (*IO_BANK0::ptr()).intr[1].write(|w| w.gpio6_edge_low().bit(true));
        (*IO_BANK0::ptr()).intr[1].write(|w| w.gpio3_edge_high().bit(true));
        
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio7_edge_low().bit(true)); // CLK[0]
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio6_edge_low().bit(true)); // CLK[1]
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio3_edge_high().bit(true)); // LAT
        (*PPB::ptr()).nvic_iser.write(|w| w.bits(1 << (IO_IRQ_BANK0 as u32)));
    }
}

fn disable_interrupts() {
    unsafe {
        (*PPB::ptr()).nvic_icer.write(|w| w.bits(1 << (IO_IRQ_BANK0 as u32)));
        
        while !INPUT_BUFFER.is_empty() {
            INPUT_BUFFER.dequeue().unwrap_or_default();
        }
    }
    
    info!("stopped NES replay");
}


pub fn run(delay: &mut Delay) {
    unsafe {
        initialize();
        
        info!("trans: {}", REPLAY_STATE.transitions.len());
        info!("first trans: {:?}", REPLAY_STATE.transitions.first());
        let first = INPUT_BUFFER.peek().unwrap_or(&[0xAA, 0x55]);
        info!("first input: {:02X} {:02X}", first[0], first[1]);
        
        info!("starting NES replay..");
        
        //io_irq_bank0_handler();
        
        gpio::set_high(RST);
        delay.delay_ms(50);
        gpio::set_low(RST);
        
        while gpio::is_high(LAT) { nop(); }
        
        enable_interrupts();
        
        while VERITAS_MODE == VeritasMode::ReplayNes {
            nop();
        }
        
        disable_interrupts();
        REPLAY_STATE.reset();
        
        gpio::set_low(RST);
        delay.delay_ms(10);
        gpio::set_low(RST_EN);
    }
}

#[inline(never)]
unsafe fn latch() {
    let timer = &*TIMER::ptr();
    let time = (timer.timelr.read().bits() as u64) | ((timer.timehr.read().bits() as u64) << 32);
    
    if LAST_LATCH + (LATCH_FILTER_US as u64) < time { // if latch filter expired, load next input
        LAST_LATCH = time;
        
        LATCHED_INPUT = INPUT_BUFFER.dequeue().unwrap_or([0xFF, 0xFF]);
        
        REPLAY_STATE.index_cur += 1;
        
        /*if let Some(transition) = REPLAY_STATE.next_transition() {
            //info!("cur: {}", REPLAY_STATE.index_cur);
            match transition {
                Transition::SoftReset => {
                    gpio::set_high(RST);
                    LATCHED_INPUT = INPUT_BUFFER.dequeue().unwrap_or([0xFF, 0xFF]);
                    info!("Transition: SoftReset");
                    gpio::set_low(RST);
                },
                Transition::PowerReset => (),
                Transition::Unsupported => (),
            }
        }*/
        if REPLAY_STATE.index_cur - 1 == REPLAY_STATE.index_len {
            VERITAS_MODE = VeritasMode::Idle;
        }
        
    }
    
    WORKING_INPUT = LATCHED_INPUT;
    
    
    //WORKING_INPUT = [0x7F, 0xFF];
    
    // set first bit's state
    for i in 0..2 {
        if WORKING_INPUT[i] & 0x80 != 0 {
            gpio::set_high(SER[i]);
        } else {
            gpio::set_low(SER[i]);
        }
    }
}

#[inline(never)]
unsafe fn clock(cnt: usize) {
    WORKING_INPUT[cnt] <<= 1;
    WORKING_INPUT[cnt] |= OVERREAD;
    
    delay(320); // CLOCK FILTER
    
    if WORKING_INPUT[cnt] & 0x80 != 0 {
        gpio::set_high(SER[cnt]);
    } else {
        gpio::set_low(SER[cnt]);
    }
}

extern "C" fn io_irq_bank0_handler() {
    gpio::set_high(PIN_DISPLAY_STROBE3); //debugging
    unsafe {
        let io_bank0 = &(*IO_BANK0::ptr());
        
        if io_bank0.proc0_ints[1].read().gpio3_edge_high().bits() { // LAT
            latch();
            
            io_bank0.intr[1].write(|w| w.gpio3_edge_high().bit(true));
        } else if io_bank0.proc0_ints[1].read().gpio7_edge_low().bits() { // CLK[0]
            clock(0);
            
            io_bank0.intr[1].write(|w| w.gpio7_edge_low().bit(true));
        } else if io_bank0.proc0_ints[1].read().gpio6_edge_low().bits() { // CLK[1]
            clock(1);
            
            io_bank0.intr[1].write(|w| w.gpio6_edge_low().bit(true));
        }
    }
    gpio::set_low(PIN_DISPLAY_STROBE3); //debugging
}