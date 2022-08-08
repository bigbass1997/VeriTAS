use cortex_m::delay::Delay;
use defmt::info;
use heapless::spsc::Queue;
use pio_proc::pio_asm;
use pio::{InstructionOperands, JmpCondition, SetDestination};
use rp_pico::hal::gpio::bank0::{Gpio8, Gpio9};
use rp_pico::hal::gpio::{FunctionUart, Pin};
use rp_pico::hal::uart::{Enabled, UartPeripheral};
use rp_pico::pac::UART1;
use embedded_hal::serial::Read;
use crate::hal::pio as p;
use crate::hal::pio::{PioSel, ShiftDirection, SmSel};
use crate::hal::pio::PioOption::{Autopull, ClockDiv, InBase, InShiftdir, OutBase, OutCount, PullThresh, SetBase, SetCount, WrapBottom, WrapTop};
use crate::VeritasState;

/// Buffered list of controller inputs. 
pub static mut INPUT_BUFFER: Queue<[u32; 4], 64> = Queue::new();

pub static mut CNT_STATE: u32 = 0;

static mut READ_BYTE_VECTOR: u8 = 0;
static mut WRITE_BYTES_VECTOR: u8 = 0;

/// Prepares the device to replay a TAS.
pub fn initialize() {
    let program = { pio_asm!("
        .origin 0
    	.wrap_target
    idle:
        mov x, x [2]
        jmp idle
        
    public read_byte:
        set pindirs, 0b10
        mov isr, null
        set x, 7
    inagain:
        wait 1 pin 0
        wait 0 pin 0 [31]
        nop [7]
        in pins, 1
        set pins, 0b10
        set pins, 0b00
        
        jmp x--, inagain
        push noblock
        jmp idle
        
        
        
    public write_bytes:
        set pins, 0b01
        set pindirs, 0b11
    outagain:
        out pins, 1
        jmp outagain [14]
        
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        
        .wrap
    ")};
    p::install_program(&program.program, PioSel::Zero);
    unsafe {
        READ_BYTE_VECTOR = program.public_defines.read_byte as u32 as u8;
        WRITE_BYTES_VECTOR = program.public_defines.write_bytes as u32 as u8;
    }
    
    let options = [
        InBase(14),
        OutBase(14),
        OutCount(1),
        SetBase(14),
        SetCount(2),
        InShiftdir(ShiftDirection::Left),
        PullThresh(32),
        Autopull(true),
        ClockDiv(10.0),
        WrapBottom(program.program.wrap.target),
        WrapTop(program.program.wrap.source),
    ];
    p::configure(PioSel::Zero, SmSel::Zero, &options);
    p::exec(PioSel::Zero, SmSel::Zero, InstructionOperands::SET { destination: SetDestination::PINDIRS, data: 0b10 }); // init input
}

pub fn run(delay: &mut Delay, uart: &mut UartPeripheral<Enabled, UART1, (Pin<Gpio8, FunctionUart>, Pin<Gpio9, FunctionUart>)>) {
    unsafe {
        crate::VERITAS_STATE = VeritasState::Replay;
        
        while crate::VERITAS_STATE == VeritasState::Replay {
            let mut ptr = 3;
            loop {
                let cmd = read_blocking();
                match cmd {
                    0xFF | 0x01 => {
                        delay.delay_us(4);
                        write_blocking(&CNT_STATE.to_be_bytes());
                        info!("{:08X}", CNT_STATE);
                        delay.delay_us(16);
                    },
                    0x00 => {
                        delay.delay_us(4);
                        write_blocking(&[0x05, 0x00, 0x02]);
                        delay.delay_us(16);
                    },
                    0x02 | 0x03 => {
                        delay.delay_us(150);
                    }
                    _ => ()
                }
                
                // TEMPORARY // To be replaced with UART handler in separate thread, via static INPUT_BUFFER
                let mut frame_break = 0;
                while let Ok(data) = uart.read() {
                    CNT_STATE &= !(0xFF << (ptr * 8));
                    CNT_STATE |= (data as u32) << (ptr * 8);
                    
                    if ptr == 0 {
                        ptr = 3;
                    } else {
                        ptr -= 1;
                    }
                    
                    frame_break += 1;
                    if frame_break == 4 {
                        break;
                    }
                }
            }
        }
    }
}

#[inline(always)]
unsafe fn read_blocking() -> u32 {
    p::exec(PioSel::Zero, SmSel::Zero, InstructionOperands::JMP { condition: JmpCondition::Always, address: READ_BYTE_VECTOR });
    
    loop {
        match p::fifo_read(PioSel::Zero, SmSel::Zero) {
            Some(data) => return data,
            None => ()
        }
    }
}

#[inline(always)]
unsafe fn write_blocking(data: &[u8]) {
    p::exec(PioSel::Zero, SmSel::Zero, InstructionOperands::JMP { condition: JmpCondition::Always, address: WRITE_BYTES_VECTOR });
    
    for byte in data {
        let encoded = encode_joybus(*byte);
        
        loop {
            if p::fifo_write(PioSel::Zero, SmSel::Zero, encoded) {
                break;
            }
        }
    }
    
    loop {
        if p::fifo_write(PioSel::Zero, SmSel::Zero, 0x3FFFFFFF) { // controller stop bit
            break;
        }
    }
    
    loop {
        if p::is_tx_empty(PioSel::Zero, SmSel::Zero) {
            break;
        }
    }
}

#[inline(always)]
fn encode_joybus(mut data: u8) -> u32 {
    let mut out = 0;
    for _ in 0..8 {
        out <<= 4;
        if (data & 0x80) != 0 {
            out |= 0b0111;
        } else {
            out |= 0b0001;
        }
        
        data <<= 1;
    }
    
    out
}