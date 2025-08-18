use cortex_m::asm::nop;
use rp2040_pac::{UART0, UART1};
use rp2040_pac::uart0::RegisterBlock;

const UART: [*const RegisterBlock; 2] = unsafe { [UART0::ptr(), UART1::ptr()] };

pub fn read_one(uart: usize) -> Option<u8> {
    if is_empty(uart) {
        None
    } else {
        Some(unsafe { (*UART[uart]).uartdr.read().data().bits() })
    }
}

#[inline(always)]
pub fn read_one_blocking(uart: usize) -> u8 {
    while is_empty(uart) { nop() }
    
    unsafe { (*UART[uart]).uartdr.read().data().bits() }
}

#[inline]
pub fn read_blocking(uart_index: usize, buf: &mut [u8]) {
    let mut bytes_read = 0;
    
    loop {
        if is_empty(uart_index) { continue }
        
        buf[bytes_read] = unsafe { (*UART[uart_index]).uartdr.read().data().bits() };
        bytes_read += 1;
        
        if bytes_read == buf.len() { break }
    }
}

#[inline(always)]
pub fn is_empty(uart: usize) -> bool {
    unsafe { (*UART[uart]).uartfr.read().rxfe().bit() }
}


pub fn write_one_blocking(uart: usize, data: u8) {
    while is_full(uart) { nop() }
    
    unsafe { (*UART[uart]).uartdr.write(|w| w.data().bits(data)); }
}

#[inline]
pub fn write_blocking(uart_index: usize, buf: &[u8]) {
    let mut bytes_written = 0;
    
    loop {
        if is_full(uart_index) { continue }
        
        unsafe { (*UART[uart_index]).uartdr.write(|w| w.data().bits(buf[bytes_written])); }
        bytes_written += 1;
        
        if bytes_written == buf.len() { break }
    }
}

#[inline(always)]
pub fn is_full(uart: usize) -> bool {
    unsafe { (*UART[uart]).uartfr.read().txff().bit() }
}