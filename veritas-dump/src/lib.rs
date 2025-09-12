use std::sync::Arc;
use base64ct::{Base64, Encoding};
use camino::Utf8PathBuf;
use crossbeam::queue::SegQueue;
use crossbeam::sync::WaitGroup;
use include_dir::{include_dir, Dir};
use tracing::{error, info};
use veritas_emulators::configs::BizHawkConfig;
use veritas_emulators::contexts::EmulatorContext;
use crate::cache::Cache;
use crate::source::Source;
use crate::parser::MovieFormat;

pub mod cache;
pub mod source;
pub mod parser;

static INCLUDES: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/includes/");

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    
    /// Unable to locate or parse a provided movie [source](Source).
    SourceNotFound,
    
    /// Unable to locate a matching ROM file.
    RomNotFound,
    
    /// Unable to locate a valid emulator.
    EmulatorNotFound,
}
impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone)]
struct DumpContext {
    source: Source,
    movie_file: Utf8PathBuf,
    movie_format: MovieFormat,
    rom: Utf8PathBuf,
}
impl DumpContext {
    pub fn dump_path(&self) -> Utf8PathBuf {
        self.movie_file.with_extension("tasd")
    }
}

type PreparedContext = (EmulatorContext, DumpContext);

/// Processes TAS movies into TASD files.
/// 
/// The dumper follows a builder-like pattern for providing movies, as well as configuring
/// the dumping environment.
/// 
/// Calling [`Dumper::dump`] will start the dumping procedure.
pub struct Dumper<'c> {
    hashes: &'c mut Cache,
    cache_root: Utf8PathBuf,
    contexts: Vec<DumpContext>,
    emulators: Vec<EmulatorContext>,
    threads: usize,
}
impl<'c> Dumper<'c> {
    pub fn new<P: Into<Utf8PathBuf>>(cache_root: P, threads: usize, hashes: &'c mut Cache) -> Self {
        let cache_root = cache_root.into();
        let cache_root = cache_root.canonicalize_utf8().unwrap_or(cache_root);
        let includes_path = cache_root.join("includes/");
        
        std::fs::create_dir_all(cache_root.join("movies/")).unwrap();
        std::fs::create_dir_all(&includes_path).unwrap();
        
        for entry in INCLUDES.find("**/*").unwrap() {
            let Some(entry) = entry.as_file() else { continue };
            let Some(name) = entry.path().to_str() else { continue };
            
            let file_path = includes_path.join(name);
            std::fs::write(file_path, entry.contents()).unwrap();
        }
        
        Self {
            cache_root,
            hashes,
            contexts: Vec::new(),
            emulators: Vec::new(),
            threads,
        }
    }
    
    pub fn movie(&mut self, source: &Source, rom_override: Option<Utf8PathBuf>) -> Result<(), ConfigError> {
        let Some((movie_data, filename)) = source.read() else {
            return Err(ConfigError::SourceNotFound)
        };
        
        let movie_file = self.cache_root.join("movies/").join(&filename);
        
        if let Source::Local(local) = source {
            std::fs::copy(local, &movie_file)?;
        } else {
            std::fs::write(&movie_file, &movie_data)?;
        }
        
        let (_, ext) = filename.rsplit_once('.').unwrap_or((&filename, ""));
        let Some(movie_format) = MovieFormat::parse(&movie_data, ext) else {
            return Err(ConfigError::SourceNotFound)
        };
        
        let rom = if let Some(rom_override) = rom_override {
            rom_override
        } else {
            self.find_rom(source, &movie_format)?
        };
        
        if !rom.is_file() {
            return Err(ConfigError::RomNotFound);
        }
        
        self.contexts.push(DumpContext {
            source: source.clone(),
            movie_file,
            movie_format,
            rom,
        });
        
        Ok(())
    }
    
    pub fn emulator(&mut self, context: EmulatorContext) {
        self.emulators.push(context);
    }
    
