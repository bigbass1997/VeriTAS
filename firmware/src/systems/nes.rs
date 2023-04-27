use cortex_m::asm::{delay, nop};
use cortex_m::delay::Delay;
use defmt::info;
use heapless::spsc::Queue;
use rp2040_pac::Interrupt::{IO_IRQ_BANK0, TIMER_IRQ_0};
use rp2040_pac::{IO_BANK0, PPB, TIMER};
use crate::hal::gpio;
use crate::hal::gpio::{PIN_CNT_18, PIN_CNT_18_DIR, PIN_CNT_3, PIN_CNT_4, PIN_CNT_5, PIN_CNT_6, PIN_CNT_7, PIN_DETECT};
use crate::replaycore::{Transition, VERITAS_MODE, REPLAY_STATE, VeritasMode};
use crate::utilcore::displays;
use crate::utilcore::displays::Port;
use crate::VTABLE0;

/// Buffered list of controller inputs. 
pub static mut INPUT_BUFFER: Queue<[u8; 2], 1024> = Queue::new();

pub static mut LATCH_FILTER_US: u32 = 8000; //TODO: Write a detection procedure to relay to the user what the time between latch and 8th clock is.
static mut OVERREAD: u8 = 1;

static mut ALARM_ACTIVATED: bool = false;
static mut FRAME_INPUT: [u8; 2] = [0xFF, 0xFF];
static mut WORKING_INPUT: [u8; 2] = [0xFF, 0xFF];

const SER: [usize; 2] = [PIN_CNT_5, PIN_CNT_4];
const CLK: [usize; 2] = [PIN_CNT_7, PIN_CNT_6];
const LAT: usize = PIN_CNT_3;
const RST: usize = PIN_CNT_18;
/// set HIGH to enable
const RST_EN: usize = PIN_CNT_18_DIR;

/// Prepares the device to replay a TAS.
pub fn initialize() {
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
    
    unsafe {
        FRAME_INPUT = INPUT_BUFFER.dequeue().unwrap_or([0xFF, 0xFF]);
        
        displays::set_display(Port::Display0, &[FRAME_INPUT[0] ^ 0xFF]);
        displays::set_display(Port::Display1, &[FRAME_INPUT[1] ^ 0xFF]);
        
        ALARM_ACTIVATED = false;
    }
}

fn enable_interrupts() {
    cortex_m::interrupt::free(|_| unsafe {
        VTABLE0.register_handler(IO_IRQ_BANK0 as usize, io_irq_bank0_handler);
        
        (*IO_BANK0::ptr()).intr[1].write(|w| w.gpio7_edge_low().bit(true));
        (*IO_BANK0::ptr()).intr[1].write(|w| w.gpio6_edge_low().bit(true));
        (*IO_BANK0::ptr()).intr[1].write(|w| w.gpio3_edge_high().bit(true));
        
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio7_edge_low().bit(true)); // CLK[0]
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio6_edge_low().bit(true)); // CLK[1]
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio3_edge_high().bit(true)); // LAT
        (*PPB::ptr()).nvic_iser.write(|w| w.bits(1 << (IO_IRQ_BANK0 as u32)));
        
        
        VTABLE0.register_handler(TIMER_IRQ_0 as usize, timer_irq_0_handler);
        
        (*TIMER::ptr()).intr.write(|w| w.alarm_0().bit(true));
        
        (*TIMER::ptr()).inte.modify(|_, w| w.alarm_0().bit(true));
        (*PPB::ptr()).nvic_iser.write(|w| w.bits(1 << (TIMER_IRQ_0 as u32)));
    });
}

fn disable_interrupts() {
    cortex_m::interrupt::free(|_| unsafe {
        (*PPB::ptr()).nvic_icer.write(|w| w.bits(1 << (IO_IRQ_BANK0 as u32)));
        (*PPB::ptr()).nvic_icer.write(|w| w.bits(1 << (TIMER_IRQ_0 as u32)));
        
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio7_edge_low().bit(false)); // CLK[0]
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio6_edge_low().bit(false)); // CLK[1]
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio3_edge_high().bit(false)); // LAT
        
        (*TIMER::ptr()).inte.modify(|r, w| w.bits(r.bits() & 0b1110));
    });
}


