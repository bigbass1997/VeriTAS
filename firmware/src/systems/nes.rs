use cortex_m::asm::nop;
use cortex_m::delay::Delay;
use defmt::info;
use heapless::spsc::Queue;
use rp2040_pac::io_bank0::gpio::gpio_ctrl::FUNCSEL_A;
use rp2040_pac::SIO;
use crate::hal::gpio;
use crate::replaycore::{VERITAS_MODE, VeritasMode};

/// Buffered list of controller inputs. 
pub static mut INPUT_BUFFER: Queue<[u8; 2], 1024> = Queue::new();

//static mut READ_BYTE_VECTOR: u8 = 0;
//static mut WRITE_BYTES_VECTOR: u8 = 0;

const SER_PIN: usize = 14;

/// Prepares the device to replay a TAS.
pub fn initialize() {
    // Data
    gpio::set_function(SER_PIN, FUNCSEL_A::SIO);
    gpio::set_pull_down_enable(SER_PIN, false);
    gpio::set_pull_up_enable(SER_PIN, false);
    gpio::set_output_disable(SER_PIN, false);
    gpio::set_input_enable(SER_PIN, false);
    gpio::set_sio_output_enable(SER_PIN, true);
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
}


pub fn run(delay: &mut Delay) {
    unsafe {
        initialize();
        
        info!("starting NES replay..");
        info!("state: {:#010X}", (*SIO::ptr()).gpio_out.read().bits());
        
        while VERITAS_MODE == VeritasMode::ReplayNes {
            while gpio::is_low(16) { nop() }
            //gpio::set_low(SER_PIN);
            //delay.delay_us(100);
            //gpio::set_high(SER_PIN);
            
            for _ in 0..8 {
                while gpio::is_high(15) { nop() }
                gpio::set_low(14);
                delay.delay_us(1);
                gpio::set_high(14);
            }
        }
    }
}