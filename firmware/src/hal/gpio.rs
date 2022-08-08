
#![allow(unused)]

use rp_pico::pac::{IO_BANK0, PADS_BANK0};
use rp_pico::pac::io_bank0::gpio::gpio_ctrl::{FUNCSEL_A, FUNCSEL_R};
use rp_pico::pac::pads_bank0::gpio::{DRIVE_A, DRIVE_R};


#[inline(always)]
pub fn function(gpio: usize) -> FUNCSEL_R {
    unsafe {
        (*IO_BANK0::ptr()).gpio[gpio].gpio_ctrl.read().funcsel()
    }
}

#[inline(always)]
pub fn set_function(gpio: usize, func: FUNCSEL_A) {
    unsafe {
        (*IO_BANK0::ptr()).gpio[gpio].gpio_ctrl.write(|w| w.funcsel().variant(func));
    }
}

#[inline(always)]
pub fn output_disable(gpio: usize) -> bool {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].read().od().bit()
    }
}

#[inline(always)]
pub fn set_output_disable(gpio: usize, flag: bool) {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].write(|w| w.od().bit(flag));
    }
}

#[inline(always)]
pub fn input_enable(gpio: usize) -> bool {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].read().ie().bit()
    }
}

#[inline(always)]
pub fn set_input_enable(gpio: usize, flag: bool) {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].write(|w| w.ie().bit(flag));
    }
}

#[inline(always)]
pub fn drive(gpio: usize) -> DRIVE_R {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].read().drive()
    }
}

#[inline(always)]
pub fn set_drive(gpio: usize, drive: DRIVE_A) {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].write(|w| w.drive().variant(drive));
    }
}

#[inline(always)]
pub fn pull_up_enable(gpio: usize) -> bool {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].read().pue().bit()
    }
}

#[inline(always)]
pub fn set_pull_up_enable(gpio: usize, flag: bool) {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].write(|w| w.pue().bit(flag));
    }
}

#[inline(always)]
pub fn pull_down_enable(gpio: usize) -> bool {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].read().pde().bit()
    }
}

#[inline(always)]
pub fn set_pull_down_enable(gpio: usize, flag: bool) {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].write(|w| w.pde().bit(flag));
    }
}

#[inline(always)]
pub fn schmitt_enable(gpio: usize) -> bool {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].read().schmitt().bit()
    }
}

#[inline(always)]
pub fn set_schmitt_enable(gpio: usize, flag: bool) {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].write(|w| w.schmitt().bit(flag));
    }
}

#[inline(always)]
pub fn slewrate(gpio: usize) -> bool {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].read().slewfast().bit()
    }
}

#[inline(always)]
pub fn set_slewrate(gpio: usize, flag: bool) {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].write(|w| w.slewfast().bit(flag));
    }
}