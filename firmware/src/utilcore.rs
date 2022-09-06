use rp2040_hal::usb::UsbBus;
use rp2040_hal::vector_table::VectorTable;
use rp2040_pac::Peripherals;
use usb_device::class_prelude::UsbBusAllocator;

pub mod comms;

/// Do not use outside of CORE1!
pub static mut VTABLE1: VectorTable = VectorTable::new();

pub fn run(usb_bus: UsbBusAllocator<UsbBus>) -> ! {
    unsafe {
        // VTABLE1 uses the same PAC, but the Cortex processor handles the underlying addresses
        // differently, because they are being accessed from within core1, instead of core0.
        let mut pac = Peripherals::steal();
        VTABLE1.init(&mut pac.PPB);
        VTABLE1.activate(&mut pac.PPB);
        
        
        comms::init_usb(usb_bus);
        
        loop {
            comms::check_usb();
        }
    }
}