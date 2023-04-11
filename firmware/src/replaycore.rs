use alloc::vec::Vec;
use bincode::{Decode, Encode};
use cortex_m::asm::nop;
use cortex_m::delay::Delay;
use defmt::Format;
use num_enum::{FromPrimitive, IntoPrimitive};
use crate::{info, systems};

#[derive(Debug, PartialEq, Eq, Copy, Clone, FromPrimitive, Encode, Decode)]
#[repr(u8)]
pub enum VeritasMode {
    Initial = 0x00,
    #[num_enum(default)]
    Idle = 0x01,
    ReplayN64 = 0x02,
    ReplayNes = 0x03,
    ReplayA2600 = 0x04,
    ReplayGenesis = 0x05,
}
use VeritasMode::*;
use crate::systems::nes::REPLAY_STATE;

pub static mut VERITAS_MODE: VeritasMode = Initial;

#[derive(Debug, Format, PartialEq, Eq, Copy, Clone, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Transition {
    SoftReset = 0x01,
    PowerReset = 0x02,
    //RestartTasdFile = 0x03,
    //PacketDerived = 0xFF,
    
    #[num_enum(default)]
    Unsupported = 0x00,
}

#[derive(Debug)]
pub struct ReplayState {
    pub index_len: u32,
    pub index_cur: u32,
    pub transitions: Vec<(u32, Transition)>,
    pub traptr: usize,
}
impl ReplayState {
    pub const fn new() -> Self { Self {
        index_len: 0xFFFFFFFF,
        index_cur: 0,
        transitions: Vec::new(),
        traptr: 0,
    }}
    
    pub fn reset(&mut self) {
        self.index_len = 0xFFFFFFFF;
        self.index_cur = 0;
        self.transitions.clear();
        self.traptr = 0;
    }
    
    #[inline(always)]
    pub fn next_transition(&mut self) -> Option<Transition> {
        if let Some((i, tra)) = self.transitions.get(self.traptr) {
            if *i == self.index_cur {
                self.traptr += 1;
                return Some(*tra);
            }
        }
        
        None
    }
}

pub fn run(mut delay: Delay) -> ! {
    unsafe {
        //gpio::set_low(PIN_CNT_18_DIR);
        //gpio::set_as_output(PIN_CNT_18, true, false); // Console reset (active-high)
        REPLAY_STATE.reset();
        VERITAS_MODE = Idle;
        info!("VeriTAS Ready!");
        
        /*gpio::set_high(PIN_CNT_18_DIR);
        loop {
            gpio::set_low(PIN_CNT_18);
            delay.delay_us(100);
            gpio::set_high(PIN_CNT_18);
            delay.delay_us(100);
        }*/
        
        loop {
            match VERITAS_MODE {
                Initial => nop(),
                Idle => nop(),
                ReplayN64 => systems::n64::run(&mut delay),
                ReplayNes => systems::nes::run(&mut delay),
                ReplayA2600 => nop(),
                ReplayGenesis => nop(),
            }
            
            nop();
        }
    }
}