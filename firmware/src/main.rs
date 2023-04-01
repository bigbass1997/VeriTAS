
#![allow(unused_unsafe)]
#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;
extern crate cortex_m_rt;

use defmt::info;
use defmt_rtt as _;
use fugit::HertzU32;
use panic_probe as _;
use rp2040_hal::clocks::{Clock, ClocksManager, ClockSource};
use rp2040_hal::gpio::pin::bank0::Pins;
use rp2040_hal::multicore::{Multicore, Stack};
use rp2040_hal::pll::{PLLConfig, setup_pll_blocking};
use rp2040_hal::pll::common_configs::{PLL_USB_48MHZ};
use rp2040_hal::xosc::setup_xosc_blocking;
use rp2040_hal::{Sio, Watchdog};
use rp2040_hal::vector_table::VectorTable;
use rp2040_hal::pac::{CorePeripherals, Peripherals};
use rp2040_hal::sio::spinlock_reset;
use usb_device::class_prelude::UsbBusAllocator;
use crate::allocator::ALLOCATOR;
use crate::hal::gpio;
use crate::hal::gpio::{PIN_CNT_18, PIN_CNT_18_DIR};

mod allocator;
mod hal;
mod replaycore;
mod systems;
mod utilcore;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

const PLL_SYS_160MHZ: PLLConfig = PLLConfig {
        vco_freq: HertzU32::MHz(1440),
        refdiv: 1,
        post_div1: 3,
        post_div2: 3,
};

static mut CORE1_STACK: Stack<16384> = Stack::new();

/// Do not use outside of CORE0!
pub static mut VTABLE0: VectorTable = VectorTable::new();

#[export_name = "main"]
pub unsafe extern "C" fn main() -> ! {
    spinlock_reset();
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
    
    let mut clocks = ClocksManager::new(pac.CLOCKS);
    let xosc = setup_xosc_blocking(pac.XOSC, HertzU32::Hz(12000000)).ok().unwrap();
    let pll_sys = setup_pll_blocking(pac.PLL_SYS, xosc.operating_frequency().into(), PLL_SYS_160MHZ, &mut clocks, &mut pac.RESETS).ok().unwrap();
    let pll_usb = setup_pll_blocking(pac.PLL_USB, xosc.operating_frequency().into(), PLL_USB_48MHZ, &mut clocks, &mut pac.RESETS).ok().unwrap();
    clocks.reference_clock.configure_clock(&xosc, xosc.get_freq()).ok().unwrap();
    clocks.system_clock.configure_clock(&pll_sys, pll_sys.get_freq()).ok().unwrap();
    clocks.usb_clock.configure_clock(&pll_usb, pll_usb.get_freq()).ok().unwrap();
    clocks.adc_clock.configure_clock(&pll_usb, pll_usb.get_freq()).ok().unwrap();
    clocks.rtc_clock.configure_clock(&pll_usb, HertzU32::Hz(46875u32)).ok().unwrap();
    clocks.peripheral_clock.configure_clock(&clocks.system_clock, clocks.system_clock.freq()).ok().unwrap();
    watchdog.enable_tick_generation(12);
    
    let core = CorePeripherals::take().unwrap();
    let delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    
    let mut sio = Sio::new(pac.SIO);
    
    let _pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    
    let usb_bus = UsbBusAllocator::new(rp2040_hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    
    gpio::set_as_output(PIN_CNT_18, true, false);
    
    gpio::set_as_output(PIN_CNT_18_DIR, true, false);
    gpio::set_low(PIN_CNT_18_DIR);
    
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _ = core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || { utilcore::run(usb_bus) }).unwrap();
    
    replaycore::run(delay);
}