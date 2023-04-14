use std::io::{Read, Write};
use std::time::Duration;
use bincode::{Decode, Encode};
use bincode::config::Configuration;
use num_enum::{FromPrimitive, IntoPrimitive};
use serialport::{ClearBuffer, SerialPort};
use tasd::spec::Transition;

const BINCODE_CONFIG: Configuration = bincode::config::standard();

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub enum Command {
    ProvideInput(System, Vec<u8>),
    ProvideTransitions(Vec<TransitionData>),
    SetReplayMode(VeritasMode),
    SetReplayLength(u64),
    SetLatchFilter(u32),
    UseInitialReset(bool),
    GetStatus(System),
    Ping,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub enum Response {
    Ok,
    DeviceStatus(String),
    BufferStatus {
        written: u16,
        remaining_space: u16,
    },
    Pong,
    Err,
}
impl Response {
    pub fn is_not_ok(&self) -> bool {
        self != &Response::Ok
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub struct TransitionData {
    index: u64,
    index_kind: u8,
    transition_kind: u8,
}
impl TransitionData {
    pub fn from_vec(other: Vec<Transition>) -> Vec<Self> {
        other.into_iter().map(TransitionData::from).collect()
    }
}
impl From<Transition> for TransitionData {
    fn from(value: Transition) -> Self {
        Self {
            index: value.index,
            index_kind: value.index_kind,
            transition_kind: value.transition_kind
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Encode, Decode, FromPrimitive, IntoPrimitive)]
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

#[derive(Debug, PartialEq, Eq, Copy, Clone, Encode, Decode)]
#[repr(u8)]
pub enum VeritasMode {
    Initial = 0x00,
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
    
    pub fn send_command(&mut self, command: Command) -> Response {
        let payload = bincode::encode_to_vec(command, BINCODE_CONFIG).expect("failed to encode command, this should never happen");
        
        let mut data = (payload.len() as u32).to_be_bytes().to_vec();
        data.extend_from_slice(&payload);
        
        self.write(&data);
        
        self.recv_response()
    }
    
    fn recv_response(&mut self) -> Response {
        let len = u32::from_be_bytes(self.read(4).try_into().unwrap());
        let payload = self.read(len as usize);
        let (response, _) = bincode::decode_from_slice(&payload, BINCODE_CONFIG).expect("failed to decode response, this should never happen");
        
        response
    }
    
    pub fn clear(&self, buffer: ClearBuffer) {
        self.inner.clear(buffer).unwrap();
    }
    
    #[allow(unused)]
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
    
    #[allow(unused)]
    pub fn write_u8(&mut self, data: u8) {
        self.inner.write_all(&[data]).unwrap();
    }
    
    pub fn write(&mut self, data: &[u8]) {
        self.inner.write_all(data).unwrap();
    }
}