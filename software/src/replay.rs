use std::cmp::{max, min};
use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;
use clap::ArgMatches;
use crossterm::{event, terminal};
use crossterm::event::{Event, KeyCode};
use log::{error, info, warn};
use serialport::{ClearBuffer, SerialPortType};
use tasd::spec::{ConsoleType, InputChunk, KEY_CONSOLE_TYPE, KEY_INPUT_CHUNK, KEY_TRANSITION, TasdMovie, Transition};
use crate::replay::comms::{Device, Response, System, VeritasMode};

mod comms;

pub fn handle(matches: &ArgMatches) {
    if matches.is_present("list-devices") {
        for port in serialport::available_ports().unwrap() {
            info!("{:?}", port);
        }
        
        return;
    }
    
    let device_path = if let Some(device) = matches.value_of("device") {
        device.to_string()
    } else {
        serialport::available_ports().unwrap()
            .into_iter()
            .filter_map(|info| if let SerialPortType::UsbPort(usbport) = info.port_type { Some((info.port_name, usbport)) } else { None })
            .find(|(_, port)| port.serial_number == Some("VeriTAS".into()))
            .map(|(name, _)| name)
            .unwrap()
    };
    let mut dev = Device::new(device_path, 500000, Duration::from_secs(6)).unwrap();
    dev.clear(ClearBuffer::All);
    
    {
        let pong = dev.ping();
        if pong != Response::Pong {
            error!("Failed to ping device. {:?}", pong);
            return;
        }
    }
    
    if matches.is_present("manual") {
        let _stdout = stdout();
        
        dev.set_replay_mode(VeritasMode::ReplayNes);
        
        terminal::enable_raw_mode().unwrap();
        
        loop {
            match event::read() {
                Ok(event) => match event {
                    Event::Key(event) => match event.code {
                        KeyCode::Char('q') => break,
                        
                        KeyCode::Char('z') => { dev.provide_input(System::Nes, &[0x7F, 0xFF]); },
                        KeyCode::Char('x') => { dev.provide_input(System::Nes, &[0xBF, 0xFF]); },
                        KeyCode::Char(' ') => { dev.provide_input(System::Nes, &[0xDF, 0xFF]); },
                        KeyCode::Enter => { dev.provide_input(System::Nes, &[0xEF, 0xFF]); },
                        KeyCode::Up => { dev.provide_input(System::Nes, &[0xF7, 0xFF]); },
                        KeyCode::Down => { dev.provide_input(System::Nes, &[0xFB, 0xFF]); },
                        KeyCode::Left => { dev.provide_input(System::Nes, &[0xFD, 0xFF]); },
                        KeyCode::Right => { dev.provide_input(System::Nes, &[0xFE, 0xFF]); },
                        _ => ()
                    }
                    _ => ()
                }
                Err(_) => ()
            }
        }
        
        terminal::disable_raw_mode().unwrap();
        
        dev.set_replay_mode(VeritasMode::Idle);
        println!("");
        return;
    }
    
    let tasd = TasdMovie::new(&PathBuf::from(matches.value_of("movie").unwrap())).expect("Failed to parse movie.");
    let console = tasd.search_by_key(vec![KEY_CONSOLE_TYPE]).first().expect("No console type provided in TASD. Cannot continue.").as_any().downcast_ref::<ConsoleType>().unwrap();
    let mut transitions: Vec<Transition> = tasd.search_by_key(vec![KEY_TRANSITION]).into_iter().map(|packet| packet.as_any().downcast_ref::<Transition>().unwrap().clone()).collect();
    for trans in &mut transitions {
        trans.index /= 2;
        
        println!("{trans}");
    }
    let inputs: Vec<u8> = {
        let chunks: Vec<&[u8]> = tasd.search_by_key(vec![KEY_INPUT_CHUNK]).iter().map(|packet| packet.as_any().downcast_ref::<InputChunk>().unwrap().inputs.as_slice()).collect();
        //let mut inputs = vec![0xFF, 0xFF];
        let mut inputs = vec![];
        for chunk in chunks {
            inputs.extend_from_slice(chunk);
        }
        
        /*for (i, chunk) in inputs.chunks_exact(2).enumerate() {
            println!("{i:>4}: {:02X} {:02X}", chunk[0], chunk[1]);
        }
        return;*/
        
        //inputs.extend_from_slice(&vec![0xFFu8; 2 * 60 * 10]);
        
        inputs
    };
    
    match console.kind.into() {
        System::Nes => {
            let mut ptr = 0usize;
            let mut has_started = false;
            let mut prev_empty = 512;
            
            dev.set_replay_length((inputs.len() / 2) as u32);
            dev.provide_transitions(transitions);
            
            info!("{}", dev.get_status());
            
            println!("Prefilling buffer...");
            while ptr < inputs.len() {
                let remaining = inputs.len() - ptr;
                let input = &inputs[ptr..(ptr + max(2, min(prev_empty, min(128, remaining))))];
                let (res, written, empty_space, system, data) = dev.provide_input(System::Nes, input);
                ptr += written as usize;
                prev_empty = empty_space as usize;
                
                if system != System::Nes {
                    warn!("Possible communication error! System doesn't match: (sent) {:?} vs (recv) {:?}", System::Nes, system);
                }
                if data != input {
                    warn!("Possible communication error! Inputs don't match: (sent) {} vs (recv) {}", hex::encode_upper(input), hex::encode_upper(data));
                }
                
                match res {
                    Response::Ok => (),
                    Response::BufferFull if !has_started => {
                        has_started = true;
                        
                        let res = dev.set_replay_mode(VeritasMode::ReplayNes);
                        if res != Response::Ok {
                            error!("Failed to set replay mode! {:?}", res);
                            return;
                        }
                        info!("Starting replay.")
                    },
                    Response::BufferFull => (),
                    
                    _ => warn!("Error occurred while sending inputs: {:?}", res)
                }
            }
        },
        System::N64 => {
            todo!()
        },
        System::Genesis => {
            todo!()
        },
        System::A2600 => {
            todo!()
        },
        _ => unimplemented!()
    }
    
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
    
    /*device.clear(ClearBuffer::All).unwrap_or_default();
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
    
    info!("Buffer filling complete!");*/
}