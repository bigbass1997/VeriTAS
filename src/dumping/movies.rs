use std::fmt::Formatter;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use zip::ZipArchive;
use crate::dumping::movies::Format::*;
use crate::dumping::movies::Source::*;
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
                    path,
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
                    path,
                    format: Format::from_extension(ext),
                    source,
                }
            },
            Local(path) => {
                let ext = path.extension().unwrap().to_string_lossy();
                
                Self {
                    path: path.to_owned(),
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
        //todo!()
        None
    }
}