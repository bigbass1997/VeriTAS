use std::collections::VecDeque;
use base64ct::{Base64, Encoding};
use camino::{Utf8Path, Utf8PathBuf};
use emu_runner::contexts::{BizHawkContext, FceuxContext, GensContext};
use emu_runner::{EmulatorContext, EmulatorContextTrait};
use crate::cache::Cache;
use crate::source::Source;
use crate::parser::MovieFormat;

pub mod cache;
pub mod source;
pub mod parser;

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

/// Processes TAS movies into TASD files.
/// 
/// The dumper follows a builder-like pattern for providing movies, as well as configuring
/// the dumping environment.
/// 
/// Calling [`Dumper::dump`] will start the dumping procedure.
pub struct Dumper<'c> {
    cache: &'c mut Cache,
    tmp: Utf8PathBuf,
    contexts: VecDeque<DumpContext>,
    emulators: Vec<EmulatorContext>,
}
impl<'c> Dumper<'c> {
    pub fn new<P: Into<Utf8PathBuf>>(temp_dir: P, cache: &'c mut Cache) -> Self {
        let tmp = temp_dir.into();
        let tmp = tmp.canonicalize_utf8().unwrap_or(tmp);
        std::fs::create_dir_all(tmp.join("movies/")).unwrap();
        std::fs::create_dir_all(tmp.join("includes/")).unwrap();
        Self {
            tmp,
            cache,
            contexts: VecDeque::new(),
            emulators: Vec::new(),
        }
    }
    
    pub fn movie(&mut self, source: Source, rom_override: Option<Utf8PathBuf>) -> Result<(), ConfigError> {
        let Some((movie_data, filename)) = source.read() else {
            return Err(ConfigError::SourceNotFound)
        };
        
        let movie_file = self.tmp.join("movies/").join(&filename);
        
        if let Source::Local(local) = &source {
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
            self.find_rom(&source, &movie_format)?
        };
        
        if !rom.is_file() {
            return Err(ConfigError::RomNotFound);
        }
        
        self.contexts.push_back(DumpContext {
            source,
            movie_file,
            movie_format,
            rom,
        });
        
        Ok(())
    }
    
