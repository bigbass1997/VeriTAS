use serialport::SerialPortInfo;
use crate::DeviceError;
use crate::devices::{ReplayDevice, ReplayInterface};

#[derive(Debug)]
pub struct TAStm32 {
    
}
impl From<TAStm32> for ReplayDevice {
    fn from(value: TAStm32) -> Self {
        Self::TAStm32(value)
    }
}
impl ReplayInterface for TAStm32 {
    fn new(_info: &SerialPortInfo) -> Option<ReplayDevice> {
        None
    }
    
    fn clear_usb_buffers(&mut self) -> bool {
        todo!()
    }
    
    fn ping(&mut self) -> Result<(), DeviceError> {
        todo!()
    }
}