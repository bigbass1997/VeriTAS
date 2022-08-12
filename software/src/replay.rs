use std::io::{Read, Write};
use std::time::Duration;
use bytes::{Buf, Bytes};
use clap::ArgMatches;
use log::{error, info};
use serialport::ClearBuffer;

mod device;

pub fn handle(matches: &ArgMatches) {
    if matches.is_present("list-devices") {
        for port in serialport::available_ports().unwrap() {
            info!("{:?}", port);
        }
        
        return;
    }
    
    let movie = std::fs::read(matches.value_of("movie").unwrap()).unwrap();
    let mut device = {
        let port_name = matches.value_of("device").unwrap();
        
        let mut device = None;
        for port in serialport::available_ports().unwrap_or_default() {
            if port.port_name == port_name {
                device = Some(serialport::new(port.port_name, 500000).timeout(Duration::from_secs(10)).open().unwrap());
                break;
            }
        }
        
        device
    }.unwrap();
    
    /*let version = u32::from_be_bytes((&movie[4..8]).try_into().unwrap());
    let start = if version == 1 || version == 2 { 0x200 } else { 0x400 };
    let controllers = movie[0x15] as usize;
    
    let inputs = {
        let mut data = Bytes::from(movie[start..].to_vec());
        let mut inputs: Vec<[u32; 4]> = vec![[0; 4]];
        
        while data.has_remaining() {
            let mut input = [0u32; 4];
            for i in 0..controllers {
                input[i] = data.get_u32();
            }
            inputs.push(input);
        }
        
        inputs
    };
    
    device.clear(ClearBuffer::All).unwrap_or_default();
    device.write_all(&[0x02]).unwrap();
    let mut buf = [0u8];
    device.read_exact(&mut buf).unwrap();
    if buf[0] != 0x20 {
        error!("Unexpected value returned: {:#04X}", buf[0]);
        return;
    }
    info!("Starting");
    
    let mut ptr = 0usize;
    loop {
        let input = inputs[ptr][0].to_be_bytes();
        
        device.write_all(&[0x01, input[0], input[1], input[2], input[3]]).unwrap();
        let mut buf = [0u8];
        device.read_exact(&mut buf).unwrap();
        
        if buf[0] == 0x01 {
            ptr += 1;
        } else {
            std::thread::sleep(Duration::from_secs(1));
        }
        if ptr == inputs.len() {
            break;
        }
    }*/
    
    device.clear(ClearBuffer::All).unwrap_or_default();
    device.write_all(&[0x04]).unwrap();
    let mut buf = [0u8];
    device.read_exact(&mut buf).unwrap();
    if buf[0] != 0x40 {
        error!("Unexpected value returned: {:#04X}", buf[0]);
        return;
    }
    info!("Starting");
    
    let mut ptr = 0usize;
    loop {
        device.write_all(&[0x03, !movie[ptr], !movie[ptr + 1]]).unwrap();
        let mut buf = [0u8];
        device.read_exact(&mut buf).unwrap();
        
        if buf[0] == 0x03 {
            ptr += 2;
        } else {
            std::thread::sleep(Duration::from_secs(1));
        }
        if ptr == movie.len() {
            break;
        }
    }
    
    info!("Buffer filling complete!");
}