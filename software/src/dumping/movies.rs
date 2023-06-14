use std::fmt::Formatter;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use base64ct::{Base64, Encoding};
use flate2::read::GzDecoder;
use log::warn;
use zip::ZipArchive;
use Format::*;
use Source::*;
use crate::dumping::roms::Hash;


#[derive(Clone, Debug)]
pub enum Format {
    Bk2,
    Fm2,
    Gmv,
}
impl Format {
    pub fn from_extension<S: AsRef<str>>(ext: S) -> Option<Self> {
        match ext.as_ref().to_lowercase().as_str() {
            "bk2" => Some(Bk2),
            "fm2" => Some(Fm2),
            "gmv" => Some(Gmv),
            _ => None
        }
    }
}

#[derive(Clone, Debug)]
pub enum Source {
    Publication(i32),
    Submission(i32),
    Userfile(u64),
    Local(PathBuf),
}
impl Source {
    /// Attempts to identify what kind of [Source] the text contains.
    /// 
    /// Parse order: [Local], [Userfile], [Submission], [Publication]
    /// 
    /// For local sources, this only checks if it exists and is a file.
    pub fn parse<S: AsRef<str>>(text: S) -> Option<Self> {
        let text = text.as_ref();
        {
            let path = Path::new(text);
            if path.is_file() {
                return Some(Local(path.to_path_buf()));
            }
        }
        
        let text = text.to_uppercase();
        if text.starts_with('#') {
            let id = text[1..].parse().unwrap();
            
            return Some(Userfile(id));
        }
        
        let last_char = text.chars().last().unwrap();
        let id = text[0..(text.len() - 1)].parse().unwrap();
        
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
            Userfile(id) => write!(f, "#{}", id),
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
    pub fn with_source(source: Source) -> Option<Self> {
        match &source {
            Publication(id) => {
                let zip = tasvideos_api_rs::get_publication_movie(*id).unwrap();
                let (data, ext) = Self::extract(zip);
                
                let path = PathBuf::from(format!("cache/movies/{}.{}", source, ext));
                std::fs::write(&path, data).unwrap();
                
                if let Some(format) = Format::from_extension(ext) { 
                    Some(Self {
                        path: path.canonicalize().unwrap_or(path),
                        format,
                        source,
                    })
                } else { None }
            },
            Submission(id) => {
                let zip = tasvideos_api_rs::get_submission_movie(*id).unwrap();
                let (data, ext) = Self::extract(zip);
                
                let path = PathBuf::from(format!("cache/movies/{}.{}", source, ext));
                std::fs::write(&path, data).unwrap();
                
                
                if let Some(format) = Format::from_extension(ext) { 
                    Some(Self {
                        path: path.canonicalize().unwrap_or(path),
                        format,
                        source,
                    })
                } else { None }
            },
            Userfile(id) => {
                let (gzip, name) = tasvideos_api_rs::get_userfile(*id).unwrap();
                let data = Self::deflate(gzip);
                
                let filename = match name {
                    Some(name) => {
                        match Path::new(&name).extension() {
                            Some(ext) => format!("{}.{}", source.to_string(), ext.to_string_lossy()),
                            None => {
                                warn!("Userfile has no file extension. File detection will not be possible for: {}", source);
                                source.to_string()
                            },
                        }
                    },
                    None => {
                        warn!("Unable to locate file extension for Userfile: {}", source);
                        source.to_string()
                    },
                };
                let path = PathBuf::from(format!("cache/movies/{}", filename));
                std::fs::write(&path, data).unwrap();
                
                if let Some(format) = Format::from_extension(path.extension().unwrap_or_default().to_string_lossy()) { 
                    Some(Self {
                        path: path.canonicalize().unwrap_or(path),
                        format,
                        source,
                    })
                } else { None }
            },
            Local(path) => {
                let ext = path.extension().unwrap().to_string_lossy();
                
                
                if let Some(format) = Format::from_extension(ext) { 
                    Some(Self {
                        path: path.to_owned().canonicalize().unwrap_or(path.to_owned()),
                        format,
                        source,
                    })
                } else { None }
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
    
    fn deflate(gzip: Vec<u8>) -> Vec<u8> {
        let mut d = GzDecoder::new(gzip.as_slice());
        let mut data = vec![];
        d.read_to_end(&mut data).unwrap();
        
        data
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
                            } else if line.starts_with("MD5") {
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
            Gmv => {
                // GMV format doesn't store any hashes! :(
                return None;
            }
        }
        
        None
    }
}