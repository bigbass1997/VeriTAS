use std::cmp::{max, min};
use std::io::stdout;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crossterm::{event, terminal};
use crossterm::event::{Event, KeyCode};
use log::{debug, error, info, warn};
use serialport::{ClearBuffer, SerialPortType};
use tasd::spec::{ConsoleType, InputChunk, KEY_CONSOLE_TYPE, KEY_INPUT_CHUNK, KEY_TRANSITION, TasdMovie, Transition};
use crate::replay::comms::{Command, Device, Response, System, TransitionData, VeritasMode};
use crate::replay::comms::Command::{GetStatus, ProvideInput, ProvideTransitions, SetLatchFilter, SetReplayLength, SetReplayMode};
use crate::ReplayArgs;

mod comms;

pub fn handle(args: ReplayArgs) {
    if args.list_devices {
        for port in serialport::available_ports().unwrap() {
            info!("{:?}", port);
        }
        
        return;
    }
    
    let device_path = args.device.unwrap_or_else(|| {
        serialport::available_ports().unwrap()
            .into_iter()
            .filter_map(|info| if let SerialPortType::UsbPort(usbport) = info.port_type { Some((info.port_name, usbport)) } else { None })
            .find(|(_, port)| port.serial_number == Some("VeriTAS".into()))
            .map(|(name, _)| name)
            .unwrap()
    });
    let mut dev = Device::new(device_path, 500000, Duration::from_secs(6)).unwrap();
    dev.clear(ClearBuffer::All);
    
    if dev.send_command(Command::Ping) != Response::Pong {
        panic!("Failed to ping device.");
    }
    
    if args.disable_reset && dev.send_command(Command::UseInitialReset(false)).is_not_ok() {
        warn!("Failed to disable initial reset");
    }
    
    if let Some(manual) = args.manual {
        let _stdout = stdout();
        
        match manual.to_lowercase().as_str() {
            "nes" => {
                if dev.send_command(SetReplayMode(VeritasMode::ReplayNes)).is_not_ok() {
                    panic!("Failed to set replay mode");
                }
                
                if dev.send_command(SetLatchFilter(args.latch_filter.unwrap_or(8000))).is_not_ok() {
                    panic!("Failed to set latch filter");
                }
                
                terminal::enable_raw_mode().unwrap();
                
                loop {
                    let mut input = None;
                    match event::read() {
                        Ok(event) => match event {
                            Event::Key(event) => match event.code {
                                KeyCode::Char('q') => break,
                                
                                KeyCode::Char('z') => { input = Some([0x7F, 0xFF]); },
                                KeyCode::Char('x') => { input = Some([0xBF, 0xFF]); },
                                KeyCode::Char(' ') => { input = Some([0xDF, 0xFF]); },
                                KeyCode::Enter => { input = Some([0xEF, 0xFF]); },
                                KeyCode::Up => { input = Some([0xF7, 0xFF]); },
                                KeyCode::Down => { input = Some([0xFB, 0xFF]); },
                                KeyCode::Left => { input = Some([0xFD, 0xFF]); },
                                KeyCode::Right => { input = Some([0xFE, 0xFF]); },
                                _ => ()
                            }
                            _ => ()
                        }
                        Err(_) => ()
                    }
                    
                    if let Some(input) = input {
                        let resp = dev.send_command(Command::ProvideInput(System::Nes, input.to_vec()));
                        match resp {
                            Response::BufferStatus { written, .. } if written == 2 => (),
                            Response::BufferStatus { written, .. } => {
                                warn!("Entire input not written {written} vs {}", input.len());
                            },
                            _ => {
                                warn!("Failed to provide input: {resp:?}");
                            }
                        }
                    }
                }
                
                terminal::disable_raw_mode().unwrap();
                
                if dev.send_command(Command::SetReplayMode(VeritasMode::Idle)).is_not_ok() {
                    panic!("Failed to set replay mode");
                }
                println!("");
            },
            "gen" | "genesis" | "md" | "megadrive" => {
                if dev.send_command(SetReplayMode(VeritasMode::ReplayGenesis)).is_not_ok() {
                    panic!("Failed to set replay mode");
                }
                
                terminal::enable_raw_mode().unwrap();
                
                loop {
                    let mut input: Option<&'static [u8]> = None;
                    match event::read() {
                        Ok(event) => match event {
                            Event::Key(event) => match event.code {
                                KeyCode::Char('q') => break,
                                
                                KeyCode::Char('z') => { input = Some(&[0x7F, 0xFF, 0xFF, 0xFF]); }, // A
                                KeyCode::Char('x') => { input = Some(&[0xFD, 0xFF, 0xFF, 0xFF]); }, // B
                                KeyCode::Char('c') => { input = Some(&[0xFE, 0xFF, 0xFF, 0xFF]); }, // C
                                KeyCode::Enter => { input = Some(&[0xBF, 0xFF, 0xFF, 0xFF]); }, // START
                                KeyCode::Up => { input = Some(&[0xDF, 0xFF, 0xFF, 0xFF]); },
                                KeyCode::Down => { input = Some(&[0xEF, 0xFF, 0xFF, 0xFF]); },
                                KeyCode::Left => { input = Some(&[0xF7, 0xFF, 0xFF, 0xFF]); },
                                KeyCode::Right => { input = Some(&[0xFB, 0xFF, 0xFF, 0xFF]); },
                                _ => ()
                            }
                            _ => ()
                        }
                        Err(_) => ()
                    }
                    
                    if let Some(input) = input {
                        let resp = dev.send_command(Command::ProvideInput(System::Genesis, input.to_vec()));
                        match resp {
                            Response::BufferStatus { written, .. } if written == 4 || written == 8 => (),
                            Response::BufferStatus { written, .. } => {
                                warn!("Entire input not written {written} vs {}", input.len());
                            },
                            _ => {
                                warn!("Failed to provide input: {resp:?}");
                            }
                        }
                    }
                }
                
                terminal::disable_raw_mode().unwrap();
                
                if dev.send_command(Command::SetReplayMode(VeritasMode::Idle)).is_not_ok() {
                    panic!("Failed to set replay mode");
                }
                println!("");
            },
            _ => warn!("unrecognized console")
        }
        
        return;
    }
    
    let tasd = TasdMovie::new(&PathBuf::from(args.movie.unwrap())).expect("Failed to parse movie.");
    let console = tasd.search_by_key(vec![KEY_CONSOLE_TYPE]).first().expect("No console type provided in TASD. Cannot continue.").as_any().downcast_ref::<ConsoleType>().unwrap();
    let mut transitions: Vec<Transition> = tasd.search_by_key(vec![KEY_TRANSITION]).into_iter().map(|packet| packet.as_any().downcast_ref::<Transition>().unwrap().clone()).collect();
    for trans in &mut transitions {
        trans.index /= 2;
        
        info!("{trans}");
    }
    let inputs: Vec<u8> = {
        let chunks: Vec<&[u8]> = tasd.search_by_key(vec![KEY_INPUT_CHUNK]).iter().map(|packet| packet.as_any().downcast_ref::<InputChunk>().unwrap().inputs.as_slice()).collect();
        let mut inputs = vec![];
        for chunk in chunks {
            inputs.extend_from_slice(chunk);
        }
        
        inputs
    };
    
    let ports: [Vec<u8>; 2] = {
        let chunks: Vec<InputChunk> = tasd.search_by_key(vec![KEY_INPUT_CHUNK]).into_iter().map(|packet| packet.as_any().downcast_ref::<InputChunk>().unwrap().clone()).collect();
        let mut ports = std::array::from_fn(|_| vec![]);
        
        for i in (0..chunks.len()).step_by(1) {
            let chunk = &chunks[i];
            ports[chunk.port as usize - 1].extend_from_slice(&chunk.inputs);
        }
        
        ports
    };
    
    let exit_early = Arc::new(AtomicBool::new(false));
    let exit = exit_early.clone();
    ctrlc::set_handler(move || {
        exit.store(true, Ordering::Relaxed);
    }).expect("Failed to set CTRL+C handler");
    
    match console.kind.into() {
        System::Nes => {
            dev.send_command(SetLatchFilter(args.latch_filter.unwrap_or(8000)));
            dev.send_command(SetReplayLength((inputs.len() / 2) as u64));
            dev.send_command(ProvideTransitions(TransitionData::from_vec(transitions)));
            
            if let Response::DeviceStatus(text) = dev.send_command(GetStatus) {
                info!("{text}");
            } else {
                warn!("Failed to receive device status");
            }
            
            let mut ptr = 0usize;
            let mut has_started = false;
            let mut prev_empty = 2;
            
            info!("Prefilling buffer...");
            while ptr < inputs.len() {
                if exit_early.load(Ordering::Relaxed) {
                    if dev.send_command(SetReplayMode(VeritasMode::Idle)).is_not_ok() {
                        error!("Failed to set replay mode!");
                    }
                    info!("Exiting..");
                    break;
                }
                
                let remaining = inputs.len() - ptr;
                let input = &inputs[ptr..(ptr + max(2, min(prev_empty, min(16, remaining))))];
                
                if let Response::BufferStatus { written, remaining_space } = dev.send_command(ProvideInput(System::Nes, input.to_vec())) {
                    ptr += written as usize;
                    prev_empty = remaining_space as usize;
                    debug!("written: {written}, remaining_space: {remaining_space}");
                    
                    if remaining_space == 0 && !has_started {
                        has_started = true;
                        
                        if dev.send_command(SetReplayMode(VeritasMode::ReplayNes)).is_not_ok() {
                            error!("Failed to set replay mode!");
                            return;
                        }
                        info!("Starting replay.")
                    } else if remaining_space < 128 && has_started {
                        std::thread::sleep(Duration::from_millis(2000));
                    }
                } else {
                    error!("Failed to receive buffer status! desync likely!");
                }
            }
            
            if !has_started {
                if dev.send_command(SetReplayMode(VeritasMode::ReplayNes)).is_not_ok() {
                    error!("Failed to set replay mode!");
                    return;
                }
                info!("Starting replay.")
            }
        },
        System::N64 => {
            todo!()
        },
        System::Genesis => {
            let mut inputs = vec![];
            /*for i in (0..ports[0].len()).step_by(2) {
                inputs.extend_from_slice(&[ports[0][i], ports[0][i + 1], ports[1][i], ports[1][i + 1]]);
            }*/
            for i in 0..ports[0].len() {
                inputs.extend_from_slice(&[ports[0][i], 0xFF, *ports[1].get(i).unwrap_or(&0xFF), 0xFF]);
                //println!("{:02X?}", &inputs[(inputs.len() - 4)..]);
            }
            //return;
            
            dev.send_command(SetReplayLength((inputs.len() / 4) as u64));
            //TODO dev.send_command(ProvideTransitions(TransitionData::from_vec(transitions)));
            
            if let Response::DeviceStatus(text) = dev.send_command(GetStatus) {
                info!("{text}");
            } else {
                warn!("Failed to receive device status");
            }
            
            let mut ptr = 0usize;
            let mut has_started = false;
            let mut prev_empty = 4;
            
            info!("Prefilling buffer...");
            while ptr < inputs.len() {
                if exit_early.load(Ordering::Relaxed) {
                    if dev.send_command(SetReplayMode(VeritasMode::Idle)).is_not_ok() {
                        error!("Failed to set replay mode!");
                    }
                    info!("Exiting..");
                    break;
                }
                
                let remaining = inputs.len() - ptr;
                let input = inputs[ptr..(ptr + max(4, min(prev_empty, min(16, remaining))))].to_vec();
                
                if let Response::BufferStatus { written, remaining_space } = dev.send_command(ProvideInput(System::Genesis, input)) {
                    ptr += written as usize;
                    prev_empty = remaining_space as usize;
                    debug!("written: {written}, remaining_space: {remaining_space}");
                    
                    if remaining_space == 0 && !has_started {
                        has_started = true;
                        
                        if dev.send_command(SetReplayMode(VeritasMode::ReplayGenesis)).is_not_ok() {
                            error!("Failed to set replay mode!");
                            return;
                        }
                        info!("Starting replay.")
                    } else if remaining_space < 128 && has_started {
                        std::thread::sleep(Duration::from_millis(2000));
                    }
                } else {
                    error!("Failed to receive buffer status! desync likely!");
                }
            }
            
            if !has_started {
                if dev.send_command(SetReplayMode(VeritasMode::ReplayGenesis)).is_not_ok() {
                    error!("Failed to set replay mode!");
                    return;
                }
                info!("Starting replay.")
            }
        },
        System::A2600 => {
            todo!()
        },
        _ => unimplemented!()
    }
}