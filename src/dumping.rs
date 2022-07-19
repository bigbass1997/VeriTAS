use std::path::PathBuf;
use clap::ArgMatches;
use crossbeam::sync::WaitGroup;
use log::{info, warn};
use crate::config::{DumperSection, SaveLoad};
use crate::dumping::movies::{Format, Movie, Source};
use crate::dumping::roms::RomCache;

pub mod movies;
pub mod roms;


pub fn handle(matches: &ArgMatches, config: DumperSection) {
    let mut cache = RomCache::load("cache/hashes.toml");
    let mut movies = vec![];
    
    // Collect movies from TASVideos
    if let Some(fetches) = matches.values_of_lossy("fetch") {
        for fetch in fetches {
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
    }
    
    // Collect movies from local machine
    if let Some(local) = matches.value_of("local") {
        let path = PathBuf::from(local);
        if path.is_file() {
            match Movie::with_source(Source::Local(path.clone())) {
                Some(movie) => movies.push(movie),
                None => warn!("Skipping unsupported movie format: {}", path.display()),
            }
        }
    }
    
    if movies.is_empty() {
        warn!("No movies were found. Exiting...");
        return;
    }
    
    // Refresh and save rom cache
    // TODO: Lock refresh behind CLI argument "--refresh"
    info!("Refreshing rom cache...");
    cache.refresh(Some(&config.rom_directory));
    cache.save("cache/hashes.toml");
    
    // Match movies to any cached roms
    info!("Attempting to match movies to roms...");
    let mut prepared = vec![];
    for movie in movies {
        if let Some(hash) = movie.find_hash() {
            if let Some(rom) = cache.search(&hash) {
                prepared.push((movie, rom));
            } else {
                warn!("Failed to find matching rom. Expected hash: {}, Movie: {}", hash, movie.source);
            }
        } else {
            warn!("Failed to find movie's rom hash. Skipping: {}", movie.source);
        }
    }
    
    // Spin up threads for dump procedure
    let wg = WaitGroup::new();
    for (movie, rom) in prepared {
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
                    
                    info!("Dump complete! {}", movie.source);
                    
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
                    
                    info!("Dump complete! {}", movie.source);
                    
                    drop(wg);
                });
                
            },
        }
    }
    
    wg.wait();
}