pub fn run(delay: &mut Delay) {
    unsafe {
        initialize();
        
        info!("trans: {}", REPLAY_STATE.transitions.len());
        info!("first trans: {:?}", REPLAY_STATE.transitions.first());
        let first = INPUT_BUFFER.peek().unwrap_or(&[0xAA, 0x55]);
        info!("first input: {:02X} {:02X}", first[0], first[1]);
        
        info!("starting NES replay..");
        
        if REPLAY_STATE.use_initial_reset {
            gpio::set_high(RST);
            delay.delay_ms(50);
            gpio::set_low(RST);
        }
        
        delay.delay_ms(5);
        
        enable_interrupts();
        
        while VERITAS_MODE == VeritasMode::ReplayNes {
            nop();
        }
        
        disable_interrupts();
        while !INPUT_BUFFER.is_empty() {
            INPUT_BUFFER.dequeue().unwrap_or_default();
        }
        REPLAY_STATE.reset();
        
        displays::set_display(Port::Display0, &[0x00]);
        displays::set_display(Port::Display1, &[0x00]);
        
        gpio::set_low(RST);
        delay.delay_ms(10);
        gpio::set_low(RST_EN);
        
        info!("stopped NES replay");
    }
}

#[link_section = ".ram_code"]
#[inline(always)]
unsafe fn latch() {
    if !ALARM_ACTIVATED {
        ALARM_ACTIVATED = true;
        (*TIMER::ptr()).alarm0.write(|w| w.bits((*TIMER::ptr()).timerawl.read().bits().wrapping_add(LATCH_FILTER_US)));
    }
    
    WORKING_INPUT = FRAME_INPUT;
    
    // set first bit's state
    for i in 0..2 {
        if WORKING_INPUT[i] & 0x80 != 0 {
            gpio::set_high(SER[i]);
        } else {
            gpio::set_low(SER[i]);
        }
    }
}

#[link_section = ".ram_code"]
#[inline(always)]
unsafe fn clock(cnt: usize) {
    WORKING_INPUT[cnt] <<= 1;
    WORKING_INPUT[cnt] |= OVERREAD;
    
    delay(200); // CLOCK FILTER
    
    if WORKING_INPUT[cnt] & 0x80 != 0 {
        gpio::set_high(SER[cnt]);
    } else {
        gpio::set_low(SER[cnt]);
    }
}

#[link_section = ".ram_code"]
extern "C" fn io_irq_bank0_handler() {
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
}

#[link_section = ".ram_code"]
extern "C" fn timer_irq_0_handler() {
    unsafe {
        ALARM_ACTIVATED = false;
        
        if let Some(tra) = REPLAY_STATE.next_transition() {
            match tra {
                Transition::SoftReset => cortex_m::interrupt::free(|_| {
                    disable_interrupts();
                    
                    gpio::set_high(RST);
                    delay(5332558);
                    gpio::set_low(RST);
                    delay(10665);
                    
                    enable_interrupts();
                }),
                _ => (),
            }
        } else {
            FRAME_INPUT = INPUT_BUFFER.dequeue().unwrap_or([0xFF, 0xFF]);
            
            displays::set_display(Port::Display0, &[FRAME_INPUT[0] ^ 0xFF]);
            displays::set_display(Port::Display1, &[FRAME_INPUT[1] ^ 0xFF]);
            
            if REPLAY_STATE.index_cur == REPLAY_STATE.index_len {
                VERITAS_MODE = VeritasMode::Idle;
                info!("Replay ended!");
            } else {
                REPLAY_STATE.index_cur += 1;
            }
        }
        
        (*TIMER::ptr()).intr.write(|w| w.alarm_0().bit(true));
    }
}