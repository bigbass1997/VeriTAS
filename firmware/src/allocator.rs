use core::alloc::Layout;
use core::panic::PanicInfo;
use alloc_cortex_m::CortexMHeap;

#[global_allocator]
pub static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    loop {}
}

/*#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}*/