#![allow(unused)]

use rp2040_pac::{Interrupt, IO_BANK0, PPB, SIO, TIMER};

#[inline(always)]
pub fn enable_nvic(intr: Interrupt) {
    unsafe {
        (*PPB::ptr()).nvic_iser.write(|w| w.bits(1 << (intr as u32)));
    }
}

#[inline(always)]
pub fn disable_nvic(intr: Interrupt) {
    unsafe {
        (*PPB::ptr()).nvic_icer.write(|w| w.bits(1 << (intr as u32)));
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
/// Interrupt kind
pub enum Edge {
    /// While low
    LevelLow = 0,
    /// While high
    LevelHigh = 1,
    /// On falling edge
    EdgeLow = 2,
    /// On rising edge
    EdgeHigh = 3,
}

#[inline(always)]
fn gpio_mask(gpio: usize, edge: Edge) -> u32 {
    1 << (gpio % 8 * 4 + edge as usize)
}

#[inline(always)]
pub fn enable_gpio_intr(gpio: usize, edge: Edge) {
    unsafe {
        let cpuid = *(SIO::ptr() as *const u32);
        let io = &(*IO_BANK0::ptr());
        
        if cpuid == 0 {
            io.proc0_inte[gpio >> 3].modify(|r, w| w.bits(r.bits() | gpio_mask(gpio, edge)));
        } else {
            io.proc1_inte[gpio >> 3].modify(|r, w| w.bits(r.bits() | gpio_mask(gpio, edge)));
        }
    }
}

#[inline(always)]
pub fn disable_gpio_intr(gpio: usize, edge: Edge) {
    unsafe {
        let cpuid = *(SIO::ptr() as *const u32);
        let io = &(*IO_BANK0::ptr());
        
        let mask = 1 << (gpio % 8 * 4 + edge as usize);
        
        if cpuid == 0 {
            io.proc0_inte[gpio >> 3].modify(|r, w| w.bits(r.bits() & !gpio_mask(gpio, edge)));
        } else {
            io.proc1_inte[gpio >> 3].modify(|r, w| w.bits(r.bits() & !gpio_mask(gpio, edge)));
        }
    }
}

#[inline(always)]
pub fn clear_gpio_intr(gpio: usize, edge: Edge) {
    unsafe {
        (*IO_BANK0::ptr()).intr[gpio >> 3].write(|w| w.bits(gpio_mask(gpio, edge)));
    }
}

#[inline(always)]
pub fn status_gpio_intr(gpio: usize, edge: Edge) -> bool {
    unsafe {
        let cpuid = *(SIO::ptr() as *const u32);
        let io = &(*IO_BANK0::ptr());
        
        if cpuid == 0 {
            io.proc0_ints[gpio >> 3].read().bits() & gpio_mask(gpio, edge) > 0
        } else {
            io.proc1_ints[gpio >> 3].read().bits() & gpio_mask(gpio, edge) > 0
        }
    }
}



#[inline(always)]
pub fn enable_alarm_intr(alarm: usize) {
    unsafe {
        (*TIMER::ptr()).inte.modify(|r, w| w.bits(r.bits() | (1 << alarm)));
    }
}

#[inline(always)]
pub fn disable_alarm_intr(alarm: usize) {
    unsafe {
        (*TIMER::ptr()).inte.modify(|r, w| w.bits(r.bits() & !(1 << alarm)));
    }
}

#[inline(always)]
pub fn clear_alarm_intr(alarm: usize) {
    unsafe {
        (*TIMER::ptr()).intr.write(|w| w.bits(1 << alarm));
    }
}

#[inline(always)]
pub fn status_alarm_intr(alarm: usize) -> bool {
    unsafe {
        (*TIMER::ptr()).ints.read().bits() & (1 << alarm) > 0
    }
}

#[inline(always)]
pub fn arm_alarm(alarm: usize, duration_us: u32) {
    unsafe {
        let reg = &(*TIMER::ptr());
        let new_time = || reg.timerawl.read().bits().wrapping_add(duration_us);
        
        match alarm {
            0 => reg.alarm0.write(|w| w.bits(new_time())),
            1 => reg.alarm1.write(|w| w.bits(new_time())),
            2 => reg.alarm2.write(|w| w.bits(new_time())),
            3 => reg.alarm3.write(|w| w.bits(new_time())),
            _ => ()
        }
    }
}