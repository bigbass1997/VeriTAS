use camino::Utf8PathBuf;
use tracing::{error, info};
use veritas_dump::cache::Cache;
use veritas_dump::Dumper;
use veritas_dump::source::Source;
use veritas_emulators::contexts::{BizHawkContext, FceuxContext};
use crate::cli::DumpArgs;

pub fn handle(args: DumpArgs) {
    let cache_root = args.cache.unwrap_or("./cache/".into());
    let hashes_path = cache_root.join("hashes.bin");
    
    let mut hashes = if let Ok(data) = std::fs::read(&hashes_path) {
        info!("Loading existing hash cache...");
        Cache::decode(&data).unwrap()
    } else {
        info!("Creating new hash cache...");
        Cache::new()
    };
    
    hashes.refresh(args.refresh);
    std::fs::write(&hashes_path, hashes.encode()).unwrap();
    
    let mut dumper = Dumper::new(&cache_root, args.threads, &mut hashes);
    for fetch in args.fetch {
        let (src, over) = fetch.split_once('=').unwrap_or_else(|| (&fetch, "")); // will break if src path contains `=`
        
        let Some(src) = Source::parse(src) else {
            error!("Failed to parse movie source: {src}");
            continue
        };
        
        let over = if over.is_empty() {
            None
        } else {
            let over = Utf8PathBuf::from(over);
            if !over.exists() {
                error!("ROM override for {src} doesn't exist: {over}");
                continue
            }
            if over.is_dir() {
                error!("ROM override for {src} is not a file: {over}");
                continue
            }
            
            Some(over)
        };
        
        if let Err(err) = dumper.movie(&src, over) {
            error!("Failed to queue {src}: {err:?}");
        }
    }
    
    //TODO read these from config
    dumper.emulator(BizHawkContext::new("/opt/emulators/bizhawk-2.10-linux/", "2.10").unwrap().into());
    dumper.emulator(FceuxContext::new("/opt/emulators/fceux-2.6.6-win64/", "2.6.6").unwrap().into());
    
    dumper.dump();
}