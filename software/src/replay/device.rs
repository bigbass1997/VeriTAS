
use serialport::{SerialPort, ClearBuffer};
use std::time::Duration;
use std::io::{Write, Read};






pub struct Device {
    pub port: Option<Box<dyn SerialPort>>,
}
impl Device {
    pub fn new() -> Self {
        Self {
            port: None,
        }
    }
    
    pub fn ping(&mut self) -> Result<String, u8> {
        self.port.as_mut().unwrap().clear(ClearBuffer::All).unwrap();
        let mut buf = [0u8];
        Self::write_read(self.port.as_mut().unwrap(), &[0xFE], &mut buf);
        
        if buf[0] == 0xEF {
            Result::Ok(String::from("pong!"))
        } else {
            Result::Err(buf[0])
        }
    }
    
    
    pub fn cli_select_port(&mut self, use_first: bool) -> bool {
        let ports_result = serialport::available_ports();
        
        if ports_result.is_ok() {
            let ports = ports_result.unwrap();
            
            if ports.is_empty() {
                return false;
            }
            
            if use_first {
                println!("Loading default port {}", ports.first().unwrap().port_name);
                self.port = Some(serialport::new(ports.first().unwrap().port_name.clone(), 500000).timeout(Duration::from_secs(30)).open().expect("Failed to open port!"));
                
                return true;
            }
            
            ports.iter().enumerate().for_each(|(i, info)| {
                println!("{}: {:?}", i, info);
            });
            print!("Choose port[0-{}]: ", (ports.len() - 1));
            std::io::stdout().flush().unwrap();
            let index: isize = Self::read_cli().parse().unwrap_or(-1);
            
            if index < 0 || index > (ports.len() - 1) as isize {
                println!("Invalid option!");
                return false;
            }
            
            println!("Loading port #{} [{}]", index, ports[index as usize].port_name);
            self.port = Some(serialport::new(ports[index as usize].port_name.clone(), 500000).timeout(Duration::from_secs(30)).open().expect("Failed to open port!"));
            
            return true;
        }
        
        false
    }
    
    fn read_cli() -> String {
        let mut cli_input = String::new();
        std::io::stdin().read_line(&mut cli_input).unwrap();
        
        cli_input.trim().to_string()
    }
    
    pub fn write_read(port: &mut Box<dyn SerialPort>, write_buf: &[u8], read_buf: &mut [u8]) {
        port.write_all(write_buf).unwrap();
        port.read_exact(read_buf).unwrap();
    }
}
