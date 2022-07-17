use std::path::PathBuf;
use clap::ArgMatches;
use log::warn;
use crate::config::{DumperSection, SaveLoad};
use crate::dumping::movies::{Movie, Source};
use crate::dumping::roms::RomCache;

pub mod movies;
pub mod roms;


pub fn handle(matches: &ArgMatches, config: DumperSection) {
    let mut cache = RomCache::load("cache/hashes.toml");
    let mut movies = vec![];
    
    // Collect movies from TASVideos
    if let Some(fetches) = matches.values_of_lossy("fetch") {
        for fetch in fetches {
            movies.push(match Source::parse(fetch) {
                Some(source) => Movie::with_source(source),
                None => continue,
            });
        }
    }
    
    // Collect movies from local machine
    if let Some(local) = matches.value_of("local") {
        let path = PathBuf::from(local);
        if path.is_file() {
            movies.push(Movie::with_source(Source::Local(path)));
        }
    }
    
    // Refresh and save rom cache
    // TODO: Lock refresh behind CLI argument "--refresh"
    cache.refresh(Some(config.rom_directory));
    cache.save("cache/hashes.toml");
    
    // Match movies to any cached roms
    let mut prepared = vec![];
    for movie in movies {
        if let Some(hash) = movie.find_hash() {
            if let Some(rom) = cache.search(&hash) {
                prepared.push((movie, rom));
            } else {
                warn!("Failed to find matching rom for this movie. Expected hash: {}, Movie: {}", hash, movie.source);
            }
        } else {
            warn!("Failed to find rom hash for this movie. Skipping: {}", movie.source);
        }
    }
    
    // Spin up threads for dump procedure
}