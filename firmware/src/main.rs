
#![allow(unused_unsafe)]
#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;
extern crate cortex_m_rt;

use defmt::info;
use defmt_rtt as _;
use panic_probe as _;
use embedded_time::rate::*;
use embedded_hal::watchdog::WatchdogDisable;
use rp2040_hal::clocks::{Clock, ClocksManager, ClockSource};
use rp2040_hal::gpio::pin::bank0::Pins;
use rp2040_hal::gpio::FunctionUart;
use rp2040_hal::multicore::{Multicore, Stack};
use rp2040_hal::pll::{PLLConfig, setup_pll_blocking};
use rp2040_hal::pll::common_configs::{PLL_USB_48MHZ};
use rp2040_hal::xosc::setup_xosc_blocking;
use rp2040_hal::uart::{UartConfig, UartPeripheral};
use rp2040_hal::{Sio, Watchdog};
use rp2040_hal::vector_table::VectorTable;
use rp2040_pac::{CorePeripherals, Peripherals};
use crate::allocator::ALLOCATOR;

mod allocator;
mod hal;
mod replaycore;
mod systems;
mod utilcore;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

const PLL_SYS_160MHZ: PLLConfig<Megahertz> = PLLConfig {
        vco_freq: Megahertz(1440),
        refdiv: 1,
        post_div1: 3,
        post_div2: 3,
};

static mut CORE1_STACK: Stack<16384> = Stack::new();

/// Do not use outside of CORE0!
pub static mut VTABLE0: VectorTable = VectorTable::new();

#[export_name = "main"]
pub unsafe extern "C" fn main() -> ! {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 16384;
        static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP_SIZE) }
    }
    
    let mut pac = Peripherals::take().unwrap();
    
    VTABLE0.init(&mut pac.PPB);
    VTABLE0.activate(&mut pac.PPB);
    
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    watchdog.disable();
    
    let mut clocks = ClocksManager::new(pac.CLOCKS);
    let xosc = setup_xosc_blocking(pac.XOSC, 12000000.Hz()).ok().unwrap();
    let pll_sys = setup_pll_blocking(pac.PLL_SYS, xosc.operating_frequency().into(), PLL_SYS_160MHZ, &mut clocks, &mut pac.RESETS).ok().unwrap();
    let pll_usb = setup_pll_blocking(pac.PLL_USB, xosc.operating_frequency().into(), PLL_USB_48MHZ, &mut clocks, &mut pac.RESETS).ok().unwrap();
    clocks.reference_clock.configure_clock(&xosc, xosc.get_freq()).ok().unwrap();
    clocks.system_clock.configure_clock(&pll_sys, pll_sys.get_freq()).ok().unwrap();
    clocks.usb_clock.configure_clock(&pll_usb, pll_usb.get_freq()).ok().unwrap();
    clocks.adc_clock.configure_clock(&pll_usb, pll_usb.get_freq()).ok().unwrap();
    clocks.rtc_clock.configure_clock(&pll_usb, 46875u32.Hz()).ok().unwrap();
    clocks.peripheral_clock.configure_clock(&clocks.system_clock, clocks.system_clock.freq()).ok().unwrap();
    
    let core = CorePeripherals::take().unwrap();
    let delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());
    
    let mut sio = Sio::new(pac.SIO);
    
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    
    let uart_pins = (
        pins.gpio8.into_mode::<FunctionUart>(),
        pins.gpio9.into_mode::<FunctionUart>(),
    );
    
    let mut uart_config = UartConfig::default();
    uart_config.baudrate = Baud(500000);
    let uart = UartPeripheral::new(
        pac.UART1,
        uart_pins,
        &mut pac.RESETS
    ).enable(uart_config, clocks.peripheral_clock.freq()).unwrap();
    
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _ = core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || { utilcore::run(uart) }).unwrap();
    
    replaycore::run(delay);
}