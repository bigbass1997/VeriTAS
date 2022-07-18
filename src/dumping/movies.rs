use std::fmt::Formatter;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use base64ct::{Base64, Encoding};
use zip::ZipArchive;
use Format::*;
use Source::*;
use crate::dumping::roms::Hash;


pub enum Format {
    Bk2,
    Fm2,
}
impl Format {
    pub fn from_extension<S: AsRef<str>>(ext: S) -> Self {
        match ext.as_ref().to_lowercase().as_str() {
            "bk2" => Bk2,
            "fm2" => Fm2,
            _ => panic!("Unrecognized movie file extension: {}", ext.as_ref())
        }
    }
}

pub enum Source {
    Publication(i32),
    Submission(i32),
    Local(PathBuf),
}
impl Source {
    pub fn parse<S: AsRef<str>>(id_code: S) -> Option<Self> {
        let id_code = id_code.as_ref().to_uppercase();
        let last_char = id_code.chars().last().unwrap();
        let id = id_code[0..(id_code.len() - 1)].parse().unwrap();
        
        match last_char {
            's' | 'S' => Some(Submission(id)),
            'm' | 'M' => Some(Publication(id)),
            _ => None
        }
    }
}
impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Publication(id) => write!(f, "{}M", id),
            Submission(id) => write!(f, "{}S", id),
            Local(path) => write!(f, "{}", path.display().to_string()),
        }
    }
}

pub struct Movie {
    pub path: PathBuf,
    pub format: Format,
    pub source: Source
}
impl Movie {
    pub fn with_source(source: Source) -> Self {
        match &source {
            Publication(id) => {
                let zip = tasvideos_api_rs::get_publication_movie(*id).unwrap();
                let (data, ext) = Self::extract(zip);
                
                let path = PathBuf::from(format!("cache/movies/{}.{}", source, ext));
                std::fs::write(&path, data).unwrap();
                
                Self {
                    path: path.canonicalize().unwrap_or(path),
                    format: Format::from_extension(ext),
                    source,
                }
            },
            Submission(id) => {
                let zip = tasvideos_api_rs::get_submission_movie(*id).unwrap();
                let (data, ext) = Self::extract(zip);
                
                let path = PathBuf::from(format!("cache/movies/{}.{}", source, ext));
                std::fs::write(&path, data).unwrap();
                
                Self {
                    path: path.canonicalize().unwrap_or(path),
                    format: Format::from_extension(ext),
                    source,
                }
            },
            Local(path) => {
                let ext = path.extension().unwrap().to_string_lossy();
                
                Self {
                    path: path.to_owned().canonicalize().unwrap_or(path.to_owned()),
                    format: Format::from_extension(ext),
                    source,
                }
            },
        }
    }
    
    fn extract(zip: Vec<u8>) -> (Vec<u8>, String) {
        let mut zip = ZipArchive::new(Cursor::new(zip)).unwrap();
        let mut file = zip.by_index(0).unwrap();
        let filename = file.enclosed_name().unwrap().to_owned();
        let filename = filename.to_string_lossy();
        let (_, ext) = filename.rsplit_once('.').unwrap();
        let mut data = vec![];
        file.read_to_end(&mut data).unwrap();
        
        (data, ext.to_owned())
    }
    
    pub fn find_hash(&self) -> Option<Hash> {
        // Attempt to use TASVideos' game database
        fn search_game(game_id: Option<i32>, version_id: Option<i32>) -> Option<Hash> {
            if game_id.is_none() || version_id.is_none() {
                return None;
            }
            
            if let Ok(game) = tasvideos_api_rs::get_game(game_id.unwrap()) {
                for version in game.versions.unwrap() {
                    if version.id.unwrap() == version_id.unwrap() {
                        if let Some(md5) = version.md5 {
                            return Some(Hash::from_hex(md5));
                        } else if let Some(sha1) = version.sha1 {
                            return Some(Hash::from_hex(sha1));
                        }
                    }
                }
            }
            
            None
        }
        match self.source {
            Publication(id) => {
                let publication = tasvideos_api_rs::get_publication(id).unwrap();
                
                if let Some(hash) = search_game(publication.game_id, publication.game_version_id) {
                    return Some(hash);
                }
            },
            Submission(id) => {
                let submission = tasvideos_api_rs::get_submission(id).unwrap();
                
                if let Some(hash) = search_game(submission.game_id, submission.game_version_id) {
                    return Some(hash);
                }
            },
            _ => ()
        }
        
        // Otherwise, resort to parsing the movie file itself
        match self.format {
            Bk2 => {
                let bk2 = std::fs::read(&self.path).unwrap();
                let mut zip = ZipArchive::new(Cursor::new(bk2)).unwrap();
                let result = zip.by_name("Header.txt");
                if let Ok(mut file) = result {
                    let mut buffer = String::new();
                    if file.read_to_string(&mut buffer).is_ok() {
                        for line in buffer.lines() {
                            if line.starts_with("SHA1") {
                                return Some(Hash::from_hex(line.split_once(" ").unwrap_or_default().1));
                            }
                        }
                    }
                }
            },
            Fm2 => {
                let text = std::fs::read_to_string(&self.path).unwrap();
                for line in text.lines() {
                    if line.starts_with("romChecksum") {
                        if let Ok(hex) = Base64::decode_vec(line.split_once(":").unwrap_or_default().1) {
                            return Some(Hash::Md5(hex.try_into().unwrap()));
                        }
                        
                        break;
                    }
                }
            },
        }
        
        None
    }
}