use defmt::info;
use pio::{InstructionOperands, Program, RP2040_MAX_PROGRAM_SIZE};
use rp_pico::pac::{PIO0, PIO1};
use num_enum::IntoPrimitive;
use rp_pico::pac::pio0::SM;

#[derive(Debug, PartialEq, Eq, Copy, Clone,)]
pub enum PioSel {
    Zero,
    One,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, IntoPrimitive)]
#[repr(u8)]
pub enum SmSel {
    Zero = 0b0001,
    One = 0b0010,
    Two = 0b0100,
    Three = 0b1000,
}

/// Refer to pages 374-375 of the RP2040 Datasheet for more details about these options.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PioOption {
    ClockDiv(f32),
    /// If true, the MSB of the Delay/Side-set instruction field is used as side-set enable, rather than a side-set data bit. 
    SideEn(bool),
    /// If true, side-set data is asserted to pin directions, instead of pin values.
    SidePindir(bool),
    JmpPin(u8),
    OutEnSel(u8),
    InlineOutEn(bool),
    OutSticky(bool),
    /// Local program address of `.wrap`. Note: Developer must account for program origin (WrapTop = .wrap + origin)
    WrapTop(u8),
    /// Local program address of `.wrap_target`. Note: Developer must account for program origin (WrapBottom = .wrap_target + origin)
    WrapBottom(u8),
    MovStatus(MovStatusConfig),
    FJoin(FJoinConfig),
    PullThresh(u8),
    PushThresh(u8),
    OutShiftdir(ShiftDirection),
    InShiftdir(ShiftDirection),
    Autopull(bool),
    Autopush(bool),
    SidesetCount(u8),
    SetCount(u8),
    OutCount(u8),
    InBase(u8),
    SidesetBase(u8),
    SetBase(u8),
    OutBase(u8),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ShiftDirection {
    Left,
    Right,
}

impl ShiftDirection {
    fn bit(self) -> bool {
        match self {
            Self::Left => false,
            Self::Right => true,
        }
    }
}

/// Comparison used for `MOV x, STATUS` instruction.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MovStatusConfig {
    /// The `MOV x, STATUS` instruction returns all ones if TX FIFO level is less than the set status, otherwise all zeros.
    Tx(u8),
    /// The `MOV x, STATUS` instruction returns all ones if RX FIFO level is less than the set status, otherwise all zeros.
    Rx(u8),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FJoinConfig {
    RxTx,
    OnlyTx,
    OnlyRx,
}

/// Writes a PIO program into memory.
/// 
/// Use `pio_sel` to specify which PIO core to install the program into: false = PIO0, true = PIO1
pub fn install_program(program: &Program<{ RP2040_MAX_PROGRAM_SIZE }>, pio_sel: PioSel) {
    unsafe {
        if let Some(origin) = program.origin {
            if origin > 0 {
                panic!("PIO max sized program must have origin=0.");
            }
        }
        let pio = match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        };
        
        for i in 0..program.code.len() {
            pio.instr_mem[i].write(|w| w.instr_mem0().bits(program.code[i]));
        }
    }
}

