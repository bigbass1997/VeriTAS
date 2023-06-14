use std::path::{Path, PathBuf};
use crossbeam::sync::WaitGroup;
use log::{info, warn};
use crate::config::{DumperSection, SaveLoad};
use crate::DumpArgs;
use crate::dumping::movies::{Format, Movie, Source};
use crate::dumping::roms::{Rom, RomCache};

pub mod movies;
pub mod roms;

pub fn handle(args: DumpArgs, config: DumperSection) {
    let mut cache = RomCache::load("cache/hashes.toml");
    let mut movies = vec![];
    
    if args.local_override.is_some() && args.local.is_none() {
        warn!("Override ROM provided, but no --local movie was specified. Override will be ignored.");
    }
    
    // Collect movies from TASVideos
    for fetch in args.fetch {
        movies.push(match Source::parse(&fetch) {
            Some(source) => match Movie::with_source(source.clone()) {
                Some(movie) => movie,
                None => {
                    warn!("Skipping unsupported movie format: {}", source);
                    continue;
                }
            },
            None => {
                warn!("Couldn't recognize movie ID: {}", fetch);
                continue;
            },
        });
    }
    
    // Collect movies from local machine
    if let Some(path) = args.local.as_ref() {
        if path.is_file() {
            match Movie::with_source(Source::Local(path.clone())) {
                Some(movie) => movies.push(movie),
                None => warn!("Skipping unsupported movie format: {}", path.display()),
            }
        } else {
            warn!("Local movie is not found: {}", path.display());
        }
    }
    
    if movies.is_empty() {
        warn!("No movies were found. Exiting...");
        return;
    }
    
    // Refresh and save rom cache
    if cache.roms.is_empty() || args.refresh || cache.is_fs_outdated(&config.rom_directory) {
        info!("Refreshing rom cache...");
        cache.refresh(Some(&config.rom_directory));
        cache.save("cache/hashes.toml");
    }
    
    // Match movies to any cached roms
    info!("Attempting to match movies to roms...");
    let mut prepared = vec![];
    for movie in movies {
        if let Some(hash) = movie.find_hash() {
            if let Some(rom) = cache.search(&hash) {
                prepared.push((movie, rom));
            } else if let Some(over) = args.local_override.as_ref() {
                if movie.path == args.local.as_ref().unwrap().canonicalize().unwrap() {
                    prepared.push((movie, Rom::with_path(over).remove(0)));
                    info!("Using override");
                }
            } else {
                warn!("Failed to find matching rom. Expected hash: {}, Movie: {}", hash, movie.source);
            }
        } else if let Some(over) = args.local_override.as_ref() {
            if movie.path == args.local.as_ref().unwrap().canonicalize().unwrap() {
                prepared.push((movie, Rom::with_path(over).remove(0)));
                info!("Using override");
            }
        } else {
            warn!("Failed to find movie's rom hash. Skipping: {}", movie.source);
        }
    }
    
    // Spin up threads for dump procedure
    let wg = WaitGroup::new();
    for (movie, rom) in prepared {
        if !rom.path.is_file() {
            warn!("Attempted to dump using cached ROM that no longer exists. Recommend running 'veritas dump --refresh'. Path: {}", rom.path.display());
            continue
        }
        info!("Beginning dump: {}", movie.source);
        
        match movie.format {
            Format::Bk2 => {
                if !config.bizhawk_path.exists() {
                    warn!("BizHawk path is empty or doesn't exist. Skipping: {}", movie.source);
                    continue;
                }
                if !config.bizhawk_path.is_dir() {
                    warn!("BizHawk path must point to the directory that contains EmuHawk.exe. Skipping: {}", movie.source);
                    continue;
                }
                
                let mut bash_path = config.bizhawk_path.canonicalize().unwrap_or(config.bizhawk_path.clone());
                bash_path.push("start-bizhawk.sh");
                if !bash_path.exists() {
                    std::fs::write(&bash_path, include_bytes!("includes/start-bizhawk.sh")).unwrap();
                }
                
                let cfg_path = PathBuf::from("cache/config-bizhawk.ini").canonicalize().unwrap();
                
                let wg = wg.clone();
                std::thread::spawn(move || {
                    let script_path = PathBuf::from("cache/tasd-bizhawk.lua").canonicalize().unwrap();
                    
                    std::process::Command::new("bash")
                        .args([
                            &bash_path.display().to_string(),
                            &format!("--config={}", cfg_path.display()),
                            &format!("--movie={}", movie.path.display()),
                            &format!("--lua={}", script_path.display()),
                            &rom.path.display().to_string(),
                        ]).output().unwrap();
                    
                    let mut dump_path = movie.path.clone();
                    dump_path.set_extension("tasd");
                    
                    if dump_path.is_file() {
                        info!("Dump complete! {} {}", dump_path.display(), rom.path.display());
                    } else {
                        warn!("Dump failed! {}", movie.source);
                    }
                    
                    drop(wg);
                });
            },
            Format::Fm2 => {
                if !config.fceux_path.exists() {
                    warn!("FCEUX path is empty or doesn't exist. Skipping: {}", movie.source);
                    continue;
                }
                
                let config = config.clone();
                let wg = wg.clone();
                std::thread::spawn(move || {
                    let script_path = PathBuf::from("cache/tasd-fceux.lua").canonicalize().unwrap();
                    
                    std::process::Command::new("wine")
                        .args([
                            &config.fceux_path.display().to_string(),
                            "-playmovie", &movie.path.display().to_string(),
                            "-lua", &script_path.display().to_string(),
                            &rom.path.display().to_string()
                        ]).output().unwrap();
                    
                    let mut dump_path = movie.path.clone();
                    dump_path.set_extension("tasd");
                    
                    if dump_path.is_file() {
                        info!("Dump complete! {} {}", dump_path.display(), rom.path.display());
                    } else {
                        warn!("Dump failed! {}", movie.source);
                    }
                    
                    drop(wg);
                });
            },
            Format::Gmv => {
                if !config.gens_path.exists() {
                    warn!("Gens path is empty or doesn't exist. Skipping: {}", movie.source);
                    continue;
                }
                let mut gens_dir = config.gens_path.clone();
                gens_dir.pop();
                
                let mut api_file = gens_dir.clone();
                api_file.push("tasd-api.lua");
                if !api_file.exists() {
                    copy_into_dir("cache/tasd-api.lua", &gens_dir);
                }
                
                let config = config.clone();
                let wg = wg.clone();
                std::thread::spawn(move || {
                    let script_path = PathBuf::from("cache/tasd-gens.lua");
                    
                    copy_into_dir(&rom.path, &gens_dir);
                    
                    let mut movie_path = PathBuf::from("cache/movies/");
                    movie_path.push(movie.path.file_name().unwrap());
                    
                    let mut prefix_path = config.gens_path.canonicalize().unwrap();
                    prefix_path.pop();
                    prefix_path.push(".wine_prefix/");
                    if !prefix_path.is_dir() {
                        std::fs::create_dir(&prefix_path).unwrap();
                    }
                    
                    std::process::Command::new("wine")
                        .args([
                            &config.gens_path.display().to_string(),
                            //"-pause", "0",
                            "-rom", &rom.path.file_name().unwrap().to_string_lossy(),
                            "-play", &movie_path.display().to_string(),
                            "-lua", &script_path.display().to_string()
                        ])
                        .env("WINEPREFIX", prefix_path.as_os_str())
                        .output().unwrap();
                    
                    let mut dump_path = movie.path.clone();
                    dump_path.set_extension("tasd");
                    
                    if dump_path.is_file() {
                        info!("Dump complete! {} {}", dump_path.display(), rom.path.display());
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



fn copy_into_dir<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to_dir: Q) {
    if !from.as_ref().is_file() || !to_dir.as_ref().is_dir() {
        return;
    }
    
    let mut to = to_dir.as_ref().to_path_buf();
    to.push(from.as_ref().file_name().unwrap());
    
    std::fs::copy(from, to).unwrap();
}