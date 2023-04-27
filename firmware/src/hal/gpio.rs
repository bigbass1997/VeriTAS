
#![allow(unused)]

use rp2040_pac::{IO_BANK0, PADS_BANK0, SIO};
use rp2040_pac::io_bank0::gpio::gpio_ctrl::{FUNCSEL_A, FUNCSEL_R};
use rp2040_pac::pads_bank0::gpio::{DRIVE_A, DRIVE_R};

pub const PIN_DISPLAY_STROBE0: usize = 0;
pub const PIN_DISPLAY_STROBE1: usize = 1;
pub const PIN_DISPLAY_STROBE2: usize = 2;
pub const PIN_DISPLAY_STROBE3: usize = 3;
pub const PIN_DISPLAY_SER: usize = 4;
pub const PIN_DISPLAY_CLK: usize = 5;

pub const PIN_CON_RESET: usize = 6;
pub const PIN_ALT_CLK: usize = 7;

pub const PIN_CNT_0: usize = 8;
pub const PIN_CNT_1: usize = 9;
pub const PIN_CNT_2: usize = 10;
pub const PIN_CNT_3: usize = 11;
pub const PIN_CNT_4: usize = 12;
pub const PIN_CNT_5: usize = 13;
pub const PIN_CNT_6: usize = 14;
pub const PIN_CNT_7: usize = 15;
pub const PIN_CNT_8: usize = 16;
pub const PIN_CNT_9: usize = 17;
pub const PIN_CNT_10: usize = 18;
pub const PIN_CNT_11: usize = 19;
pub const PIN_CNT_12: usize = 20;
pub const PIN_CNT_13: usize = 21;
pub const PIN_CNT_14: usize = 22;
pub const PIN_CNT_15: usize = 23;
pub const PIN_CNT_16: usize = 24;

pub const PIN_CNT_17: usize = 25;
pub const PIN_CNT_18: usize = 26;
pub const PIN_CNT_17_DIR: usize = 27;
pub const PIN_CNT_18_DIR: usize = 28;

pub const PIN_DETECT: usize = 29;

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

#[inline(always)]
pub fn set_state(gpio: usize, state: bool) {
    if state {
        set_high(gpio);
    } else {
        set_low(gpio);
    }
}