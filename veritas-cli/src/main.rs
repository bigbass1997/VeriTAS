//! 
//! 
//!
use veritas_dump::cache::Cache;
use clap::Parser;
use crate::cli::{Args, Command, EncodeArgs, ReplayArgs};

pub mod cli;
pub mod dump;

fn main() {
    let args: Args = Args::parse();
    
    let trace = tracing_subscriber::fmt();
    if let Some(level) = args.verbose {
        trace.with_max_level(level).init();
    } else {
        trace.init();
    }
    
    match args.command {
        Command::Dump(args) => dump::handle(args),
        Command::Replay(args) => handle_replay(args),
        Command::Encode(args) => handle_encode(args),
    }
    
    return;
    {
    let mut cache = if let Ok(data) = std::fs::read("testcache2.bin") {
        println!("loading existing cache");
        Cache::decode(&data).unwrap()
    } else {
        Cache::new()
    };
    
    //cache.refresh(Some("/data/storage/roms/veritas-roms/"));
    //std::fs::write("testcache2.bin", cache.encode()).unwrap();
    
    cache.refresh(None);
    
    //let mut dumper = Dumper::new("cache", &mut cache);
    //dumper.movie(src, None).unwrap();
    /*dumper.emulator("/opt/emulators/bizhawk-master/output/").unwrap();
    dumper.emulator("/opt/emulators/bizhawk-2.9.1-linux/").unwrap();
    dumper.emulator("/opt/emulators/fceux-2.5.0-win64/bin/").unwrap();
    dumper.emulator("/opt/emulators/fceux-2.6.6-win64/").unwrap();
    
    for (source, dump) in dumper.dump() {
        println!("{source}: {dump}");
    }
    */
    /*let mut hash = [0u8; 20];
    hex::decode_to_slice("2E8A5923516FDBD110DD328661072E8D3C026601", &mut hash).unwrap();*/
    
    
    /*
    for hash in cache.find_hashes("/data/storage/roms/nes-nointro/Code Name - Viper (USA).nes") {
        for path in cache.find_paths(|h| h.sha1 == hash.sha1) {
            println!("{path}");
        }
        //println!("{:02X?}", hash.sha1);
    }
    for hash in cache.find_hashes("/data/storage/roms/nes-nointro-ines/Code Name - Viper (USA).nes") {
        for path in cache.find_paths(|h| h.sha1 == hash.sha1) {
            println!("{path}");
        }
        //println!("{:02X?}", hash.sha1);
    }
    for hash in cache.find_hashes("/data/storage/roms/nes-gr8fam/Code Name - Viper (U) [!] (2).nes") {
        println!("{:02X?}", hash.sha1);
    }*/
    
    /*
    //let (data, filename) = Source::Submission(9145).read().unwrap();
    let (data, filename) = Source::Submission(9186).read().unwrap();
    std::fs::write("/tmp/9186S.bk2", &data).unwrap();
    
    let format = MovieFormat::parse(&data, filename.split_once('.').unwrap().1).unwrap();
    match format {
        MovieFormat::Bk2(bk2) => {
            let hashstr = bk2.checksum.unwrap();
            let mut hash = [0u8; 20];
            hex::decode_to_slice(&hashstr, &mut hash).unwrap();
            
            println!("{:#?}", cache.find_paths(|h| h.sha1 == hash));
            println!("SHA1: {hashstr}");
        },
        _ => ()
    }
    //E2814B00276B30058070C2149FD837C0FFE046CC
    
    for bundle in cache.find_hashes("/data/storage/roms/nes-nointro/Tetris 2 + Bombliss (Japan) (Rev A).nes") {
        println!("MD5: {}, SHA1: {}", hex::encode_upper(bundle.md5), hex::encode_upper(bundle.sha1));
    }
    println!();
    for bundle in cache.find_hashes("/data/storage/roms/nes-gr8fam/Tetris 2 + BomBliss (J) [a1].nes") {
        println!("MD5: {}, SHA1: {}", hex::encode_upper(bundle.md5), hex::encode_upper(bundle.sha1));
    }
    
    println!();
    
    {
        let mut hash = [0u8; 20];
        hex::decode_to_slice("F2827DC07D863CD111C5876504A35EA33811F511", &mut hash).unwrap();
        println!("{:#?}", cache.find_paths(|h| h.sha1 == hash));
    }
    
    */
    }
}

fn handle_replay(args: ReplayArgs) {
    
}

fn handle_encode(args: EncodeArgs) {
    
}
