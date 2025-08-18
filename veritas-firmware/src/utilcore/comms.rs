use alloc::{format, vec};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bincode::config::Configuration;
use bincode::{Decode, Encode};
use ringbuffer::RingBuffer;
use num_enum::{IntoPrimitive, FromPrimitive};
use rp2040_hal::usb::UsbBus;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usbd_serial::SerialPort;
use defmt::{info, println};
use usb_device::device::StringDescriptors;
use usb_device::LangID;
use crate::replaycore::{VERITAS_MODE, REPLAY_STATE, VeritasMode};
use crate::systems;

const BINCODE_CONFIG: Configuration = bincode::config::standard();

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub enum Command {
    ProvideInput(System, Vec<u8>),
    ProvideTransitions(Vec<TransitionData>),
    SetReplayMode(VeritasMode),
    SetReplayLength(u64),
    SetLatchFilter(u32),
    UseInitialReset(bool),
    GetStatus,
    Ping,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub enum Response {
    Ok,
    DeviceStatus(String),
    BufferStatus {
        written: u32,
        remaining_space: u32,
    },
    Pong,
    Err,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub struct TransitionData {
    index: u64,
    index_kind: u8,
    transition_kind: u8,
}


#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum System {
    Nes = 0x01,
    Snes = 0x02,
    N64 = 0x03,
    Genesis = 0x08,
    A2600 = 0x09,
    #[num_enum(default)]
    Unknown = 0xFF,
}


pub struct UsbController<'a> {
    pub usb_bus: Option<UsbBusAllocator<UsbBus>>,
    pub usb_dev: Option<UsbDevice<'a, UsbBus>>,
    pub serial: Option<SerialPort<'a, UsbBus>>
}
impl<'a> UsbController<'a> {
    pub const fn empty() -> Self { Self {
        usb_bus: None,
        usb_dev: None,
        serial: None
    }}
    
    #[inline(always)]
    pub fn try_recv_command(&mut self) -> Option<Command> {
        _try_recv_command(self)
    }
    
    
    #[inline(always)]
    pub fn send_response(&mut self, resp: Response) {
        _send_response(self, resp);
    }
    
    #[inline(always)]
    pub fn poll(&mut self) -> bool {
        if let Some(usb_dev) = self.usb_dev.as_mut() {
            if let Some(serial) = self.serial.as_mut() {
                return usb_dev.poll(&mut [serial]);
            }
        }
        
        false
    }
    
    #[allow(unused)]
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
    
