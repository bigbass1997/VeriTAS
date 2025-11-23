use std::io::{Read, Write};
use std::time::Duration;
use bincode::config::Configuration;
use bincode::{Decode, Encode};
use serialport::{ClearBuffer, SerialPort, SerialPortInfo, SerialPortType};
use crate::DeviceError;
use crate::devices::{ReplayDevice, ReplayInterface};

const BAUD: u32 = 500_000;
const TIMEOUT: Duration = Duration::from_secs(6);
//pub const BINCODE_CFG: Configuration<BigEndian, Fixint> = bincode::config::standard().with_big_endian().with_fixed_int_encoding();
pub const BINCODE_CFG: Configuration = bincode::config::standard();

/// Sent by the host to provide instructions or information to a replay device.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
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
pub struct TransitionData {
    index: u64,
    index_kind: u8,
    transition_kind: u8,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Encode, Decode)]
#[repr(u8)]
pub enum System {
    Nes = 0x01,
    Snes = 0x02,
    N64 = 0x03,
    Genesis = 0x08,
    A2600 = 0x09,
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

/// Sent by the replay device to acknowledge a command or provide other information to the host.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum Message {
    Ok,
    DeviceStatus(String),
    BufferStatus {
        written: u32,
        remaining_space: u32,
    },
    Pong,
    Err,
}

#[derive(Debug)]
pub struct VeriTAS {
    inner: Box<dyn SerialPort>,
}
impl From<VeriTAS> for ReplayDevice {
    fn from(value: VeriTAS) -> Self {
        Self::VeriTAS(value)
    }
}
impl ReplayInterface for VeriTAS {
    fn new(info: &SerialPortInfo) -> Option<ReplayDevice> {
        
        let builder = serialport::new(&info.port_name, BAUD).timeout(TIMEOUT);
        
        match &info.port_type {
            SerialPortType::UsbPort(info) => {
                if info.product.as_ref().is_some_and(|s| s == "VeriTAS") {
                    return Some(Self {
                        inner: builder
                            .open()
                            .ok()?
                    }.into());
                }
            },
            _ => ()
        }
        
        None
    }
    
    fn clear_usb_buffers(&mut self) -> bool {
        self.inner.clear(ClearBuffer::All).is_ok()
    }
    
    fn ping(&mut self) -> Result<(), DeviceError> {
        self.send(Command::Ping)?;
        
        let msg = self.recv()?;
        
        if let Message::Pong = msg {
            Ok(())
        } else {
            Err(DeviceError::UnexpectedResponse(format!("expected pong but received {msg:?}")))
        }
    }
}

impl VeriTAS {
    pub fn send(&mut self, cmd: Command) -> Result<(), DeviceError> {
        let payload = bincode::encode_to_vec(cmd, BINCODE_CFG).expect("should encode successfully");
        assert!(payload.len() <= u32::MAX as usize, "encoded command was larger than u32::MAX bytes");
        
        let mut buf = (payload.len() as u32).to_be_bytes().to_vec();
        buf.extend(payload);
        
        self.inner.write_all(&buf)?;
        
        Ok(())
    }
    
    pub fn recv(&mut self) -> Result<Message, DeviceError> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        let len = u32::from_be_bytes(buf) as usize;
        
        let mut buf = vec![0; len];
        self.inner.read_exact(&mut buf)?;
        
        let (msg, bytes) = bincode::decode_from_slice(&buf, BINCODE_CFG)?;
        assert_eq!(len, bytes, "decoded message used {bytes} bytes when it should have used {len} bytes");
        
        Ok(msg)
    }
    
    pub fn bytes_to_read(&mut self) -> Option<u32> {
        self.inner.bytes_to_read().ok()
    }
    
    pub fn bytes_to_write(&mut self) -> Option<u32> {
        self.inner.bytes_to_write().ok()
    }
    
    pub fn recv_all(&mut self) -> Result<Vec<Message>, DeviceError> {
        let mut messages = Vec::with_capacity(1);
        loop {
            if let Some(bytes) = self.bytes_to_read() && bytes == 0 {
                break
            }
            
            match self.recv() {
                Ok(msg) => messages.push(msg),
                Err(DeviceError::Io(err)) => if let std::io::ErrorKind::TimedOut = err.kind() {
                    break
                } else {
                    return Err(DeviceError::Io(err))
                },
                Err(err) => return Err(err),
            }
        }
        
        Ok(messages)
    }
}