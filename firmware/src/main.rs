
#![allow(unused_unsafe)]
#![no_std]
#![no_main]

use defmt::info;
use embedded_time::rate::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::InputPin;
use panic_probe as _;

use rp_pico::hal::{clocks::Clock, pac, sio::Sio, uart, watchdog::Watchdog};
use embedded_hal::serial::Read;
use embedded_hal::watchdog::WatchdogDisable;
use pio::{InstructionOperands, JmpCondition};
use pio_proc::pio_asm;
use rp_pico::hal::clocks::{ClocksManager, ClockSource};
use rp_pico::hal::gpio::{FunctionPio0, FunctionUart, Pin};
use rp_pico::hal::pll::{PLLConfig, setup_pll_blocking};
use rp_pico::hal::pll::common_configs::{PLL_USB_48MHZ};
use rp_pico::hal::xosc::setup_xosc_blocking;
use rp_pico::hal::pio::{PinDir, PIOBuilder, PIOExt, Running, ShiftDirection, SM0, StateMachine, Tx, ValidStateMachine};
use rp_pico::hal::pio::Rx;
use rp_pico::hal::uart::UartPeripheral;
use rp_pico::pac::PIO0;

pub const PLL_SYS_160MHZ: PLLConfig<Megahertz> = PLLConfig {
        vco_freq: Megahertz(1440),
        refdiv: 1,
        post_div1: 3,
        post_div2: 3,
};

static mut CNT_STATE: u32 = 0;

#[export_name = "main"]
pub unsafe extern "C" fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    watchdog.disable();
    
    let mut clocks = ClocksManager::new(pac.CLOCKS);
    let xosc = setup_xosc_blocking(pac.XOSC, 12000000.Hz()).ok().unwrap();
    let pll_sys = setup_pll_blocking(pac.PLL_SYS, xosc.operating_frequency().into(), PLL_SYS_160MHZ, &mut clocks, &mut pac.RESETS).ok().unwrap();
    let pll_usb = setup_pll_blocking(pac.PLL_USB, xosc.operating_frequency().into(), PLL_USB_48MHZ, &mut clocks, &mut pac.RESETS).ok().unwrap();
    clocks.reference_clock.configure_clock(&xosc, xosc.get_freq()).ok().unwrap();
    clocks.system_clock.configure_clock(&pll_sys, pll_sys.get_freq()).ok().unwrap();
    clocks.usb_clock.configure_clock(&pll_usb, pll_usb.get_freq()).ok().unwrap();
    clocks.adc_clock.configure_clock(&pll_usb, pll_usb.get_freq()).ok().unwrap();
    clocks.rtc_clock.configure_clock(&pll_usb, 46875u32.Hz()).ok().unwrap();
    clocks.peripheral_clock.configure_clock(&clocks.system_clock, clocks.system_clock.freq()).ok().unwrap();
    
    let core = pac::CorePeripherals::take().unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());
    
    let sio = Sio::new(pac.SIO);
    
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    
    let uart_pins = (
        pins.gpio8.into_mode::<FunctionUart>(),
        pins.gpio9.into_mode::<FunctionUart>(),
    );
    
    let mut uart = UartPeripheral::new(
        pac.UART1,
        uart_pins,
        &mut pac.RESETS
    ).enable(uart::common_configs::_115200_8_N_1, clocks.peripheral_clock.freq()).unwrap();
    
    //uart.write_full_blocking(b"UART TESTING!");
    
    let program = pio_asm!("
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
    ");
    
    let _: Pin<_, FunctionPio0> = pins.gpio14.into_mode();
    let _: Pin<_, FunctionPio0> = pins.gpio15.into_mode();
    let detect = pins.gpio16.into_floating_input();
    
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program.program).unwrap();
    let (mut sm, mut rx, mut tx) = PIOBuilder::from_program(installed)
        //.jmp_pin(14)
        .in_pin_base(14)
        .out_pins(14, 1)
        .set_pins(14, 2)
        .in_shift_direction(ShiftDirection::Left)
        .pull_threshold(32)
        .autopull(true)
        .clock_divisor(10.0)
        .build(sm0);
    sm.set_pindirs([(14, PinDir::Input), (15, PinDir::Output)]);
    
    info!("starting.. {}", sm.instruction_address());
    let mut sm = sm.start();
    
    let rx = &mut rx;
    let tx = &mut tx;
    
    loop {
        if detect.is_high().unwrap() {
            break;
        }
    }
    delay.delay_ms(100);
    
    let mut ptr = 3;
    loop {
        let cmd = read_blocking(&mut sm, program.public_defines.read_byte, rx);
        match cmd {
            0xFF | 0x01 => {
                delay.delay_us(4);
                write_blocking(&mut sm, program.public_defines.write_bytes, tx, &CNT_STATE.to_be_bytes());
                //info!("{:08X}", CNT_STATE);
                delay.delay_us(40);
            },
            0x00 => {
                delay.delay_us(4);
                write_blocking(&mut sm, program.public_defines.write_bytes, tx, &[0x05, 0x00, 0x02]);
                delay.delay_us(40);
            }
            0x02 | 0x03 => {
                delay.delay_us(150);
            }
            _ => ()
        }
        
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
        
        //uart.write_full_blocking(&[cmd as u8]);
        /*let mut buf = [0u8; 4];
        
        if uart.read_raw(&mut buf).unwrap_or(0) == 4 {
            info!("{:08X}", u32::from_be_bytes(buf));
            CNT_STATE = u32::from_be_bytes(buf);
        }*/
    }
}

#[inline(always)]
fn read_blocking<SM: ValidStateMachine>(sm: &mut StateMachine<(PIO0, SM0), Running>, read_location: i32, rx: &mut Rx<SM>) -> u32 {
    sm.exec_instruction(InstructionOperands::JMP { condition: JmpCondition::Always, address: read_location as u32 as u8 }.encode());
    
    loop {
        match rx.read() {
            Some(data) => return data,
            None => ()
        }
    }
}

#[inline(always)]
fn write_blocking<SM: ValidStateMachine>(sm: &mut StateMachine<(PIO0, SM0), Running>, write_location: i32, tx: &mut Tx<SM>, data: &[u8]) {
    sm.exec_instruction(InstructionOperands::JMP { condition: JmpCondition::Always, address: write_location as u32 as u8 }.encode());
    
    for byte in data {
        let encoded = encode_joybus(*byte);
        
        loop {
            if tx.write(encoded) {
                break;
            }
        }
    }
    
    loop {
        if tx.write(0x3FFFFFFF) { // controller stop bit
            break;
        }
    }
    
    loop {
        if tx.is_empty() {
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