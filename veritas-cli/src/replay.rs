use std::time::Duration;
use camino::Utf8PathBuf;
use veritas_dump::cache::HashBundle;
use veritas_replay::n8sim::fs::{FsMapper, FsNode};
use veritas_replay::n8sim::{Input, N8Simulator};
use veritas_replay::{ReplayDevice, ReplayInterface};
use veritas_replay::veritas::{Command, Message, System, VeritasMode};
use crate::cli::ReplayArgs;

pub fn handle_replay(cache_root: Utf8PathBuf, _args: ReplayArgs) {
    let n8map = FsMapper::load(cache_root.join("n8map.bin")).unwrap_or_else(|_| FsMapper::new());
    //n8map.pretty_print();
    let bundle = HashBundle::from_path("/data/storage/roms/nes-nointro/Super Mario Bros. 3 (USA) (Rev A).nes").unwrap().into_iter().next().unwrap();
    let (path, _) = n8map.find(|node| if let FsNode::File { hashes, .. } = node { hashes.contains(&bundle) } else { false }).unwrap();
    
    println!("{path}");
    
    let mut dev = ReplayDevice::open_name("/dev/ttyACM0").unwrap();
    let dev = dev.as_veritas().unwrap();
    
    dev.clear_usb_buffers();
    
    println!("{:?}", dev.ping());
    
    dev.send(Command::GetStatus).unwrap();
    println!("{:#?}", dev.recv().unwrap());
    
    dev.send(Command::UseInitialReset(false)).unwrap();
    dev.recv().unwrap();
    dev.send(Command::SetReplayMode(VeritasMode::ReplayNes)).unwrap();
    dev.recv().unwrap();
    
    
    N8Simulator::new(&n8map, |inputs| {
        let inputs: Vec<u8> = inputs
            .flat_map(|p1| [p1.0, Input::new().0])
            .collect();
        
        dev.send(Command::ProvideInput(System::Nes, inputs)).unwrap();
        println!("{:?}", dev.recv().unwrap());
        
        loop {
            std::thread::sleep(Duration::from_millis(30));
            dev.send(Command::ProvideInput(System::Nes, vec![])).unwrap();
            if let Ok(Message::BufferStatus { remaining_space, .. }) = dev.recv() && remaining_space < 16384 {
                //println!("remaining: {}", 16384 - remaining_space);
            } else {
                break;
            }
        }
    })
        //.down(1);
        .to_path("/EDFC/SAVE/0_clear-sram.nes")
        .b(1).flush(50)
        .down(1).b(1).flush(1000)
        .a(1).flush(1000)
        .a(1).flush(1000)
        .to_path(path) // /VeriTAS/_000073/Kung Fu (PC10).nes
        .b(1).flush(100);
        //.b(1).flush(100);
        //.down(2).b(1).flush(500); // /VeriTAS/_000007/Ninja Gaiden III - The Ancient Ship of Doom (USA).nes
    
    
    
    
    
    dev.send(Command::SetReplayMode(VeritasMode::Idle)).unwrap();
    
    //println!("included: {}", n8map.include_files("/media/bigbass/EVERDRIVE/"));
    //println!("registered: {}", n8map.register_files("/data/storage/roms/nes-nointro-ines/"));
    //println!("registered: {}", n8map.register_files("/data/storage/roms/nes-homebrew/"));
    //println!("registered: {}", n8map.register_files("/data/storage/roms/nes-hacks/"));
    //println!("registered: {}", n8map.register_files("/data/storage/roms/nes-intellivision/"));
    //println!("registered: {}", n8map.register_files("/data/storage/roms/fds-champion/"));
    //println!("registered: {}", n8map.register_files("/data/storage/roms/nes-nointro/"));
    //println!("registered: {}", n8map.register_files("/data/storage/roms/output/"));
    //println!("registered: {}", n8map.register_files("/data/storage/roms/nes-gr8fam/"));
    //n8map.pretty_print();
    
    //n8map.copy_to("/media/bigbass/EVERDRIVE/");
    //n8map.copy_to("./cache/output/");
    //n8map.save("n8map.bin").unwrap();
    
    /*let search = ["Ninja Gaiden III - The Ancient Ship of Doom (USA).nes"];
    for s in search {
        if let Some((path, _node)) = n8map.find(|node| node.name() == s) {
            println!("{s}: {path}");
        } else {
            println!("{s} not found")
        }
    }*/
    
}