    pub fn emulator<P: AsRef<Utf8Path>>(&mut self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();
        
        let ctx: EmulatorContext = 'ctx: {
            if let Ok(ctx) = BizHawkContext::new(&path) { break 'ctx ctx.into(); }
            if let Ok(ctx) = FceuxContext::new(&path) { break 'ctx ctx.into(); }
            if let Ok(ctx) = GensContext::new(&path) { break 'ctx ctx.into(); }
            
            return Err(ConfigError::EmulatorNotFound)
        };
        
        self.emulators.push(ctx);
        Ok(())
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
        //TODO: create threadpool and work queues
        
        // 1. determine which emulator and emu version is needed for movie
        // 2. check if emulator is available in `self`
        // 3. create emu-runner context
        // 4. spawn/send-to thread to perform the dump
        // 5. return successful dumps
        
        // *1/*2. if emulator version checking is unsupported, default to first matching emulator
        
        let includes = self.tmp.join("includes/");
        //TODO: Remove these, and instead generate the config on-the-fly for every spawned emulator instance
        std::fs::write(includes.join("2.6.config.ini"), include_bytes!("includes/2.6.config.ini")).unwrap();
        std::fs::write(includes.join("2.6.1.config.ini"), include_bytes!("includes/2.6.1.config.ini")).unwrap();
        std::fs::write(includes.join("2.6.2.config.ini"), include_bytes!("includes/2.6.2.config.ini")).unwrap();
        std::fs::write(includes.join("2.6.3.config.ini"), include_bytes!("includes/2.6.3.config.ini")).unwrap();
        std::fs::write(includes.join("2.7.config.ini"), include_bytes!("includes/2.7.config.ini")).unwrap();
        std::fs::write(includes.join("2.8.config.ini"), include_bytes!("includes/2.8.config.ini")).unwrap();
        std::fs::write(includes.join("2.8-rc1.config.ini"), include_bytes!("includes/2.8-rc1.config.ini")).unwrap();
        std::fs::write(includes.join("2.9.config.ini"), include_bytes!("includes/2.9.config.ini")).unwrap();
        std::fs::write(includes.join("2.9-rc1.config.ini"), include_bytes!("includes/2.9-rc1.config.ini")).unwrap();
        std::fs::write(includes.join("2.9-rc2.config.ini"), include_bytes!("includes/2.9-rc2.config.ini")).unwrap();
        std::fs::write(includes.join("2.9-rc3.config.ini"), include_bytes!("includes/2.9-rc3.config.ini")).unwrap();
        std::fs::write(includes.join("2.9.1.config.ini"), include_bytes!("includes/2.9.1.config.ini")).unwrap();
        std::fs::write(includes.join("tasd-api.lua"), include_bytes!("includes/tasd-api.lua")).unwrap();
        std::fs::write(includes.join("tasd-bizhawk.lua"), include_bytes!("includes/tasd-bizhawk.lua")).unwrap();
        std::fs::write(includes.join("tasd-fceux.lua"), include_bytes!("includes/tasd-fceux.lua")).unwrap();
        std::fs::write(includes.join("tasd-gens.lua"), include_bytes!("includes/tasd-gens.lua")).unwrap();
        
        let mut dumps = vec![];
        
        while let Some(ctx) = self.contexts.pop_front() {
            let dump_path = ctx.movie_file.with_extension("tasd");
            
            match ctx.movie_format {
                MovieFormat::Bk2(bk2) => {
                    let mut emu = self.emulators.iter()
                        .filter_map(|emu| match emu {
                            EmulatorContext::BizHawk(emu) => Some(emu),
                            _ => None,
                        })
                        .filter(|emu| bk2.emu_version.as_ref().is_some_and(|required_ver| emu.detect_version().is_some_and(|emu_ver| required_ver.contains(&emu_ver))))
                        .cloned()
                        .next();
                    
                    if emu.is_none() {
                        emu = self.emulators.iter()
                            .find_map(|emu| match emu {
                                EmulatorContext::BizHawk(emu) => Some(emu),
                                _ => None,
                            })
                            .cloned();
                    }
                    
                    let Some(mut emu) = emu else { continue };
                    
                    if let Some(ver) = emu.detect_version() {
                        emu = emu.with_config(includes.join(format!("{ver}.config.ini")));
                    }
                    
                    //println!("dumping with bizhawk {}", emu.detect_version().unwrap_or_default());
                    emu.with_rom(ctx.rom)
                        .with_movie(ctx.movie_file)
                        .with_lua(includes.join("tasd-bizhawk.lua"))
                        .run()
                        .unwrap();
                    
                    dumps.push((ctx.source, dump_path));
                },
                MovieFormat::Fm2(_fm2) => {
                    let emu = self.emulators.iter()
                        .find_map(|emu| match emu {
                            EmulatorContext::Fceux(emu) => Some(emu),
                            _ => None,
                        })
                        .cloned();
                    let Some(emu) = emu else { continue };
                    
                    emu.with_rom(ctx.rom)
                        .with_movie(ctx.movie_file)
                        .with_lua(includes.join("tasd-fceux.lua"))
                        .run()
                        .unwrap();
                    
                    dumps.push((ctx.source, dump_path));
                },
                MovieFormat::Gmv(_gmv) => {
                    todo!()
                },
            }
        }
        
        dumps
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
                let paths = self.cache.find_paths(|bundle| bundle.sha1.as_slice() == &sha1);
                if let Some(path) = paths.into_iter().next() {
                    return Ok(path.into());
                }
            }
            
            let md5 = ver.md5.and_then(|hash| hex::decode(hash).ok());
            if let Some(md5) = md5 {
                let paths = self.cache.find_paths(|bundle| bundle.md5.as_slice() == &md5);
                if let Some(path) = paths.into_iter().next() {
                    return Ok(path.into());
                }
            }
        }
        
        println!("searching movie");
        println!("{movie:?}");
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
            
            let paths = self.cache.find_paths(|bundle| bundle.sha1.as_slice() == &hash || bundle.md5.as_slice() == &hash);
            if let Some(path) = paths.into_iter().next() {
                return Ok(path.into());
            }
        }
        
        Err(ConfigError::RomNotFound)
    }
}
