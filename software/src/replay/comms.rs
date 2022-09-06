use std::io::{Read, Write};
use std::time::Duration;
use log::error;
use num_enum::{IntoPrimitive, FromPrimitive};
use serialport::{ClearBuffer, SerialPort};

#[derive(Debug, PartialEq, Eq, Copy, Clone, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Command {
    SetReplayMode = 0x01,
    ProvideInput = 0x02,
    GetStatus = 0x03,
    
    Ping = 0xAA,
    
    #[num_enum(default)]
    Invalid = 0x00,
}


#[derive(Debug, PartialEq, Eq, Copy, Clone, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Response {
    Ok = 0x01,
    Text = 0x02,
    BufferFull = 0xF0,
    
    Pong = 0x55,
    
    #[num_enum(default)]
    Err = 0x00,
}


#[derive(Debug, PartialEq, Eq, Copy, Clone, IntoPrimitive, FromPrimitive)]
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

#[derive(Debug, PartialEq, Eq, Copy, Clone, IntoPrimitive, FromPrimitive)]
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

pub struct Device {
    inner: Box<dyn SerialPort>,
}
impl Device {
    pub fn new<S: AsRef<str>>(port_name: S, baud: u32, timeout: Duration) -> Option<Self> {
        let mut inner = None;
        for port in serialport::available_ports().unwrap_or_default() {
            if port.port_name == port_name.as_ref() {
                inner = Some(serialport::new(port.port_name, baud).timeout(timeout).open().unwrap());
                break;
            }
        }
        if inner.is_none() {
            return None;
        }
        
        Some(Self {
            inner: inner.unwrap()
        })
    }
    
    pub fn set_replay_mode(&mut self, mode: VeritasMode) -> Response {
        self.write(&[Command::SetReplayMode.into(), mode.into()]);
        
        self.read_u8().into()
    }
    
    pub fn provide_input(&mut self, system: System, data: &[u8]) -> (Response, u16, u16, System, Vec<u8>) {
        self.write(&[&[Command::ProvideInput.into(), system.into()], (data.len() as u16).to_be_bytes().as_slice(), data].concat());
        let res = self.read(data.len() + 6);
        
        (res[0].into(), u16::from_be_bytes([res[1], res[2]]), u16::from_be_bytes([res[3], res[4]]), res[5].into(), res[6..].to_vec())
    }
    
    pub fn get_status(&mut self) -> String {
        self.write_u8(Command::GetStatus.into());
        let res = self.read(5);
        
        if Response::from(res[0]) != Response::Text {
            error!("Invalid GetStatus response!");
            
            String::new()
        } else {
            let res = self.read(u32::from_be_bytes(res[1..].try_into().unwrap()) as usize);
            
            String::from_utf8_lossy(&res).to_string()
        }
    }
    
    pub fn ping(&mut self) -> Response {
        self.write_u8(Command::Ping.into());
        
        self.read_u8().into()
    }
    
    
    pub fn clear(&self, buffer: ClearBuffer) {
        self.inner.clear(buffer).unwrap();
    }
    
    pub fn read_u8(&mut self) -> u8 {
        let mut buf = [0u8];
        self.inner.read_exact(&mut buf).unwrap();
        
        buf[0]
    }
    
    pub fn read(&mut self, len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        self.inner.read_exact(&mut buf).unwrap();
        
        buf
    }
    
    pub fn write_u8(&mut self, data: u8) {
        self.inner.write_all(&[data]).unwrap();
    }
    
    pub fn write(&mut self, data: &[u8]) {
        self.inner.write_all(data).unwrap();
    }
}