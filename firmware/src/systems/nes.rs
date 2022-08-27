use cortex_m::asm::{delay, nop};
use cortex_m::delay::Delay;
use defmt::info;
use heapless::spsc::Queue;
use rp2040_pac::Interrupt::IO_IRQ_BANK0;
use rp2040_pac::{IO_BANK0, PPB, SIO, TIMER};
use crate::hal::gpio;
use crate::replaycore::{ReplayState, VERITAS_MODE, VeritasMode};
use crate::VTABLE0;

/// Buffered list of controller inputs. 
pub static mut INPUT_BUFFER: Queue<[u8; 2], 1024> = Queue::new();
pub static mut REPLAY_STATE: ReplayState = ReplayState::new();

static mut LATCH_FILTER_US: u32 = 8000;
static mut OVERREAD: u8 = 1;

static mut LAST_LATCH: u64 = 0;
static mut LATCHED_INPUT: [u8; 2] = [0xFF, 0xFF];
static mut WORKING_INPUT: [u8; 2] = [0xFF, 0xFF];

const SER: [usize; 2] = [14, 17];
const CLK: [usize; 2] = [15, 18];
const LAT: usize = 16;
const RST: usize = 13;

const DEBUG: usize = 12;

/// Prepares the device to replay a TAS.
pub fn initialize() {
    gpio::set_as_output(DEBUG, false, false); // Debug
    gpio::set_high(DEBUG);
    
    for pin in SER { // Player 1 and 2 serial
        gpio::set_as_output(pin, false, false);
        gpio::set_high(pin);
    }
    
    for pin in CLK { // Player 1 and 2 clock
        gpio::set_as_input(pin, false, false);
    }
    
    gpio::set_as_input(LAT, false, false); // Shared latch
    
    gpio::set_as_output(RST, false, true); // Console reset (active-high)
    gpio::set_low(RST);
}

fn enable_interrupts() {
    unsafe {
        VTABLE0.register_handler(IO_IRQ_BANK0 as usize, io_irq_bank0_handler);
        
        (*IO_BANK0::ptr()).intr[1].write(|w| w.gpio7_edge_low().bit(true));
        (*IO_BANK0::ptr()).intr[2].write(|w| w.gpio2_edge_low().bit(true));
        (*IO_BANK0::ptr()).intr[2].write(|w| w.gpio0_edge_high().bit(true));
        
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio7_edge_low().bit(true)); // CLK[0]
        (*IO_BANK0::ptr()).proc0_inte[2].modify(|_, w| w.gpio2_edge_low().bit(true)); // CLK[1]
        (*IO_BANK0::ptr()).proc0_inte[2].modify(|_, w| w.gpio0_edge_high().bit(true)); // LAT
        (*PPB::ptr()).nvic_iser.write(|w| w.bits(1 << (IO_IRQ_BANK0 as u32)))
    }
}


pub fn run(_delay: &mut Delay) {
    unsafe {
        initialize();
        
        gpio::set_high(RST);
        info!("starting NES replay..");
        info!("state: {:#010X}", (*SIO::ptr()).gpio_out.read().bits());
        gpio::set_low(RST);
        
        while gpio::is_high(LAT) { nop(); }
        
        enable_interrupts();
        
        while VERITAS_MODE == VeritasMode::ReplayNes {
            nop()
        }
    }
}

#[inline(always)]
unsafe fn latch() {
    let timer = &*TIMER::ptr();
    let time = (timer.timelr.read().bits() as u64) | ((timer.timehr.read().bits() as u64) << 32);
    
    if LAST_LATCH + (LATCH_FILTER_US as u64) < time { // if latch filter expired, load next input
        LAST_LATCH = time;
        
        LATCHED_INPUT = INPUT_BUFFER.dequeue().unwrap_or_else(|| [0xFF, 0xFF]);
        REPLAY_STATE.index_cur += 1;
    }
    
    WORKING_INPUT = LATCHED_INPUT;
    
    // set first bit's state
    for i in 0..2 {
        if WORKING_INPUT[i] & 0x80 != 0 {
            gpio::set_high(SER[i]);
        } else {
            gpio::set_low(SER[i]);
        }
    }
}

#[inline(always)]
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
    unsafe {
        let io_bank0 = &(*IO_BANK0::ptr());
        if io_bank0.proc0_ints[1].read().gpio7_edge_low().bits() { // CLK[0]
            clock(0);
            
            io_bank0.intr[1].write(|w| w.gpio7_edge_low().bit(true));
        } else if io_bank0.proc0_ints[2].read().gpio2_edge_low().bits() { // CLK[1]
            clock(1);
            
            io_bank0.intr[2].write(|w| w.gpio2_edge_low().bit(true));
        } else if io_bank0.proc0_ints[2].read().gpio0_edge_high().bits() { // LAT
            latch();
            
            io_bank0.intr[2].write(|w| w.gpio0_edge_high().bit(true));
        }
    }
}