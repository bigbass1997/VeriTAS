use cortex_m::delay::Delay;
use defmt::info;
use heapless::spsc::Queue;
use pio_proc::pio_asm;
use pio::{InstructionOperands, JmpCondition, SetDestination};
use rp_pico::pac::io_bank0::gpio::gpio_ctrl::FUNCSEL_A;
use crate::hal::{gpio, pio as p};
use crate::hal::pio::{PioSel, ShiftDirection, SmSel};
use crate::hal::pio::PioOption::{Autopull, ClockDiv, InBase, InShiftdir, OutBase, OutCount, PullThresh, SetBase, SetCount, WrapBottom, WrapTop};
use crate::replaycore::{VERITAS_MODE, VeritasMode};

/// Buffered list of controller inputs. 
pub static mut INPUT_BUFFER: Queue<[u32; 4], 1024> = Queue::new();

static mut READ_BYTE_VECTOR: u8 = 0;
static mut WRITE_BYTES_VECTOR: u8 = 0;

/// Prepares the device to replay a TAS.
pub fn initialize() {
    // Data
    gpio::set_function(14, FUNCSEL_A::PIO0);
    gpio::set_pull_down_enable(14, false);
    gpio::set_pull_up_enable(14, false);
    
    // Debug
    gpio::set_function(15, FUNCSEL_A::PIO0);
    gpio::set_pull_down_enable(15, false);
    gpio::set_pull_up_enable(15, false);
    
    // Detect
    gpio::set_function(16, FUNCSEL_A::SIO);
    gpio::set_pull_down_enable(16, false);
    gpio::set_pull_up_enable(16, false);
    gpio::set_input_enable(16, true);
    gpio::set_output_disable(16, true);
    
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
    p::start(PioSel::Zero, SmSel::Zero);
}

pub fn run(delay: &mut Delay) {
    unsafe {
        initialize();
        
        info!("starting N64 replay..");
        
        while gpio::is_low(16) {}
        delay.delay_ms(100);
        
        while VERITAS_MODE == VeritasMode::ReplayN64 {
            let cmd = read_blocking();
            match cmd {
                0xFF | 0x01 => {
                    //delay.delay_us(4);
                    let state = INPUT_BUFFER.dequeue().unwrap_or_default();
                    write_blocking(&state[0].to_be_bytes());
                    delay.delay_us(16);
                },
                0x00 => {
                    //delay.delay_us(4);
                    write_blocking(&[0x05, 0x00, 0x02]);
                    delay.delay_us(16);
                },
                0x02 | 0x03 => {
                    delay.delay_us(150);
                }
                _ => ()
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