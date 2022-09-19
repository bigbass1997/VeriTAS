use alloc::collections::btree_map::BTreeMap;
use cortex_m::asm::nop;
use cortex_m::delay::Delay;
use num_enum::FromPrimitive;
use crate::{info, systems};

#[derive(Debug, PartialEq, Eq, Copy, Clone, FromPrimitive)]
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

pub static mut VERITAS_MODE: VeritasMode = Initial;

#[derive(Debug, PartialEq, Eq, Copy, Clone, FromPrimitive)]
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
    pub transitions: BTreeMap<u32, Transition>,
}
impl ReplayState {
    pub const fn new() -> Self { Self {
        index_len: 0xFFFFFFFF,
        index_cur: 0,
        transitions: BTreeMap::new()
    }}
    
    pub fn reset(&mut self) {
        self.index_len = 0xFFFFFFFF;
        self.index_cur = 0;
        self.transitions.clear();
    }
}

pub fn run(mut delay: Delay) -> ! {
    unsafe {
        VERITAS_MODE = Idle;
        info!("VeriTAS Ready!");
        
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