pub fn configure(pio_sel: PioSel, sm_sel: SmSel, options: &[PioOption]) {
    unsafe {
        let pio = match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        };
        let sm = match sm_sel {
            SmSel::Zero => &pio.sm[0],
            SmSel::One => &pio.sm[1],
            SmSel::Two => &pio.sm[2],
            SmSel::Three => &pio.sm[3],
        };
        if options.is_empty() { return }
        
        for option in options {
            match option {
                PioOption::ClockDiv(clock_div) => {
                    let int = *clock_div as u16;
                    let frac = ((*clock_div - int as f32) * 256.0) as u8;
                    
                    sm.sm_clkdiv.modify(|_, w| w.int().bits(int).frac().bits(frac));
                },
                PioOption::SideEn(side_en) => sm.sm_execctrl.modify(|_, w| w.side_en().bit(*side_en)),
                PioOption::SidePindir(side_pindir) => sm.sm_execctrl.modify(|_, w| w.side_pindir().bit(*side_pindir)),
                PioOption::JmpPin(jmp_pin) => sm.sm_execctrl.modify(|_, w| w.jmp_pin().bits(*jmp_pin)),
                PioOption::OutEnSel(out_en_sel) => sm.sm_execctrl.modify(|_, w| w.out_en_sel().bits(*out_en_sel)),
                PioOption::InlineOutEn(inline_out_en) => sm.sm_execctrl.modify(|_, w| w.inline_out_en().bit(*inline_out_en)),
                PioOption::OutSticky(out_sticky) => sm.sm_execctrl.modify(|_, w| w.out_sticky().bit(*out_sticky)),
                PioOption::WrapTop(wrap_top) => sm.sm_execctrl.modify(|_, w| w.wrap_top().bits(*wrap_top)),
                PioOption::WrapBottom(wrap_bottom) => sm.sm_execctrl.modify(|_, w| w.wrap_bottom().bits(*wrap_bottom)),
                PioOption::MovStatus(config) => {
                    let n = match config {
                        MovStatusConfig::Tx(n) => {
                            sm.sm_execctrl.modify(|_, w| w.status_sel().bit(false));
                            n
                        },
                        MovStatusConfig::Rx(n) => {
                            sm.sm_execctrl.modify(|_, w| w.status_sel().bit(true));
                            n
                        }
                    };
                    sm.sm_execctrl.modify(|_, w| w.status_n().bits(*n));
                },
                PioOption::FJoin(config) => {
                    use FJoinConfig::*;
                    let (fjoin_rx, fjoin_tx) = match config {
                        RxTx => (false, false),
                        OnlyTx => (false, true),
                        OnlyRx => (true, false),
                    };
                    sm.sm_shiftctrl.modify(|_, w| w.fjoin_rx().bit(fjoin_rx).fjoin_tx().bit(fjoin_tx));
                },
                PioOption::PullThresh(pull_thresh) => sm.sm_shiftctrl.modify(|_, w| w.pull_thresh().bits(*pull_thresh)),
                PioOption::PushThresh(push_thresh) => sm.sm_shiftctrl.modify(|_, w| w.push_thresh().bits(*push_thresh)),
                PioOption::OutShiftdir(out_shiftdir) => sm.sm_shiftctrl.modify(|_, w| w.out_shiftdir().bit(out_shiftdir.bit())),
                PioOption::InShiftdir(in_shiftdir) => sm.sm_shiftctrl.modify(|_, w| w.in_shiftdir().bit(in_shiftdir.bit())),
                PioOption::Autopull(autopull) => sm.sm_shiftctrl.modify(|_, w| w.autopull().bit(*autopull)),
                PioOption::Autopush(autopush) => sm.sm_shiftctrl.modify(|_, w| w.autopush().bit(*autopush)),
                PioOption::SidesetCount(sideset_count) => sm.sm_pinctrl.modify(|_, w| w.sideset_count().bits(*sideset_count)),
                PioOption::SetCount(set_count) => sm.sm_pinctrl.modify(|_, w| w.set_count().bits(*set_count)),
                PioOption::OutCount(out_count) => sm.sm_pinctrl.modify(|_, w| w.out_count().bits(*out_count)),
                PioOption::InBase(in_base) => sm.sm_pinctrl.modify(|_, w| w.in_base().bits(*in_base)),
                PioOption::SidesetBase(sideset_base) => sm.sm_pinctrl.modify(|_, w| w.sideset_base().bits(*sideset_base)),
                PioOption::SetBase(set_base) => sm.sm_pinctrl.modify(|_, w| w.set_base().bits(*set_base)),
                PioOption::OutBase(out_base) => sm.sm_pinctrl.modify(|_, w| w.out_base().bits(*out_base)),
            }
        }
    }
}

pub fn start(pio_sel: PioSel, sm_sel: SmSel) {
    unsafe {
        match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        }.ctrl.write(|w| w.sm_enable().bits(u8::from(sm_sel)));
    }
}

