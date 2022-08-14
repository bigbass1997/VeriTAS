
#![allow(unused)]

use rp2040_pac::{IO_BANK0, PADS_BANK0, SIO};
use rp2040_pac::io_bank0::gpio::gpio_ctrl::{FUNCSEL_A, FUNCSEL_R};
use rp2040_pac::pads_bank0::gpio::{DRIVE_A, DRIVE_R};


#[inline(always)]
pub fn function(gpio: usize) -> FUNCSEL_R {
    unsafe {
        (*IO_BANK0::ptr()).gpio[gpio].gpio_ctrl.read().funcsel()
    }
}

#[inline(always)]
pub fn set_function(gpio: usize, func: FUNCSEL_A) {
    unsafe {
        (*IO_BANK0::ptr()).gpio[gpio].gpio_ctrl.modify(|_, w| w.funcsel().variant(func));
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
        (*PADS_BANK0::ptr()).gpio[gpio].modify(|_, w| w.od().bit(flag));
    }
}

#[inline(always)]
pub fn sio_output_enable(gpio: usize) -> bool {
    unsafe {
        (*SIO::ptr()).gpio_oe.read().bits() & (1 << gpio) != 0
    }
}

#[inline(always)]
pub fn set_sio_output_enable(gpio: usize, flag: bool) {
    unsafe {
        if flag {
            (*SIO::ptr()).gpio_oe_set.write(|w| w.bits(1 << gpio));
        } else {
            (*SIO::ptr()).gpio_oe_clr.write(|w| w.bits(1 << gpio));
        }
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
        (*PADS_BANK0::ptr()).gpio[gpio].modify(|_, w| w.ie().bit(flag));
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
        (*PADS_BANK0::ptr()).gpio[gpio].modify(|_, w| w.drive().variant(drive));
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
        (*PADS_BANK0::ptr()).gpio[gpio].modify(|_, w| w.pue().bit(flag));
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
        (*PADS_BANK0::ptr()).gpio[gpio].modify(|_, w| w.pde().bit(flag));
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
        (*PADS_BANK0::ptr()).gpio[gpio].modify(|_, w| w.schmitt().bit(flag));
    }
}

#[inline(always)]
pub fn slewrate(gpio: usize) -> bool {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].read().slewfast().bit()
    }
}

#[inline(always)]
pub fn set_as_input(gpio: usize, pull_up: bool, pull_down: bool) {
    set_function(gpio, FUNCSEL_A::SIO);
    set_pull_down_enable(gpio, pull_down);
    set_pull_up_enable(gpio, pull_up);
    set_output_disable(gpio, true);
    set_input_enable(gpio, true);
    set_sio_output_enable(gpio, false);
}

#[inline(always)]
pub fn set_as_output(gpio: usize, pull_up: bool, pull_down: bool) {
    set_function(gpio, FUNCSEL_A::SIO);
    set_pull_down_enable(gpio, pull_down);
    set_pull_up_enable(gpio, pull_up);
    set_output_disable(gpio, false);
    set_input_enable(gpio, false);
    set_sio_output_enable(gpio, true);
    set_function(gpio, FUNCSEL_A::SIO);
}

#[inline(always)]
pub fn set_slewrate(gpio: usize, flag: bool) {
    unsafe {
        (*PADS_BANK0::ptr()).gpio[gpio].modify(|_, w| w.slewfast().bit(flag));
    }
}

#[inline(always)]
pub fn is_high(gpio: usize) -> bool {
    unsafe {
        (*SIO::ptr()).gpio_in.read().bits() & (1 << gpio) != 0
    }
}

#[inline(always)]
pub fn is_low(gpio: usize) -> bool {
    unsafe {
        (*SIO::ptr()).gpio_in.read().bits() & (1 << gpio) == 0
    }
}

#[inline(always)]
pub fn set_high(gpio: usize) {
    unsafe {
        (*SIO::ptr()).gpio_out_set.write(|w| w.bits(1 << gpio));
    }
}

#[inline(always)]
pub fn set_low(gpio: usize) {
    unsafe {
        (*SIO::ptr()).gpio_out_clr.write(|w| w.bits(1 << gpio));
    }
}