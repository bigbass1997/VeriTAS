use std::path::PathBuf;
use std::time::Duration;
use clap::ArgMatches;
use log::{error, info, warn};
use serialport::ClearBuffer;
use tasd::spec::{ConsoleType, InputChunk, KEY_CONSOLE_TYPE, KEY_INPUT_CHUNK, TasdMovie};
use crate::replay::comms::{Device, Response, System, VeritasMode};

mod comms;

pub fn handle(matches: &ArgMatches) {
    if matches.is_present("list-devices") {
        for port in serialport::available_ports().unwrap() {
            info!("{:?}", port);
        }
        
        return;
    }
    
    let mut dev = Device::new(matches.value_of("device").unwrap(), 500000, Duration::from_secs(10)).unwrap();
    dev.clear(ClearBuffer::All);
    
    let tasd = TasdMovie::new(&PathBuf::from(matches.value_of("movie").unwrap())).expect("Failed to parse movie.");
    let console = tasd.search_by_key(vec![KEY_CONSOLE_TYPE]).first().expect("No console type provided in TASD. Cannot continue.").as_any().downcast_ref::<ConsoleType>().unwrap();
    let inputs: Vec<u8> = {
        let chunks: Vec<&[u8]> = tasd.search_by_key(vec![KEY_INPUT_CHUNK]).iter().map(|packet| packet.as_any().downcast_ref::<InputChunk>().unwrap().inputs.as_slice()).collect();
        let mut inputs = vec![];
        for chunk in chunks {
            inputs.extend_from_slice(chunk);
        }
        
        inputs
    };

    {
        let pong = dev.ping();
        if pong != Response::Pong {
            error!("Failed to ping device. {:?}", pong);
            return;
        }
    }

    info!("{}", dev.get_status());
    
    match console.kind.into() {
        System::Nes => {
            let mut ptr = 0usize;
            let mut has_started = false;
            
            println!("Prefilling buffer...");
            while ptr < inputs.len() {
                let input = &inputs[ptr..(ptr + 2)];
                let (res, system, data) = dev.provide_input(System::Nes, input);
                
                if system != System::Nes {
                    warn!("Possible communication error! System doesn't match: (sent) {:?} vs (recv) {:?}", System::Nes, system);
                }
                if data != input {
                    warn!("Possible communication error! Inputs don't match: (sent) {} vs (recv) {}", hex::encode_upper(input), hex::encode_upper(data));
                }
                
                match res {
                    Response::Ok => ptr += 2,
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