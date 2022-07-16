use std::fmt::Formatter;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use clap::ArgMatches;
use log::{debug, error, info, warn};
use md5::Digest;
use hex::FromHex;
use zip::ZipArchive;
use crate::config::{DumperSection, HashCache, Config};
use crate::dumping::TasvideosId::*;
use crate::dumping::DumpError::*;

#[derive(Debug)]
pub enum DumpError {
    Tasvideos(tasvideos_api_rs::Error),
    MissingGame,
    MissingGameVersion,
    MissingHash,
}

pub enum TasvideosId {
    Pub(i32),
    Sub(i32),
}
impl std::fmt::Display for TasvideosId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Pub(id) => format!("{}M", *id),
            Sub(id) => format!("{}S", *id),
        })
    }
}
impl TasvideosId {
    pub fn parse<S: AsRef<str>>(id_code: S) -> Option<Self> {
        let id_code = id_code.as_ref().to_uppercase();
        let last_char = id_code.chars().last().unwrap();
        let id = id_code[0..(id_code.len() - 1)].parse().unwrap();
        
        match last_char {
            's' | 'S' => Some(Self::Sub(id)),
            'm' | 'M' => Some(Self::Pub(id)),
            _ => None
        }
    }
}


#[derive(Default, Clone, Debug)]
pub struct MovieDump {
    pub movie: Option<PathBuf>,
    pub rom_hash: Option<Digest>,
    pub rom: Option<PathBuf>,
    pub dump: Option<PathBuf>,
}
impl MovieDump {
    pub fn ready_to_dump(&self) -> bool {
        self.movie.is_some() && self.rom.is_some()
    }
}

pub fn handle(matches: &ArgMatches, config: DumperSection) {
    let mut movies = vec![];
    
    // Fetch any remote movies from TASVideos
    if let Some(fetches) = matches.values_of_lossy("fetch") {
        for id in fetches {
            if let Some(id) = TasvideosId::parse(&id) {
                info!("Downloading {}", id);
                let zip = download(&id).unwrap();
                let mut zip = ZipArchive::new(Cursor::new(zip)).unwrap();
                let filename = zip.file_names().next().unwrap().to_owned();
                
                let mut extracted = zip.by_name(&filename).unwrap();
                let mut data = vec![];
                extracted.read_to_end(&mut data).unwrap();
                
                let filename = format!("{}.{}", id, filename.rsplit_once('.').unwrap_or_default().1);
                let path = PathBuf::from(format!("cache/movies/{}", filename));
                std::fs::write(&path, data).unwrap();
                info!("File saved to {}", path.display());
                
                movies.push(MovieDump {
                    movie: Some(path),
                    rom_hash: Some(tasvideos_game_hash(&id).unwrap()),
                    ..Default::default()
                });
            } else {
                error!("Failed to parse publication/submission ID number: {}", id);
            }
        }
    }
    
    // Check if a valid local file was provided
    /*if let Some(local) = matches.value_of("local") {
        let path = PathBuf::from(local);
        if !path.is_file() {
            error!("Local path is not a file, or could not be found: {}", path.display());
        } else {
            movies.push(MovieDump {
                movie: Some(path),
                ..Default::default()
            });
        }
    }*/
    
    // Attempt to locate matching rom for each movie
    let mut cache = HashCache::load("cache/hashes.toml");
    
    for movie in &mut movies {
        if movie.rom_hash.is_none() {
            warn!("Skipping {} due to unknown rom hash.", movie.movie.as_ref().unwrap().display());
            continue;
        }
        
        let rom_hash = movie.rom_hash.unwrap();
        
        for hash in &cache.hashes {
            if Digest(<[u8; 16]>::from_hex(hash.0).unwrap()) == rom_hash {
                movie.rom = Some(hash.1.clone());
                break;
            }
        }
        
        if movie.rom.is_none() {
            if !config.rom_directory.is_dir() {
                panic!("rom_directory is not configured to a valid directory: {}", config.rom_directory.display());
            }
            
            info!("ROM hash not found in cache. Refreshing cache...");
            cache.refresh(Some(config.rom_directory.canonicalize().unwrap()));
            cache.save("cache/hashes.toml");
            
            for hash in &cache.hashes {
                if Digest(<[u8; 16]>::from_hex(hash.0).unwrap()) == rom_hash {
                    movie.rom = Some(hash.1.clone());
                    break;
                }
            }
        }
        
        info!("Movie: {:?}", movie);
    }
    
    // Spin up emulator for movies with rom
    for movie in &movies {
        if movie.ready_to_dump() {
            /*match movie.movie.unwrap().extension().unwrap().to_string_lossy().to_string().as_str() {
                
            }*/
        }
    }
}

fn download(id: &TasvideosId) -> Result<Vec<u8>, DumpError> {
    match id {
        Sub(id) => tasvideos_api_rs::get_submission_movie(*id),
        Pub(id) => tasvideos_api_rs::get_publication_movie(*id),
    }.map_err(|err| Tasvideos(err))
}

fn tasvideos_game_hash(id: &TasvideosId) -> Result<Digest, DumpError> {
    if let (Some(game_id), Some(version_id)) = match id {
        Sub(id) => match tasvideos_api_rs::get_submission(*id) {
            Ok(sub) => (sub.game_id, sub.game_version_id),
            Err(err) => return Err(Tasvideos(err)),
        },
        Pub(id) => match tasvideos_api_rs::get_publication(*id) {
            Ok(publ) => (publ.game_id, publ.game_version_id),
            Err(err) => return Err(Tasvideos(err)),
        }
    } {
        let game = match tasvideos_api_rs::get_game(game_id) {
            Ok(game) => game,
            Err(err) => return Err(Tasvideos(err)),
        };
        
        let mut version = None;
        for ver in game.versions.unwrap() {
            if let Some(id) = ver.id {
                if id == version_id {
                    version = Some(ver);
                    break;
                }
            }
        }
        if version.is_none() { return Err(MissingGameVersion) }
        let version = version.unwrap();
        
        if let Some(hash) = version.md5 {
            Ok(Digest(<[u8; 16]>::from_hex(hash).unwrap()))
        } else {
            Err(MissingHash)
        }
    } else {
        Err(MissingGame)
    }
}