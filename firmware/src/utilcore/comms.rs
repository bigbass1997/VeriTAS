use alloc::{format, vec};
use num_enum::{IntoPrimitive, FromPrimitive};
use rp2040_hal::usb::UsbBus;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usbd_serial::SerialPort;
use defmt::info;
use crate::replaycore::{Transition, VERITAS_MODE, VeritasMode};
use crate::systems;
use crate::systems::nes::REPLAY_STATE;

#[derive(IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Command {
    SetReplayMode = 0x01,
    ProvideInput = 0x02,
    ProvideTransitions = 0x03,
    SetReplayLength = 0x04,
    GetStatus = 0x05,
    
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


pub struct UsbController<'a> {
    usb_bus: Option<UsbBusAllocator<UsbBus>>,
    usb_dev: Option<UsbDevice<'a, UsbBus>>,
    serial: Option<SerialPort<'a, UsbBus>>
}
impl<'a> UsbController<'a> {
    pub const fn empty() -> Self { Self {
        usb_bus: None,
        usb_dev: None,
        serial: None
    }}
    
    #[inline(always)]
    pub fn poll(&mut self) -> bool {
        if let Some(usb_dev) = self.usb_dev.as_mut() {
            if let Some(serial) = self.serial.as_mut() {
                return usb_dev.poll(&mut [serial]);
            }
        }
        
        false
    }
    
    #[inline(always)]
    pub fn read_one(&mut self) -> Option<u8> {
        if let Some(serial) = self.serial.as_mut() {
            let mut buf = [0u8];
            match serial.read(&mut buf) {
                Ok(count) if count == 1 => return Some(buf[0]),
                _ => ()
            }
        }
        
        None
    }
    
    #[inline(always)]
    pub fn read_one_blocking(&mut self) -> u8 {
        let serial = self.serial.as_mut().expect("USB serial not initialized");
        let mut buf = [0u8];
        loop {
            match serial.read(&mut buf) {
                Ok(count) if count == 1 => return buf[0],
                _ => ()
            }
        }
    }
    
    #[inline(always)]
    pub fn read_blocking(&mut self, buf: &mut [u8]) {
        let serial = self.serial.as_mut().expect("USB serial not initialized");
        let mut ptr = 0usize;
        
        while ptr < buf.len() {
            let mut data = [0u8];
            match serial.read(&mut data) {
                Ok(count) if count == 1 => {
                    buf[ptr] = data[0];
                    ptr += 1;
                },
                _ => ()
            }
        }
    }
    
    #[inline(always)]
    pub fn write_one_blocking(&mut self, buf: u8) {
        let serial = self.serial.as_mut().expect("USB serial not initialized");
        
        loop {
            match serial.write(&[buf]) {
                Ok(count) if count == 1 => break,
                _ => ()
            }
        }
    }
    
    #[inline(always)]
    pub fn write_blocking(&mut self, buf: &[u8]) {
        let serial = self.serial.as_mut().expect("USB serial not initialized");
        let mut ptr = 0usize;
        
        while ptr < buf.len() {
            match serial.write(&[buf[ptr]]) {
                Ok(count) if count == 1 => ptr += 1,
                _ => ()
            }
        }
    }
}

pub static mut USB: UsbController = UsbController::empty();

pub fn init_usb(usb_bus: UsbBusAllocator<UsbBus>) {
    unsafe {
        USB.usb_bus = Some(usb_bus);
        USB.serial = Some(SerialPort::new(USB.usb_bus.as_ref().unwrap()));
        USB.usb_dev = Some(UsbDeviceBuilder::new(USB.usb_bus.as_ref().unwrap(), UsbVidPid(0x16C0, 0x27DD))
            .manufacturer("Bigbass")
            .product("VeriTAS")
            .serial_number("VeriTAS") //TODO: provide version number
            .device_class(2)
            .self_powered(true)
            .build());
    }
}

pub fn check_usb() {
    unsafe {
        if !USB.poll() {
            return;
        }
        
        if let Some(cmd) = USB.read_one() {
            match cmd.into() {
                Command::SetReplayMode => {
                    VERITAS_MODE = USB.read_one_blocking().into();
                    
                    USB.write_one_blocking(Response::Ok.into());
                },
                Command::ProvideInput => {
                    match USB.read_one_blocking().into() {
                        System::Nes => {
                            use crate::systems::nes::INPUT_BUFFER;
                            let mut len = [0u8; 2];
                            USB.read_blocking(&mut len);
                            let len = u16::from_be_bytes([len[0], len[1]]);
                            
                            let mut inputs = vec![0u8; len as usize];
                            USB.read_blocking(&mut inputs);
                            let mut ptr = 0usize;
                            let status = loop {
                                if !INPUT_BUFFER.is_full() {
                                    if ptr < inputs.len() {
                                        let input = [inputs[ptr], inputs[ptr + 1]];
                                        INPUT_BUFFER.enqueue(input).unwrap();
                                        
                                        ptr += 2;
                                    } else {
                                        break (Response::Ok, ptr as u16, ((INPUT_BUFFER.capacity() - INPUT_BUFFER.len()) * 2) as u16);
                                    }
                                } else {
                                    break (Response::BufferFull, ptr as u16, 0);
                                }
                            };
                            
                            USB.write_blocking(&[&[status.0.into()], status.1.to_be_bytes().as_slice(), status.2.to_be_bytes().as_slice(), &[System::Nes.into()], inputs.as_slice()].concat());
                        },
                        System::N64 => {
                            use crate::systems::n64::INPUT_BUFFER;
                            
                            let mut input = [0u8; 16];
                            USB.read_blocking(&mut input);
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
                            
                            USB.write_blocking(&[&[status.into(), System::N64.into()], input.as_slice()].concat());
                        },
                        System::Genesis => {
                            USB.write_one_blocking(Response::Err.into());
                        },
                        System::A2600 => {
                            USB.write_one_blocking(Response::Err.into());
                        },
                        System::Unknown => {
                            USB.write_one_blocking(Response::Err.into());
                        },
                    }
                },
                Command::ProvideTransitions => {
                    let mut count = [0u8; 4];
                    USB.read_blocking(&mut count);
                    
                    for _ in 0..u32::from_be_bytes(count) {
                        let mut buf = [0u8; 5];
                        USB.read_blocking(&mut buf);
                        
                        let tra: (u32, Transition) = (u32::from_be_bytes(buf[0..4].try_into().unwrap()), buf[4].into());
                        let u: u8 = tra.1.into();
                        info!("Added {:02X} at {}", u, tra.0);
                        REPLAY_STATE.transitions.push(tra);
                    }
                    
                    USB.write_one_blocking(Response::Ok.into());
                },
                Command::SetReplayLength => {
                    let mut length = [0u8; 4];
                    USB.read_blocking(&mut length);
                    
                    REPLAY_STATE.index_len = u32::from_be_bytes(length);
                    
                    USB.write_one_blocking(Response::Ok.into());
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
                    USB.write_blocking(&[&[Response::Text.into()], (s.len() as u32).to_be_bytes().as_slice(), s.as_bytes()].concat());
                }
                
                Command::Ping => {
                    USB.write_one_blocking(Response::Pong.into());
                }
                
                Command::Invalid => {
                    USB.write_one_blocking(Response::Err.into());
                },
            }
        }
    }
}