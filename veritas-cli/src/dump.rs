use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::ser::PrettyFormatter;
use serde_json::Serializer;
use tracing::{error, info, warn};
use veritas_dump::cache::Cache;
use veritas_dump::Dumper;
use veritas_dump::source::Source;
use veritas_emulators::contexts::{BizHawkContext, EmulatorContext, FceuxContext, GensContext};
use crate::cli::DumpArgs;


#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct EmulatorEntry {
    path: Utf8PathBuf,
    version: String,
}
impl EmulatorEntry {
    pub fn new<P: Into<Utf8PathBuf>, S: Into<String>>(path: P, version: S) -> Self {
        Self {
            path: path.into(),
            version: version.into(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DumpConfig {
    #[serde(skip)]
    path: Utf8PathBuf,
    
    bizhawk: Vec<EmulatorEntry>,
    fceux: Vec<EmulatorEntry>,
    gens: Vec<EmulatorEntry>,
}
impl DumpConfig {
    pub fn load(path: impl Into<Utf8PathBuf>) -> Result<Self, String> {
        let path = path.into();
        
        if !path.is_file() {
            let cfg = Self { path, ..Default::default() };
            cfg.save().map_err(|e| e.to_string())?;
            
            return Ok(cfg);
        }
        
        
        let data = std::fs::read(&path).map_err(|e| e.to_string())?;
        
        let mut cfg: DumpConfig = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        cfg.path = path;
        
        Ok(cfg)
    }
    
    pub fn save(&self) -> Result<(), std::io::Error> {
        let mut data = Vec::with_capacity(128);
        let mut ser = Serializer::with_formatter(&mut data, PrettyFormatter::with_indent(b"    "));
        
        self.serialize(&mut ser).expect("should be serializable to JSON");
        
        std::fs::write(&self.path, data)
    }
}


pub fn handle(cache_root: Utf8PathBuf, args: DumpArgs) {
    let hashes_path = cache_root.join("hashes.bin");
    
    let mut cfg = DumpConfig::load(cache_root.join("dump_config.json")).expect("expected valid JSON file");
    
    for (path, ver) in args.emulator.iter().filter_map(|pair| pair.rsplit_once(',')).map(|(path, ver)| (Utf8Path::new(path), ver)) {
        if !path.exists() || ver.is_empty() {
            continue
        }
        
        if BizHawkContext::new(path, ver).is_ok() {
            let entry = EmulatorEntry::new(path, ver);
            if !cfg.bizhawk.contains(&entry) {
                cfg.bizhawk.push(entry);
                info!("Added new BizHawk emulator from {path}");
            } else {
                info!("Emulator already registered: {path}");
            }
            continue
        }
        
        if FceuxContext::new(path, ver).is_ok() {
            let entry = EmulatorEntry::new(path, ver);
            if !cfg.fceux.contains(&entry) {
                cfg.fceux.push(entry);
                info!("Added new FCEUX emulator from {path}");
            } else {
                info!("Emulator already registered: {path}");
            }
            continue
        }
        
        if let Ok(ver) = ver.parse() && GensContext::new(path, ver).is_ok() {
            let entry = EmulatorEntry::new(path, ver.to_string());
            if !cfg.gens.contains(&entry) {
                cfg.gens.push(entry);
                info!("Added new Gens emulator from {path}");
            } else {
                info!("Emulator already registered: {path}");
            }
            continue
        }
        
        warn!("Couldn't identify any compatible emulator at {path}");
    }
    if args.emulator.len() > 0 {
        cfg.save().unwrap();
    }
    
    
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
    
    let bizhawk = cfg.bizhawk.into_iter()
        .filter_map(|EmulatorEntry { path, version }| BizHawkContext::new(path, version).ok())
        .map(|ctx| EmulatorContext::from(ctx));
    let fceux = cfg.fceux.into_iter()
        .filter_map(|EmulatorEntry { path, version }| FceuxContext::new(path, version).ok())
        .map(|ctx| EmulatorContext::from(ctx));
    let gens = cfg.gens.into_iter()
        .filter_map(|EmulatorEntry { path, version }| version.parse().map(|v| (path, v)).ok())
        .filter_map(|(path, ver)| GensContext::new(path, ver).ok())
        .map(|ctx| EmulatorContext::from(ctx));
    
    let emus = bizhawk.chain(fceux).chain(gens);
    for emu in emus {
        dumper.emulator(emu);
    }
    
    dumper.dump();
}