
#![allow(unused)]

use rp2040_pac::{IO_BANK0, PADS_BANK0, SIO};
use rp2040_pac::io_bank0::gpio::gpio_ctrl::{FUNCSEL_A, FUNCSEL_R};
use rp2040_pac::pads_bank0::gpio::{DRIVE_A, DRIVE_R};

pub const PIN_DISPLAY_STROBE0: Gpio = Gpio(0);
pub const PIN_DISPLAY_STROBE1: Gpio = Gpio(1);
pub const PIN_DISPLAY_STROBE2: Gpio = Gpio(2);
pub const PIN_DISPLAY_STROBE3: Gpio = Gpio(3);
pub const PIN_DISPLAY_SER: Gpio = Gpio(4);
pub const PIN_DISPLAY_CLK: Gpio = Gpio(5);

pub const PIN_CON_RESET: Gpio = Gpio(6);
pub const PIN_ALT_CLK: Gpio = Gpio(7);

pub const PIN_CNT_0: Gpio = Gpio(8);
pub const PIN_CNT_1: Gpio = Gpio(9);
pub const PIN_CNT_2: Gpio = Gpio(10);
pub const PIN_CNT_3: Gpio = Gpio(11);
pub const PIN_CNT_4: Gpio = Gpio(12);
pub const PIN_CNT_5: Gpio = Gpio(13);
pub const PIN_CNT_6: Gpio = Gpio(14);
pub const PIN_CNT_7: Gpio = Gpio(15);
pub const PIN_CNT_8: Gpio = Gpio(16);
pub const PIN_CNT_9: Gpio = Gpio(17);
pub const PIN_CNT_10: Gpio = Gpio(18);
pub const PIN_CNT_11: Gpio = Gpio(19);
pub const PIN_CNT_12: Gpio = Gpio(20);
pub const PIN_CNT_13: Gpio = Gpio(21);
pub const PIN_CNT_14: Gpio = Gpio(22);
pub const PIN_CNT_15: Gpio = Gpio(23);
pub const PIN_CNT_16: Gpio = Gpio(24);

pub const PIN_CNT_17: Gpio = Gpio(25);
pub const PIN_CNT_18: Gpio = Gpio(26);
pub const PIN_CNT_17_DIR: Gpio = Gpio(27);
pub const PIN_CNT_18_DIR: Gpio = Gpio(28);

pub const PIN_DETECT: Gpio = Gpio(29);

pub const CP_1: Gpio = PIN_CNT_17;
pub const CP_1_DIR: Gpio = PIN_CNT_17_DIR;

//pub const CP_2: Gpio = PIN_CNT_; // Unconnected
pub const CP_3: Gpio = PIN_CNT_16;
pub const CP_4: Gpio = PIN_CNT_13;
pub const CP_5: Gpio = PIN_CNT_11;
pub const CP_6: Gpio = PIN_CNT_9;
pub const CP_7: Gpio = PIN_CNT_7;
pub const CP_8: Gpio = PIN_CNT_5;

pub const CP_11: Gpio = PIN_CNT_18;
pub const CP_11_DIR: Gpio = PIN_CNT_18_DIR;

//pub const CP_12: Gpio = PIN_CNT_; // Unconnected
pub const CP_13: Gpio = PIN_CNT_14;
pub const CP_14: Gpio = PIN_CNT_12;
pub const CP_15: Gpio = PIN_CNT_10;
pub const CP_16: Gpio = PIN_CNT_6;
pub const CP_17: Gpio = PIN_CNT_4;
pub const CP_18: Gpio = PIN_CNT_3;

pub const CP_21: Gpio = PIN_CNT_15;
pub const CP_22: Gpio = PIN_CNT_8;
pub const CP_23: Gpio = PIN_CNT_0;
pub const CP_24: Gpio = PIN_CNT_1;
pub const CP_25: Gpio = PIN_CNT_2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gpio(pub usize);
impl Gpio {
    pub const fn new(gpio: usize) -> Self {
        Self(gpio)
    }
    
    #[inline(always)]
    pub fn function(self) -> FUNCSEL_R {
        unsafe { (*IO_BANK0::ptr()).gpio(self.0).gpio_ctrl().read().funcsel() }
    }
    
    #[inline(always)]
    pub fn set_function(self, func: FUNCSEL_A) -> Self {
        unsafe {
            (*IO_BANK0::ptr()).gpio(self.0).gpio_ctrl().write_with_zero(|w| w.funcsel().variant(func));
            self.set_output_disable(false);
        }
        
        self
    }
    
