use alloc::vec::Vec;
use cortex_m::asm::nop;
use cortex_m::delay::Delay;
use crate::{info, systems};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum VeritasMode {
    Initial,
    Idle,
    ReplayN64,
    ReplayNes,
    ReplayA2600,
    ReplayGenesis,
}
use VeritasMode::*;

pub static mut VERITAS_MODE: VeritasMode = Initial;

pub enum Transition {
    SoftReset,
    HardReset,
}

pub struct ReplayConfig {
    pub index_len: u32,
    pub transitions: Vec<(u32, Transition)>,
}

pub fn run(mut delay: Delay) -> ! {
    unsafe {
        VERITAS_MODE = Idle;
        info!("VeriTAS Ready!");
        VERITAS_MODE = ReplayNes;
        
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