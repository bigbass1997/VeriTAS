use serialport::SerialPortInfo;
use crate::devices::tastm32::TAStm32;
use crate::devices::veritas::VeriTAS;

pub mod tastm32;
pub mod veritas;

#[derive(Debug)]
pub enum DeviceError {
    Io(std::io::Error),
    SerialPort(serialport::Error),
    Decode(bincode::error::DecodeError),
    
    UnexpectedResponse(String),
}
impl From<std::io::Error> for DeviceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serialport::Error> for DeviceError {
    fn from(value: serialport::Error) -> Self {
        Self::SerialPort(value)
    }
}
impl From<bincode::error::DecodeError> for DeviceError {
    fn from(value: bincode::error::DecodeError) -> Self {
        Self::Decode(value)
    }
}


pub trait ReplayInterface {
    fn new(info: &SerialPortInfo) -> Option<ReplayDevice>;
    
    fn clear_usb_buffers(&mut self) -> bool;
    
    fn ping(&mut self) -> Result<(), DeviceError>;
}

#[derive(Debug)]
pub enum ReplayDevice {
    VeriTAS(VeriTAS),
    TAStm32(TAStm32),
}

impl ReplayDevice {
    pub fn open_first() -> Option<Self> {
        serialport::available_ports()
            .ok()?
            .into_iter()
            .next()
            .map(|info| Self::try_open(&info))
            .flatten()
    }
    
    pub fn open_name<S: AsRef<str>>(name: S) -> Option<Self> {
        let name = name.as_ref();
        
        serialport::available_ports()
            .ok()?
            .into_iter()
            .find(|info| info.port_name == name)
            .map(|info| Self::try_open(&info))
            .flatten()
    }
    
    pub fn open_all() -> Vec<Self> {
        let Ok(ports) = serialport::available_ports() else { return vec![] };
        
        ports.into_iter()
            .filter_map(|info| Self::try_open(&info))
            .collect()
    }
    
    fn try_open(info: &SerialPortInfo) -> Option<Self> {
        VeriTAS::new(&info)
            .or_else(|| TAStm32::new(&info))
    }
    
    
    
    pub fn clear_usb_buffers(&mut self) -> bool {
        use ReplayDevice::*;
        match self {
            VeriTAS(d) => d.clear_usb_buffers(),
            TAStm32(d) => d.clear_usb_buffers(),
        }
    }
    
    pub fn ping(&mut self) -> Result<(), DeviceError> {
        use ReplayDevice::*;
        match self {
            VeriTAS(d) => d.ping(),
            TAStm32(d) => d.ping(),
        }
    }
    
    pub fn as_veritas(&mut self) -> Option<&mut VeriTAS> {
        if let Self::VeriTAS(d) = self {
            Some(d)
        } else {
            None
        }
    }
}