    #[inline(always)]
    pub fn output_disable(self) -> bool {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).read().od().bit() }
    }
    
    #[inline(always)]
    pub fn set_output_disable(self, od: bool) -> Self {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).modify(|_, w| w.od().bit(od)); }
        
        self
    }
    
    #[inline(always)]
    pub fn input_enable(self) -> bool {
        unsafe {
            (*PADS_BANK0::ptr()).gpio(self.0).read().ie().bit()
        }
    }
    
    #[inline(always)]
    pub fn set_input_enable(self, ie: bool) -> Self {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).modify(|_, w| w.ie().bit(ie)); }
        
        self
    }
    
    /// Returns the state of this GPIO's SIO output enable.
    /// 
    /// Note: This setting should have no effect if the GPIO's function is _not_ set to SIO. 
    #[inline(always)]
    pub fn output_enable(self) -> bool {
        unsafe {
            (*SIO::ptr()).gpio_oe().read().bits() & (1 << self.0) != 0
        }
    }
    
    /// Configures this GPIO's SIO output enable.
    /// 
    /// Note: This setting should have no effect if the GPIO's function is _not_ set to SIO. 
    #[inline(always)]
    pub fn set_sio_output_enable(self, oe: bool) -> Self {
        unsafe {
            if oe {
                (*SIO::ptr()).gpio_oe_set().write(|w| w.bits(1 << self.0));
            } else {
                (*SIO::ptr()).gpio_oe_clr().write(|w| w.bits(1 << self.0));
            }
        }
        
        self
    }
    
    #[inline(always)]
    pub fn drive(self) -> DRIVE_R {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).read().drive() }
    }
    
    #[inline(always)]
    pub fn set_drive(self, drive: DRIVE_A) -> Self {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).modify(|_, w| w.drive().variant(drive)); }
        
        self
    }
    
    #[inline(always)]
    pub fn pull_up_enable(self) -> bool {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).read().pue().bit() }
    }
    
    #[inline(always)]
    pub fn set_pull_up_enable(self, pue: bool) -> Self {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).modify(|_, w| w.pue().bit(pue)); }
        
        self
    }
    
    #[inline(always)]
    pub fn pull_down_enable(self) -> bool {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).read().pde().bit() }
    }
    
    #[inline(always)]
    pub fn set_pull_down_enable(self, pde: bool) -> Self {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).modify(|_, w| w.pde().bit(pde)); }
        
        self
    }
    
    #[inline(always)]
    pub fn schmitt_enable(self) -> bool {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).read().schmitt().bit() }
    }
    
    #[inline(always)]
    pub fn set_schmitt_enable(self, schmitt: bool) -> Self {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).modify(|_, w| w.schmitt().bit(schmitt)); }
        
        self
    }
    
    #[inline(always)]
    pub fn slewrate(self) -> bool {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).read().slewfast().bit() }
    }
    
    #[inline(always)]
    pub fn set_slewrate(self, is_fast: bool) -> Self {
        unsafe { (*PADS_BANK0::ptr()).gpio(self.0).modify(|_, w| w.slewfast().bit(is_fast)); }
        
        self
    }
    
    #[inline(always)]
    pub fn is_high(self) -> bool {
        unsafe {
            (*SIO::ptr()).gpio_in().read().bits() & (1 << self.0) != 0
        }
    }
    
    #[inline(always)]
    pub fn is_low(self) -> bool {
        unsafe {
            (*SIO::ptr()).gpio_in().read().bits() & (1 << self.0) == 0
        }
    }
    
    #[inline(always)]
    pub fn set_high(self) -> Self {
        unsafe {
            (*SIO::ptr()).gpio_out_set().write(|w| w.bits(1 << self.0));
        }
        
        self
    }
    
    #[inline(always)]
    pub fn set_low(self) -> Self {
        unsafe {
            (*SIO::ptr()).gpio_out_clr().write(|w| w.bits(1 << self.0));
        }
        
        self
    }
    
    #[inline(always)]
    pub fn into_input(self, pull_up: bool, pull_down: bool) -> Self {
        self.set_function(FUNCSEL_A::SIO);
        self.set_pull_down_enable(pull_down);
        self.set_pull_up_enable(pull_up);
        self.set_output_disable(true);
        self.set_input_enable(true);
        self.set_sio_output_enable(false);
        
        self
    }
    
    #[inline(always)]
    pub fn into_output(self, pull_up: bool, pull_down: bool) -> Self {
        self.set_function(FUNCSEL_A::SIO);
        self.set_pull_down_enable(pull_down);
        self.set_pull_up_enable(pull_up);
        self.set_output_disable(false);
        self.set_input_enable(false);
        self.set_sio_output_enable(true);
        
        self
    }
}
