
#![allow(unused_unsafe)]
#![no_std]
#![no_main]

use defmt::info;
use embedded_time::rate::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::InputPin;
use panic_probe as _;

use rp_pico::hal::{clocks::Clock, pac, sio::Sio, watchdog::Watchdog};
use embedded_hal::watchdog::WatchdogDisable;
use rp_pico::hal::clocks::{ClocksManager, ClockSource};
use rp_pico::hal::gpio::FunctionUart;
use rp_pico::hal::pll::{PLLConfig, setup_pll_blocking};
use rp_pico::hal::pll::common_configs::{PLL_USB_48MHZ};
use rp_pico::hal::xosc::setup_xosc_blocking;
use rp_pico::hal::uart::{UartConfig, UartPeripheral};
use rp_pico::pac::io_bank0::gpio::gpio_ctrl::FUNCSEL_A;
use rp_pico::pac::PIO0;
use crate::hal::pio::{PioSel, SmSel};

mod hal;
mod systems;

pub const PLL_SYS_160MHZ: PLLConfig<Megahertz> = PLLConfig {
        vco_freq: Megahertz(1440),
        refdiv: 1,
        post_div1: 3,
        post_div2: 3,
};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum VeritasState {
    Initial,
    Idle,
    Replay,
}

pub static mut VERITAS_STATE: VeritasState = VeritasState::Idle;

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
    
    let sio = Sio::new(pac.SIO);
    
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
    let mut uart = UartPeripheral::new(
        pac.UART1,
        uart_pins,
        &mut pac.RESETS
    ).enable(uart_config, clocks.peripheral_clock.freq()).unwrap();
    
    
    hal::gpio::set_function(14, FUNCSEL_A::PIO0);
    hal::gpio::set_function(15, FUNCSEL_A::PIO0);
    //hal::gpio::set_function(16, FUNCSEL_A::SIO);
    let detect = pins.gpio16.into_floating_input();
    
    systems::n64::initialize();
    
    hal::pio::start(PioSel::Zero, SmSel::Zero);
    
    info!("starting..");
    info!("started? {}", (*PIO0::ptr()).ctrl.read().sm_enable().bits());
    
    loop {
        if detect.is_high().unwrap() {
            break;
        }
    }
    delay.delay_ms(100);
    
    systems::n64::run(&mut delay, &mut uart);
    
    loop {}
}
