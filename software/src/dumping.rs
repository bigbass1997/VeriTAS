use std::fs::File;
use std::io::Read;
use camino::Utf8PathBuf;
use crossbeam::sync::WaitGroup;
use emu_runner::contexts::{BizHawkContext, FceuxContext, GensContext};
use emu_runner::contexts::gens::GensVersion;
use emu_runner::EmulatorContext;
use emu_runner::includes::copy_if_different;
use log::{error, info, warn};
use zip::ZipArchive;
use crate::config::{DumperSection, SaveLoad};
use crate::DumpArgs;
use crate::dumping::movies::{Format, Movie, Source};
use crate::dumping::roms::{Rom, RomCache};

pub mod movies;
pub mod roms;

fn collect_movies(args: &DumpArgs) -> Vec<Movie> {
    let mut movies = vec![];
    
    // Parse provided movie sources, download as necessary from TASVideos
    for fetch in &args.fetch {
        movies.push(match Source::parse(fetch) {
            Some(source) => match Movie::with_source(source.clone()) {
                Some(movie) => movie,
                None => {
                    warn!("Skipping unsupported/unrecognized movie: {}", source);
                    continue;
                }
            },
            None => {
                warn!("Failed to parse movie path/ID: {}", fetch);
                continue;
            },
        });
    }
    
    movies
}

fn pair_movies(args: &DumpArgs, cache: &RomCache, movies: Vec<Movie>) -> Vec<(Movie, Rom)> {
    if movies.is_empty() {
        return vec![];
    }
    
    info!("Attempting to locate roms for each movie...");
    let mut pairs = vec![];
    
    let mut rom_override = args.rom_override.clone();
    if movies.len() > 1 && args.rom_override.is_some() {
        warn!("Override ROM provided, but multiple movies specified. ROM will only be used with the first movie.");
        rom_override = None;
    }
    
    for movie in movies {
        // Use override if it exists and is valid
        if let Some(path) = rom_override.take() {
            if path.is_file() {
                let roms = Rom::with_path(&path);
                if roms.len() > 0 {
                    pairs.push((movie, Rom::with_path(path).remove(0)));
                    continue;
                } else {
                    error!("Provided override ROM was not recognized. Override ignored.");
                }
            } else {
                error!("Provided override ROM does not exist. Override ignored.");
            }
        }
        
        // Search for hash and compare it against rom cache
        if let Some(hash) = movie.find_hash() {
            if let Some(rom) = cache.search(&hash) {
                pairs.push((movie, rom));
            } else {
                warn!("Failed to find matching rom. Expected hash: {}. Skipping: {}", hash, movie.source);
            }
        } else {
            warn!("Failed to find movie's rom hash. Skipping: {}", movie.source);
        }
    }

    pairs
}