    /// Begins to dump all [queued][Dumper::movie] movies.
    /// 
    /// Returns a list of tuples that contain each dumped source and the corresponding dump file.
    /// 
    /// The movie queue will be empty after calling this method. The Dumper instance can be reused
    /// without re-adding emulator or other configuration data.
    /// 
    /// # Notes
    /// * Some versions of BizHawk may produce popup "errors" (e.g. wrong/missing cycle count) that must be cleared manually to proceed.
    /// * There is no guarantee the dumped inputs accurately reflect the movie's intent. As in, if a movie doesn't sync correctly, the dump process has no way to know that.
    pub fn dump(&mut self) -> Vec<(Source, Utf8PathBuf)> {
        // 1. determine which emulator and emu version is needed for movie
        // 2. check if emulator is available in `self`
        // 3. create emu-runner context
        // 4. spawn/send-to thread to perform the dump
        // 5. return successful dumps
        
        // *1/*2. if emulator version checking is unsupported, default to first matching emulator
        
        let dumps = Arc::new(SegQueue::new());
        let queue = Arc::new(self.prepare_contexts());
        
        if queue.is_empty() {
            return vec![];
        }
        
        let wg = WaitGroup::new();
        for i in 0..self.threads.clamp(1, queue.len()) {
            let wg = wg.clone();
            let dumps = dumps.clone();
            let queue = queue.clone();
            std::thread::Builder::new().name(format!("dumper_{i}")).spawn(move || {
                while let Some((mut emu, dump)) = queue.pop() {
                    match emu.run() {
                        Ok(_) => {
                            let tasd = dump.dump_path();
                            let DumpContext { source: src, rom, .. } = dump;
                            
                            if tasd.exists() {
                                info!("Dumped: {src} | TASD: {tasd} | ROM: {rom}");
                                dumps.push((src, tasd))
                            } else {
                                error!("Dump failed: {src}")
                            }
                        },
                        Err(err) => error!("{}: {err:?}", dump.source)
                    }
                }
                
                drop(wg);
            }).expect("should have spawned a thread");
        }
        wg.wait();
        
        Arc::into_inner(dumps).expect("arc should only have one reference").into_iter().collect()
    }
    
    fn prepare_contexts(&mut self) -> SegQueue<PreparedContext> {
        let includes = self.cache_root.join("includes/");
        let prepared = SegQueue::new();
        
        let mut contexts = vec![];
        std::mem::swap(&mut contexts, &mut self.contexts);
        
        for ctx in contexts {
            match &ctx.movie_format {
                MovieFormat::Bk2(bk2) => {
                    let required_ver = bk2.emu_version.as_ref().map(|v| v.trim_start_matches("Version ").to_string());
                    
                    let emu = self.emulators.iter()
                        .filter_map(|emu| emu.as_bizhawk())
                        .find(|emu| required_ver.as_ref().is_some_and(|required_ver| required_ver == emu.version()))
                        .cloned();
                    
                    /*if emu.is_none() {
                        emu = self.emulators.iter()
                            .find_map(|emu| emu.as_bizhawk())
                            .cloned();
                    }*/
                    
                    let Some(emu) = emu else {
                        let emu_ver = if let Some(ver) = required_ver {
                            format!("BizHawk v{}", ver.as_str())
                        } else {
                            format!("BizHawk")
                        };
                        error!("Failed to locate compatible emulator ({emu_ver}) for {}", ctx.source);
                        
                        continue
                    };
                    
                    let config = BizHawkConfig::veritas(emu.version());
                    let config_path = ctx.movie_file.with_extension("ini");
                    std::fs::write(&config_path, config.to_vec()).unwrap();
                    
                    prepared.push((
                        EmulatorContext::BizHawk(
                            emu.with_rom(&ctx.rom)
                                .with_movie(&ctx.movie_file)
                                .with_config(config_path)
                                .with_lua(includes.join("tasd-bizhawk.lua"))
                        ),
                        ctx
                    ));
                },
                MovieFormat::Fm2(_fm2) => {
                    let required_ver = 'find_ver: {
                        match ctx.source {
                            Source::Publication(_) => {
                                if let Some(p) = ctx.source.publication_metadata() && let Some(emu_ver) = p.emulator_version && emu_ver.contains('.') {
                                    let emu_ver = emu_ver.trim();
                                    
                                    break 'find_ver emu_ver.split_once(' ').unwrap_or(("", emu_ver)).1.trim().to_string();
                                }
                            },
                            Source::Submission(_) => {
                                if let Some(s) = ctx.source.submission_metadata() && let Some(emu_ver) = s.emulator_version && emu_ver.contains('.') {
                                    let emu_ver = emu_ver.trim();
                                    
                                    break 'find_ver emu_ver.split_once(' ').unwrap_or(("", emu_ver)).1.trim().to_string();
                                }
                            },
                            _ => ()
                        }
                        
                        _fm2.version_string()
                    };
                    
                    let mut emu = self.emulators.iter()
                        .filter_map(|emu| emu.as_fceux())
                        .find(|emu| emu.version == required_ver)
                        .cloned();
                    
                    if emu.is_none() {
                        emu = self.emulators.iter()
                            .find_map(|emu| emu.as_fceux())
                            .cloned();
                    }
                    
                    let Some(emu) = emu else {
                        error!("Failed to locate compatible emulator (FCEUX v{required_ver}) for {}", ctx.source);
                        
                        continue
                    };
                    
                    prepared.push((
                        EmulatorContext::Fceux(
                            emu.with_rom(&ctx.rom)
                                .with_movie(&ctx.movie_file)
                                //.with_config(config_path) //TODO: Write FCEUX config generator
                                .with_lua(includes.join("tasd-fceux.lua"))
                        ),
                        ctx
                    ));
                },
                MovieFormat::Gmv(_gmv) => {
                    todo!()
                },
            }
        }
        
