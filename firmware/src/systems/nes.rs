use cortex_m::asm::{delay, nop};
use cortex_m::delay::Delay;
use defmt::info;
use heapless::spsc::Queue;
use rp2040_pac::Interrupt::IO_IRQ_BANK0;
use rp2040_pac::io_bank0::gpio::gpio_ctrl::FUNCSEL_A;
use rp2040_pac::{IO_BANK0, PPB, SIO, TIMER};
use crate::hal::gpio;
use crate::replaycore::{VERITAS_MODE, VeritasMode};
use crate::VTABLE0;

/// Buffered list of controller inputs. 
pub static mut INPUT_BUFFER: Queue<[u8; 2], 1024> = Queue::new();

static mut LATCH_FILTER_US: u32 = 8000;
static mut OVERREAD: u8 = 1;

static mut LAST_LATCH: u64 = 0;
static mut LATCHED_INPUT: [u8; 2] = [0xFF, 0xFF];
static mut WORKING_INPUT: [u8; 2] = [0xFF, 0xFF];
//static mut READ_BYTE_VECTOR: u8 = 0;
//static mut WRITE_BYTES_VECTOR: u8 = 0;

const SER_PIN: usize = 14;

/// Prepares the device to replay a TAS.
pub fn initialize() {
    // Reset
    gpio::set_pull_down_enable(13, true);
    gpio::set_pull_up_enable(13, false);
    gpio::set_output_disable(13, false);
    gpio::set_input_enable(13, false);
    gpio::set_sio_output_enable(13, true);
    gpio::set_function(13, FUNCSEL_A::SIO);
    gpio::set_low(13);
    
    // Data
    gpio::set_pull_down_enable(SER_PIN, false);
    gpio::set_pull_up_enable(SER_PIN, false);
    gpio::set_output_disable(SER_PIN, false);
    gpio::set_input_enable(SER_PIN, false);
    gpio::set_sio_output_enable(SER_PIN, true);
    gpio::set_function(SER_PIN, FUNCSEL_A::SIO);
    gpio::set_high(SER_PIN);
    
    // Clock
    gpio::set_function(15, FUNCSEL_A::SIO);
    gpio::set_pull_down_enable(15, false);
    gpio::set_pull_up_enable(15, false);
    gpio::set_output_disable(15, true);
    gpio::set_input_enable(15, true);
    gpio::set_sio_output_enable(15, false);
    
    // Latch
    gpio::set_function(16, FUNCSEL_A::SIO);
    gpio::set_pull_down_enable(16, false);
    gpio::set_pull_up_enable(16, false);
    gpio::set_output_disable(16, true);
    gpio::set_input_enable(16, true);
    gpio::set_sio_output_enable(16, false);
    
    gpio::set_high(13);
    
    unsafe {
        VTABLE0.register_handler(IO_IRQ_BANK0 as usize, io_irq_bank0_handler);
        
        (*IO_BANK0::ptr()).proc0_inte[2].modify(|_, w| w.gpio0_edge_high().bit(true));
        (*IO_BANK0::ptr()).proc0_inte[1].modify(|_, w| w.gpio7_edge_low().bit(true));
        (*PPB::ptr()).nvic_iser.write(|w| w.bits(1 << (IO_IRQ_BANK0 as u32)))
    }
}


pub fn run(delay: &mut Delay) {
    unsafe {
        initialize();
        
        info!("starting NES replay..");
        info!("state: {:#010X}", (*SIO::ptr()).gpio_out.read().bits());
        gpio::set_low(13);
        
        while VERITAS_MODE == VeritasMode::ReplayNes {
            //while gpio::is_low(16) { nop() }
            //gpio::set_low(SER_PIN);
            //delay.delay_us(100);
            //gpio::set_high(SER_PIN);
            
            /*for _ in 0..8 {
                while gpio::is_high(15) { nop() }
                gpio::set_low(14);
                delay.delay_us(1);
                gpio::set_high(14);
            }*/
            nop()
        }
    }
}

#[inline(always)]
unsafe fn latch() {
    let timer = &*TIMER::ptr();
    
    let time = (timer.timelr.read().bits() as u64) | ((timer.timehr.read().bits() as u64) << 32);
    if LAST_LATCH + (LATCH_FILTER_US as u64) < time { // load next input
        //info!("{}", time - LAST_LATCH);
        LAST_LATCH = time;
        
        LATCHED_INPUT = INPUT_BUFFER.dequeue().unwrap_or_else(|| [0xFF, 0xFF]);
        /*gpio::set_low(SER_PIN);
        delay(10);
        gpio::set_high(SER_PIN);*/
    }
    
    WORKING_INPUT = LATCHED_INPUT;
    
    // TODO: Support 2 controllers
    if WORKING_INPUT[0] & 0x80 != 0 {
        gpio::set_high(SER_PIN);
    } else {
        gpio::set_low(SER_PIN);
    }
}

#[inline(always)]
unsafe fn clock(cnt: usize) {
    WORKING_INPUT[cnt] <<= 1;
    WORKING_INPUT[cnt] |= OVERREAD;
    
    delay(320); // CLOCK FILTER
    
    // TODO: Support 2 controllers (place pin numbers in [usize; 2] for indexing)
    if WORKING_INPUT[cnt] & 0x80 != 0 {
        gpio::set_high(SER_PIN);
    } else {
        gpio::set_low(SER_PIN);
    }
}

extern "C" fn io_irq_bank0_handler() {
    unsafe {
        let io_bank0 = &(*IO_BANK0::ptr());
        if io_bank0.proc0_ints[1].read().gpio7_edge_low().bits() { // check for interrupt on pin 15
            clock(0);
            
            io_bank0.intr[1].write(|w| w.gpio7_edge_low().bit(true)); // clear interrupt
        } else if io_bank0.proc0_ints[2].read().gpio0_edge_high().bits() { // check for interrupt on pin 16
            latch();
            
            io_bank0.intr[2].write(|w| w.gpio0_edge_high().bit(true)); // clear interrupt
        }
    }
}