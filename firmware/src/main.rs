
#![allow(unused_unsafe)]
#![no_std]
#![no_main]

use defmt::info;
use embedded_time::rate::*;
use defmt_rtt as _;
use panic_probe as _;

use rp_pico::hal::{clocks::Clock, pac, sio::Sio, watchdog::Watchdog};
use embedded_hal::watchdog::WatchdogDisable;
use rp_pico::hal::clocks::{ClocksManager, ClockSource};
use rp_pico::hal::gpio::bank0::{Gpio8, Gpio9};
use rp_pico::hal::gpio::{FunctionUart, Pin};
use rp_pico::hal::multicore::{Multicore, Stack};
use rp_pico::hal::pll::{PLLConfig, setup_pll_blocking};
use rp_pico::hal::pll::common_configs::{PLL_USB_48MHZ};
use rp_pico::hal::xosc::setup_xosc_blocking;
use rp_pico::hal::uart::{Enabled, UartConfig, UartPeripheral};

mod hal;
mod replaycore;
mod systems;
mod utilcore;

pub const PLL_SYS_160MHZ: PLLConfig<Megahertz> = PLLConfig {
        vco_freq: Megahertz(1440),
        refdiv: 1,
        post_div1: 3,
        post_div2: 3,
};

static mut CORE1_STACK: Stack<16384> = Stack::new();


#[export_name = "main"]
pub unsafe extern "C" fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    
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
    
    let core = pac::CorePeripherals::take().unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());
    
    let mut sio = Sio::new(pac.SIO);
    
    let pins = rp_pico::Pins::new(
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