        prepared
    }
    
    fn find_rom(&self, source: &Source, movie: &MovieFormat) -> Result<Utf8PathBuf, ConfigError> {
        // if source is Pub or Sub:
        //     check catalog first
        // if catalog is missing OR unable to locate rom using catalog:
        //     check parsed movie file
        
        'search_catalog: {
            let (gid, vid) = match source {
                Source::Publication(id) => {
                    let Ok(p) = tasvideos_api_rs::get_publication(*id) else { break 'search_catalog };
                    
                    let Some(gid) = p.game_id else { break 'search_catalog };
                    let Some(vid) = p.game_version_id else { break 'search_catalog };
                    
                    (gid, vid)
                },
                Source::Submission(id) => {
                    let Ok(s) = tasvideos_api_rs::get_submission(*id) else { break 'search_catalog };
                    
                    let Some(gid) = s.game_id else { break 'search_catalog };
                    let Some(vid) = s.game_version_id else { break 'search_catalog };
                    
                    (gid, vid)
                },
                _ => break 'search_catalog,
            };
            
            let Ok(game) = tasvideos_api_rs::get_game(gid) else { break 'search_catalog };
            let Some(versions) = game.versions else { break 'search_catalog };
            let Some(ver) = versions.into_iter().find(|v| v.id.is_some_and(|id| id == vid)) else { break 'search_catalog };
            
            let sha1 = ver.sha1.and_then(|hash| hex::decode(hash).ok());
            if let Some(sha1) = sha1 {
                let paths = self.hashes.find_paths(|bundle| bundle.sha1.as_slice() == &sha1);
                if let Some(path) = paths.into_iter().next() {
                    return Ok(path.into());
                }
            }
            
            let md5 = ver.md5.and_then(|hash| hex::decode(hash).ok());
            if let Some(md5) = md5 {
                let paths = self.hashes.find_paths(|bundle| bundle.md5.as_slice() == &md5);
                if let Some(path) = paths.into_iter().next() {
                    return Ok(path.into());
                }
            }
        }
        
        'search_movie: {
            let hash = match movie {
                MovieFormat::Bk2(bk2) => {
                    let Some(hash) = &bk2.checksum else { break 'search_movie };
                    let Ok(hash) = hex::decode(hash) else { break 'search_movie };
                    
                    hash
                },
                MovieFormat::Fm2(fm2) => {
                    let (_, rom_checksum) = fm2.rom_checksum.split_once(':').unwrap_or(("", &fm2.rom_checksum));
                    let Ok(hash) = Base64::decode_vec(rom_checksum) else { break 'search_movie };
                    
                    // Despite what FCEUX's docs say, the checksum is NOT encoded using the hexadecimal/hexified representation of the MD5 hash
                    // So we do not have to convert to/from hex here
                    
                    hash
                },
                MovieFormat::Gmv(_) => break 'search_movie, // GMV doesn't store ROM details *facepalm*
            };
            
            let paths = self.hashes.find_paths(|bundle| bundle.sha1.as_slice() == &hash || bundle.md5.as_slice() == &hash);
            if let Some(path) = paths.into_iter().next() {
                return Ok(path.into());
            }
        }
        
        Err(ConfigError::RomNotFound)
    }
}