pub fn stop(pio_sel: PioSel, sm_sel: SmSel) {
    unsafe {
        let pio = match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        };
        let sm_mask = pio.ctrl.read().sm_enable().bits() & (!u8::from(sm_sel));
        
        pio.ctrl.modify(|_, w| w.sm_enable().bits(sm_mask));
    }
}

pub fn start_multiple(pio_sel: PioSel, sm_sels: &[SmSel]) {
    unsafe {
        let mut sm_mask = 0;
        for sel in sm_sels {
            sm_mask |= u8::from(*sel);
        }
        
        match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        }.ctrl.modify(|_, w| w.clkdiv_restart().bits(sm_mask).sm_enable().bits(sm_mask));
    }
}

pub fn stop_multiple(pio_sel: PioSel, sm_sels: &[SmSel]) {
    unsafe {
        let mut sm_mask = 0;
        for sel in sm_sels {
            sm_mask |= u8::from(*sel);
        }
        
        let pio = match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        };
        sm_mask = pio.ctrl.read().sm_enable().bits() & (!sm_mask);
        
        pio.ctrl.modify(|_, w| w.sm_enable().bits(sm_mask));
    }
}

#[inline(always)]
pub fn is_rx_empty(pio_sel: PioSel, sm_sel: SmSel) -> bool {
    unsafe {
        match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        }.fstat.read().rxempty().bits() & u8::from(sm_sel) != 0
    }
}

#[inline(always)]
pub fn is_rx_full(pio_sel: PioSel, sm_sel: SmSel) -> bool {
    unsafe {
        match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        }.fstat.read().rxfull().bits() & u8::from(sm_sel) != 0
    }
}

/// Attempts to read a word from TX FIFO.
#[inline]
pub fn fifo_read(pio_sel: PioSel, sm_sel: SmSel) -> Option<u32> {
    unsafe {
        let pio = match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        };
        let sm_mask: u8 = sm_sel.into();
        
        if pio.fstat.read().rxempty().bits() & sm_mask != 0 {
            None
        } else {
            Some(match sm_sel {
                SmSel::Zero => &pio.rxf[0],
                SmSel::One => &pio.rxf[1],
                SmSel::Two => &pio.rxf[2],
                SmSel::Three => &pio.rxf[3],
            }.read().bits())
        }
    }
}

#[inline(always)]
pub fn is_tx_empty(pio_sel: PioSel, sm_sel: SmSel) -> bool {
    unsafe {
        match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        }.fstat.read().txempty().bits() & u8::from(sm_sel) != 0
    }
}

#[inline(always)]
pub fn is_tx_full(pio_sel: PioSel, sm_sel: SmSel) -> bool {
    unsafe {
        match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        }.fstat.read().txfull().bits() & u8::from(sm_sel) != 0
    }
}

/// Attempts to write a word to RX FIFO.
/// 
/// Returns true is value was written.
#[inline]
pub fn fifo_write(pio_sel: PioSel, sm_sel: SmSel, data: u32) -> bool {
    unsafe {
        let pio = match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        };
        let sm_mask: u8 = sm_sel.into();
        
        if pio.fstat.read().txfull().bits() & sm_mask != 0 {
            false
        } else {
            match sm_sel {
                SmSel::Zero => &pio.txf[0],
                SmSel::One => &pio.txf[1],
                SmSel::Two => &pio.txf[2],
                SmSel::Three => &pio.txf[3],
            }.write(|w| w.bits(data));
            
            true
        }
    }
}

#[inline]
pub fn exec(pio_sel: PioSel, sm_sel: SmSel, instr: InstructionOperands) {
    unsafe {
        let pio = match pio_sel {
            PioSel::Zero => &(*PIO0::ptr()),
            PioSel::One => &(*PIO1::ptr()),
        };
        match sm_sel {
            SmSel::Zero => &pio.sm[0],
            SmSel::One => &pio.sm[1],
            SmSel::Two => &pio.sm[2],
            SmSel::Three => &pio.sm[3],
        }.sm_instr.write(|w| w.bits(instr.encode() as u32))
    }
}