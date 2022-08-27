use alloc::format;
use num_enum::{IntoPrimitive, FromPrimitive};
use crate::hal::uart;
use crate::replaycore::{VERITAS_MODE, VeritasMode};
use crate::{info, systems};

#[derive(IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Command {
    SetReplayMode = 0x01,
    ProvideInput = 0x02,
    GetStatus = 0x03,
    
    Ping = 0xAA,
    
    #[num_enum(default)]
    Invalid = 0x00,
}


#[derive(IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Response {
    Ok = 0x01,
    Text = 0x02,
    BufferFull = 0xF0,
    
    Pong = 0x55,
    
    #[num_enum(default)]
    Err = 0x00,
}


#[derive(IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum System {
    Nes = 0x01,
    //Snes = 0x02,
    N64 = 0x03,
    Genesis = 0x08,
    A2600 = 0x09,
    #[num_enum(default)]
    Unknown = 0xFF,
}

pub fn check_uart() {
    unsafe {
        if let Some(cmd) = uart::read_one(1) {
            match cmd.into() {
                Command::SetReplayMode => {
                    VERITAS_MODE = uart::read_one_blocking(1).into();
                    
                    uart::write_one_blocking(1, Response::Ok.into());
                },
                Command::ProvideInput => {
                    match uart::read_one_blocking(1).into() {
                        System::Nes => {
                            use crate::systems::nes::INPUT_BUFFER;
                            
                            let mut input = [0u8; 2];
                            uart::read_blocking(1, &mut input);
                            let status = if !INPUT_BUFFER.is_full() {
                                INPUT_BUFFER.enqueue(input).unwrap();
                                
                                Response::Ok
                            } else {
                                Response::BufferFull
                            };
                            
                            uart::write_blocking(1, &[&[status.into(), System::Nes.into()], input.as_slice()].concat());
                        },
                        System::N64 => {
                            use crate::systems::n64::INPUT_BUFFER;
                            
                            let mut input = [0u8; 16];
                            uart::read_blocking(1, &mut input);
                            let status = if !INPUT_BUFFER.is_full() {
                                let mut inputs = [0u32; 4];
                                for i in 0..4 {
                                    inputs[i] = u32::from_be_bytes(input[(i * 4)..((i + 1) * 4)].try_into().unwrap());
                                }
                                
                                INPUT_BUFFER.enqueue(inputs).unwrap();
                                
                                Response::Ok
                            } else {
                                Response::BufferFull
                            };
                            
                            uart::write_blocking(1, &[&[status.into(), System::N64.into()], input.as_slice()].concat());
                        },
                        System::Genesis => {
                            uart::write_one_blocking(1, Response::Err.into());
                        },
                        System::A2600 => {
                            uart::write_one_blocking(1, Response::Err.into());
                        },
                        System::Unknown => {
                            uart::write_one_blocking(1, Response::Err.into());
                        },
                    }
                },
                Command::GetStatus => {
                    let mode = VERITAS_MODE;
                    let index: (u32, u32) = match mode {
                        VeritasMode::ReplayNes => (
                            systems::nes::REPLAY_STATE.index_cur,
                            systems::nes::REPLAY_STATE.index_len,
                        ),
                        _ => (0, 0)
                    };
                    
                    let s = format!("Mode: {:?}, Index: {}/{}", mode, index.0, index.1);
                    uart::write_blocking(1, &[&[Response::Text.into()], (s.len() as u32).to_be_bytes().as_slice(), s.as_bytes()].concat());
                }
                
                Command::Ping => {
                    uart::write_one_blocking(1, Response::Pong.into());
                }
                
                Command::Invalid => {
                    uart::write_one_blocking(1, Response::Err.into());
                },
            }
        }
    }
}