    pub fn read_four(&mut self) -> Option<[u8; 4]> {
        if let Some(serial) = self.serial.as_mut() {
            let mut buf = [0u8; 4];
            
            match serial.read(&mut buf) {
                Ok(count) if count == 0 => return None,
                Ok(count) if count < 4 => {
                    for i in count..4 {
                        buf[i] = self.read_one_blocking();
                    }
                    
                    return Some(buf);
                },
                Ok(_) => return Some(buf),
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
                _ => (),
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
    
    #[allow(unused)]
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

#[unsafe(link_section = ".ram_code")]
fn _try_recv_command(usb: &mut UsbController) -> Option<Command> {
    if let Some(len) = usb.read_four() {
        let len = u32::from_be_bytes(len);
        let mut buf = vec![0u8; len as usize];
        usb.read_blocking(&mut buf);
        
        match bincode::decode_from_slice(&buf, BINCODE_CONFIG) {
            Ok((command, _)) => Some(command),
            Err(err) => {
                println!("err: {}", err.to_string().as_str());
                
                None
            },
        }
    } else {
        None
    }
}

#[unsafe(link_section = ".ram_code")]
fn _send_response(usb: &mut UsbController, resp: Response) {
    let payload = if let Ok(payload) = bincode::encode_to_vec(resp, BINCODE_CONFIG) {
        payload
    } else {
        return;
    };
    
    let mut data = (payload.len() as u32).to_be_bytes().to_vec();
    data.extend_from_slice(&payload);
    
    usb.write_blocking(&data);
}

pub static mut USB: UsbController = UsbController::empty();

pub fn init_usb(usb_bus: UsbBusAllocator<UsbBus>) {
    unsafe {
        USB.usb_bus = Some(usb_bus);
        USB.serial = Some(SerialPort::new(USB.usb_bus.as_ref().unwrap()));
        USB.usb_dev = Some(UsbDeviceBuilder::new(USB.usb_bus.as_ref().unwrap(), UsbVidPid(0x16C0, 0x27DD))
            .strings(&[
                StringDescriptors::new(LangID::EN_US)
                    .manufacturer("Bigbass")
                    .product("VeriTAS")
                    .serial_number("VeriTAS"), //TODO: provide version number
            ])
            .expect("valid string descriptors")
            .device_class(2)
            .self_powered(true)
            .build());
    }
}

#[unsafe(link_section = ".ram_code")]
pub fn check_usb() {
    unsafe {
        if !USB.poll() {
            return;
        }
        
        if let Some(cmd) = USB.try_recv_command() {
            match cmd {
                Command::ProvideInput(system, inputs) => {
                    match system {
                        System::Nes => {
                            use crate::systems::nes::INPUT_BUFFER;
                            let mut ptr = 0usize;
                            while !INPUT_BUFFER.is_full() && ptr <= inputs.len() - 2 /*&& ptr < (u32::MAX - 1) as usize*/ {
                                let input = [inputs[ptr], inputs[ptr + 1]];
                                INPUT_BUFFER.enqueue(input);
                                
                                ptr += 2;
                            }
                            
                            USB.send_response(Response::BufferStatus {
                                written: ptr as u32,
                                remaining_space: ((INPUT_BUFFER.capacity() - INPUT_BUFFER.len()) * 2) as u32,
                            });
                        },
                        System::Snes => {
                            USB.send_response(Response::Err);
                        },
                        System::N64 => {
                            /*use crate::systems::n64::INPUT_BUFFER;
                            
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
                            };*/
                            
                            USB.send_response(Response::Err);
                        },
                        System::Genesis => {
                            use crate::systems::genesis::INPUT_BUFFER;
                            
                            let mut ptr = 0usize;
                            while !INPUT_BUFFER.is_full() && ptr <= inputs.len() - 4 && ptr < (u16::MAX - 1) as usize {
                                let input = inputs[ptr..(ptr + 4)].try_into().unwrap();
                                INPUT_BUFFER.enqueue(input).unwrap();
                                
                                ptr += 4;
                            }
                            
                            USB.send_response(Response::BufferStatus {
                                written: ptr as u32,
                                remaining_space: ((INPUT_BUFFER.capacity() - INPUT_BUFFER.len()) * 4) as u32,
                            });
                        },
                        System::A2600 => {
                            USB.send_response(Response::Err);
                        },
                        System::Unknown => {
                            USB.send_response(Response::Err);
                        },
                    }
                },
                Command::ProvideTransitions(transitions) => {
                    REPLAY_STATE.transitions.extend(
                        transitions.into_iter()
                            .map(|tra| (tra.index as u32, tra.transition_kind.into()))
                            .inspect(|(index, tra)| { info!("Added {:02X} at {}", u8::from(*tra), index); })
                    );
                    
                    USB.send_response(Response::Ok);
                },
                Command::SetReplayMode(mode) => {
                    VERITAS_MODE = mode;
                    
                    USB.send_response(Response::Ok);
                },
                Command::SetReplayLength(length) => {
                    REPLAY_STATE.index_len = length as u32; //TODO handle what happens if length > u32
                    
                    USB.send_response(Response::Ok);
                },
                Command::SetLatchFilter(time) => {
                    systems::nes::LATCH_FILTER_US = time;
                    
                    USB.send_response(Response::Ok);
                },
                Command::UseInitialReset(use_reset) => {
                    REPLAY_STATE.use_initial_reset = use_reset;
                    
                    USB.send_response(Response::Ok);
                },
                Command::GetStatus => {
                    USB.send_response(Response::DeviceStatus(format!("Mode: {:?}, Index: {}/{}", VERITAS_MODE, REPLAY_STATE.index_cur, REPLAY_STATE.index_len)));
                },
                Command::Ping => {
                    USB.send_response(Response::Pong);
                },
            }
        }
    }
}