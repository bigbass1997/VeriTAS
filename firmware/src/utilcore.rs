use defmt::info;
use rp2040_hal::usb::UsbBus;
use rp2040_hal::vector_table::VectorTable;
use rp2040_hal::pac::Peripherals;
use rp2040_pac::Interrupt::USBCTRL_IRQ;
use usb_device::class_prelude::UsbBusAllocator;

pub mod comms;
pub mod displays;

/// Do not use outside of CORE1!
pub static mut VTABLE1: VectorTable = VectorTable::new();

#[link_section = ".ram_code"]
pub fn run(usb_bus: UsbBusAllocator<UsbBus>) -> ! {
    unsafe {
        // VTABLE1 uses the same PAC, but the Cortex processor handles the underlying addresses
        // differently, because they are being accessed from within core1, instead of core0.
        let mut pac = Peripherals::steal();
        VTABLE1.init(&mut pac.PPB);
        VTABLE1.activate(&mut pac.PPB);
        
        displays::initialize();
        
        info!("Initializing usb...");
        comms::init_usb(usb_bus);
        
        VTABLE1.register_handler(USBCTRL_IRQ as usize, usbctrl_irq_handler);
        pac.PPB.nvic_iser.write(|w| w.bits(1 << (USBCTRL_IRQ as u32)));
        info!("Done with usb");
        
        loop {
            //comms::check_usb();
            displays::check_displays();
        }
    }
}

#[link_section = ".ram_code"]
extern "C" fn usbctrl_irq_handler() {
    unsafe {
        crate::gpio::set_high(crate::gpio::PIN_DISPLAY_STROBE2); //debugging
        comms::check_usb();
        crate::gpio::set_low(crate::gpio::PIN_DISPLAY_STROBE2); //debugging
    }
}
