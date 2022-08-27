use defmt::info;
use rp2040_hal::gpio::pin::FunctionUart;
use rp2040_hal::gpio::pin::bank0::{Gpio8, Gpio9};
use rp2040_hal::gpio::Pin;
use rp2040_hal::uart::{Enabled, UartPeripheral};
use rp2040_hal::vector_table::VectorTable;
use rp2040_pac::{Peripherals, UART1};
use crate::replaycore::{VERITAS_MODE, VeritasMode};

pub mod comms;

/// Do not use outside of CORE1!
pub static mut VTABLE1: VectorTable = VectorTable::new();

pub fn run(_uart: UartPeripheral<Enabled, UART1, (Pin<Gpio8, FunctionUart>, Pin<Gpio9, FunctionUart>)>) -> ! {
    unsafe {
        // VTABLE1 uses the same PAC, but the Cortex processor handles the underlying addresses
        // differently, because they are being accessed from within core1, instead of core0.
        let mut pac = Peripherals::steal();
        VTABLE1.init(&mut pac.PPB);
        VTABLE1.activate(&mut pac.PPB);
        
        loop {
            comms::check_uart();
            
            /*let mut cmd = [0u8];
            if let Ok(_) = uart.read_full_blocking(&mut cmd) {
                match cmd[0] {
                    0x01 => {
                        use crate::systems::n64::INPUT_BUFFER;
                        
                        let mut input = [0u8; 4];
                        uart.read_full_blocking(&mut input).unwrap_or_default();
                        if !INPUT_BUFFER.is_full() {
                            INPUT_BUFFER.enqueue([u32::from_be_bytes(input), 0, 0, 0]).unwrap();
                            uart.write_full_blocking(&[0x01]);
                        } else {
                            uart.write_full_blocking(&[0xFF]);
                        }
                    },
                    0x02 => {
                        VERITAS_MODE = VeritasMode::ReplayN64;
                        uart.write_full_blocking(&[0x20]);
                    },
                    0x03 => {
                        use crate::systems::nes::INPUT_BUFFER;
                        
                        let mut input = [0u8; 2];
                        uart.read_full_blocking(&mut input).unwrap_or_default();
                        if !INPUT_BUFFER.is_full() {
                            INPUT_BUFFER.enqueue(input).unwrap();
                            uart.write_full_blocking(&[0x03]);
                        } else {
                            uart.write_full_blocking(&[0xFF]);
                        }
                    },
                    0x04 => {
                        VERITAS_MODE = VeritasMode::ReplayNes;
                        uart.write_full_blocking(&[0x40]);
                    },
                    0xFE => {
                        info!("ping");
                        uart.write_full_blocking(&[0xEF]);
                    },
                    _ => ()
                }
            }*/
        }
    }
}