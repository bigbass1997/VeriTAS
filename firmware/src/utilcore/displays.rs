use cortex_m::asm::{delay, nop};
use heapless::spsc::Queue;
use heapless::Vec;
use num_enum::{FromPrimitive, IntoPrimitive};
use crate::hal::gpio;
use crate::hal::gpio::{PIN_DISPLAY_CLK, PIN_DISPLAY_SER, PIN_DISPLAY_STROBE0, PIN_DISPLAY_STROBE1, PIN_DISPLAY_STROBE2, PIN_DISPLAY_STROBE3};

const STROBE_PINS: [usize; 4] = [
    PIN_DISPLAY_STROBE0,
    PIN_DISPLAY_STROBE1,
    PIN_DISPLAY_STROBE2,
    PIN_DISPLAY_STROBE3,
];

const BLANK: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
const STARTUP: [u8; 6] = [0xAA, 0x55, 0x11, 0x33, 0xDD, 0xFF];

#[derive(Debug, PartialEq, Copy, Clone, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Port {
    Display0 = 0,
    Display1 = 1,
    Display2 = 2,
    Display3 = 3,
    
    #[num_enum(default)]
    Err = 0xFF,
}

pub static mut NES_STATE: [u8; 2] = [0xFF, 0xFF];

static mut PORT_QUEUES: [Queue<Vec<u8, 8>, 4>; 4] = [Queue::new(), Queue::new(), Queue::new(), Queue::new()];


pub fn set_display<T: Into<Vec<u8, 8>>>(port: Port, data: T) {
    if port == Port::Err {
        return;
    }
    
    unsafe {
        PORT_QUEUES[port as usize].enqueue(data.into()).unwrap_or_default();
    }
}

pub fn check_displays() {
    for i in 0..4 {
        if let Some(data) = unsafe { PORT_QUEUES[i].dequeue() } {
            write((i as u8).into(), &data);
        }
    }
    
    /*unsafe {
        write(Port::Display0, &[NES_STATE[0] ^ 0xFF]);
        write(Port::Display1, &[NES_STATE[1] ^ 0xFF]);
    }*/
}

pub fn initialize() {
    gpio::set_low(PIN_DISPLAY_CLK);
    gpio::set_low(PIN_DISPLAY_SER);
    
    gpio::set_as_output(PIN_DISPLAY_CLK, true, false);
    gpio::set_as_output(PIN_DISPLAY_SER, true, false);
    for i in 0..4 {
        gpio::set_low(STROBE_PINS[i]);
        gpio::set_as_output(STROBE_PINS[i], true, false);
    }
    
    for byte in STARTUP {
        for port in 0..4 {
            write(port.into(), &[byte]);
        }
        delay(20000000);
    }
    
    
    for port in 0..4 {
        write(port.into(), &BLANK);
    }
}

fn write(port: Port, data: &[u8]) {
    for byte in data {
        for i in 0..8 {
            let state = (byte & (0x80 >> i)) != 0;
            set_ser(state);
            pulse_pin(PIN_DISPLAY_CLK);
        }
        latch(port);
    }
}

#[inline]
fn set_ser(state: bool) {
    if state {
        gpio::set_high(PIN_DISPLAY_SER);
    } else {
        gpio::set_low(PIN_DISPLAY_SER);
    }
    nop();
}

#[inline]
fn pulse_pin(pin: usize) {
    gpio::set_high(pin);
    nop();
    gpio::set_low(pin);
    nop();
}

#[inline]
fn latch(port: Port) {
    pulse_pin(STROBE_PINS[port as usize]);
}