pub fn handle(args: DumpArgs, config: DumperSection) {
    std::fs::create_dir_all("cache/movies").unwrap_or_default();
    let mut cache = RomCache::load("cache/hashes.toml");
    let movies = collect_movies(&args);
    
    if movies.is_empty() {
        info!("No movies were found. Exiting...");
        return;
    }
    
    // Refresh and save rom cache
    if cache.roms.is_empty() || args.refresh || cache.is_fs_outdated(&config.roms_path) {
        info!("Refreshing rom cache...");
        cache.refresh(Some(&config.roms_path));
        cache.save("cache/hashes.toml");
    }
    
    copy_if_different(include_bytes!("includes/2.6.config.ini"),     "cache/2.6.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.6.1.config.ini"),   "cache/2.6.1.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.6.2.config.ini"),   "cache/2.6.2.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.6.3.config.ini"),   "cache/2.6.3.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.7.config.ini"),     "cache/2.7.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.8.config.ini"),     "cache/2.8.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.8-rc1.config.ini"), "cache/2.8-rc1.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.9.config.ini"),     "cache/2.9.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.9-rc1.config.ini"), "cache/2.9-rc1.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.9-rc2.config.ini"), "cache/2.9-rc2.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.9-rc3.config.ini"), "cache/2.9-rc3.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/2.9.1.config.ini"),   "cache/2.9.1.config.ini").unwrap();
    copy_if_different(include_bytes!("includes/tasd-api.lua"),       "cache/tasd-api.lua").unwrap();
    copy_if_different(include_bytes!("includes/tasd-bizhawk.lua"),   "cache/tasd-bizhawk.lua").unwrap();
    copy_if_different(include_bytes!("includes/tasd-fceux.lua"),     "cache/tasd-fceux.lua").unwrap();
    copy_if_different(include_bytes!("includes/tasd-gens.lua"),      "cache/tasd-gens.lua").unwrap();
    
    let pairs = pair_movies(&args, &cache, movies);
    
    // Spin up threads for dump procedure
    let wg = WaitGroup::new();
    for (movie, rom) in pairs {
        if !rom.path.is_file() {
            warn!("Attempted to dump using cached ROM that no longer exists. Recommend running 'veritas dump --refresh'. Path: {}", rom.path);
            continue
        }
        info!("Beginning dump: {}", movie.source);
        
        match movie.format {
            Format::Bk2 => {
                let mut emu_path = config.bizhawk_emus.first().cloned();
                for path in &config.bizhawk_emus {
                    let ctx = BizHawkContext::new(path);
                    let ctx = if let Ok(ctx) = ctx {
                        ctx
                    } else {
                        continue;
                    };
                    
                    if let Some(ver) = ctx.detect_version() {
                        let mut zip = ZipArchive::new(File::open(&movie.path).unwrap()).unwrap();
                        let mut header = zip.by_name("Header.txt").unwrap();
                        let mut text = String::with_capacity(256);
                        header.read_to_string(&mut text).unwrap();
                        
                        let expected = text.lines().find(|line| line.starts_with("OriginalEmuVersion")).unwrap();
                        let (_, expected) = expected.rsplit_once(' ').unwrap();
                        
                        if ver == expected {
                            emu_path = Some(path.clone());
                            break;
                        }
                    }
                }
                if emu_path.is_none() {
                    error!("Unable to locate BizHawk emulator! Be sure you provided at least one emulator path in veritas.toml.");
                    continue;
                }
                let mut ctx = BizHawkContext::new(emu_path.unwrap()).unwrap()
                    //.with_config("cache/config-bizhawk.ini")
                    .with_lua("cache/tasd-bizhawk.lua")
                    .with_movie(&movie.path)
                    .with_rom(&rom.path);
                
                if let Some(ver) = ctx.detect_version() {
                    let path: Utf8PathBuf = format!("{ver}.config.ini").into();
                    if path.is_file() {
                        ctx = ctx.with_config(path);
                    }
                }
                
                let wg = wg.clone();
                std::thread::spawn(move || {
                    emu_runner::run(ctx).unwrap();
                    
                    let mut dump_path = movie.path.clone();
                    dump_path.set_extension("tasd");
                    
                    if dump_path.is_file() {
                        info!("Dump complete! {dump_path} {}", rom.path);
                    } else {
                        warn!("Dump failed! {}", movie.source);
                    }
                    
                    drop(wg);
                });
            },
            Format::Fm2 => {
                let emu_path = config.fceux_emus.first().cloned();
                if emu_path.is_none() {
                    error!("Unable to locate FCEUX emulator! Be sure you provided at least one emulator path in veritas.toml.");
                    continue;
                }
                let ctx = FceuxContext::new(emu_path.unwrap()).unwrap()
                    .with_lua("cache/tasd-fceux.lua")
                    .with_movie(&movie.path)
                    .with_rom(&rom.path);
                
                let wg = wg.clone();
                std::thread::spawn(move || {
                    emu_runner::run(ctx).unwrap();
                    
                    let mut dump_path = movie.path.clone();
                    dump_path.set_extension("tasd");
                    
                    if dump_path.is_file() {
                        info!("Dump complete! {dump_path} {}", rom.path);
                    } else {
                        warn!("Dump failed! {}", movie.source);
                    }
                    
                    drop(wg);
                });
            },
            Format::Gmv => {
                let ctx = GensContext::new(&config.gens_emu, GensVersion::GitA2425B5).unwrap()
                    .with_lua("cache/tasd-gens.lua")
                    .with_movie(&movie.path)
                    .with_rom(&rom.path);
                if !ctx.lua.as_ref().unwrap().is_absolute() {
                    let mut path = ctx.working_dir();
                    path.push("tasd-api.lua");
                    copy_if_different(&std::fs::read("cache/tasd-api.lua").unwrap(), path).unwrap();
                }
                
                let wg = wg.clone();
                std::thread::spawn(move || {
                    //let mut movie_path = PathBuf::from("cache/movies/");
                    //movie_path.push(movie.path.file_name().unwrap());
                    
                    emu_runner::run(ctx).unwrap();
                    
                    let mut dump_path = movie.path.clone();
                    dump_path.set_extension("tasd");
                    
                    if dump_path.is_file() {
                        info!("Dump complete! {dump_path} {}", rom.path);
                    } else {
                        warn!("Dump failed! {}", movie.source);
                    }
                    
                    drop(wg);
                });
            },
        }
    }
    
    wg.wait();
}