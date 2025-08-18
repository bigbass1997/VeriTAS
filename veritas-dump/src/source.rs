use std::fmt::Formatter;
use std::io::{Cursor, Read};
use camino::{Utf8Path, Utf8PathBuf};
use flate2::read::GzDecoder;
use zip::ZipArchive;

#[derive(Debug)]
pub enum Error {
    InvalidSource,
    Io(std::io::Error),
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Represents the location of a TAS movie file.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// TASVideos publication
    Publication(i32),
    
    /// TASVideos submission
    Submission(i32),
    
    /// TASVideos userfile
    Userfile(u64),
    
    /// Filepath on the current system
    Local(Utf8PathBuf),
}
impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Publication(id) => write!(f, "{id}M"),
            Self::Submission(id) => write!(f, "{id}S"),
            Self::Userfile(id) => write!(f, "#{id}"),
            Self::Local(path) => write!(f, "{path}"),
        }
    }
}
impl Source {
    /// Attempts to identify the movie specified by the provided text.
    /// 
    /// Text is parsed in the following order, according to these rules:
    /// * **[Local](Source::Local)**: A local filepath to an existing file (excluding directories)
    /// * **[Userfile](Source::Userfile)**: A number, optionally prefixed with a `#` symbol (`#1234567890123456789`)
    /// * **[Submission](Source::Submission)**: A number followed by the letter `S` or `s` (`1234S`)
    /// * **[Publication](Source::Publication)**: A number followed by the letter `M` or `m` (`5678M`)
    pub fn parse<S: AsRef<str>>(text: S) -> Option<Self> {
        use Source::*;
        
        let text = text.as_ref();
        if text.is_empty() {
            return None;
        }
        
        let path = Utf8Path::new(text);
        if path.is_file() {
            return Some(Local(path.to_path_buf()));
        }
        
        if !text.is_ascii() {
            return None;
        }
        
        if text.starts_with('#') && text.len() > 1 {
            return Some(Userfile(text.split_at(1).1.parse().ok()?));
        }
        if text.chars().all(|c| c.is_ascii_digit()) {
            return Some(Userfile(text.parse().ok()?));
        }
        
        let (id, last) = text.split_at(text.len() - 1);
        let id = id.parse().ok()?;
        
        match last {
            "s" | "S" => Some(Submission(id)),
            "m" | "M" => Some(Publication(id)),
            _ => None
        }
    }
    
    /// Reads or downloads the movie file.
    /// 
    /// Returns `None` if source points to a missing or unretrievable file.
    pub fn read(&self) -> Option<(Vec<u8>, String)> {
        Some(match self {
            Self::Publication(id) => {
                let zip = tasvideos_api_rs::get_publication_movie(*id).ok()?;
                let (data, ext) = Self::extract(zip)?;
                
                (data, format!("{self}.{ext}"))
            },
            Self::Submission(id) => {
                let zip = tasvideos_api_rs::get_submission_movie(*id).ok()?;
                let (data, ext) = Self::extract(zip)?;
                
                (data, format!("{self}.{ext}"))
            },
            Self::Userfile(id) => {
                let (gzip, name) = tasvideos_api_rs::get_userfile(*id).ok()?;
                let data = Self::deflate(gzip);
                
                let filename = match name {
                    Some(name) => match Utf8Path::new(&name).extension() {
                        Some(ext) => format!("{self}.{ext}"),
                        None => self.to_string(),
                    },
                    None => self.to_string(),
                };
                
                (data, filename)
            },
            Self::Local(path) => (std::fs::read(path).ok()?, path.file_name()?.to_string())
        })
    }
    
    pub fn is_local(&self) -> bool {
        match self {
            Self::Local(_) => true,
            _ => false,
        }
    }
    
    fn extract(zip: Vec<u8>) -> Option<(Vec<u8>, String)> {
        let mut zip = ZipArchive::new(Cursor::new(zip)).ok()?;
        let mut file = zip.by_index(0).ok()?;
        let filename = file.enclosed_name()?.to_owned();
        let filename = filename.to_string_lossy();
        let (_, ext) = filename.rsplit_once('.')?;
        let mut data = vec![];
        file.read_to_end(&mut data).unwrap();
        
        Some((data, ext.to_owned()))
    }
    
    fn deflate(gzip: Vec<u8>) -> Vec<u8> {
        let mut decoder = GzDecoder::new(gzip.as_slice());
        let mut data = Vec::with_capacity(gzip.len()); // decompressed should be at least as large as compressed
        decoder.read_to_end(&mut data).unwrap();
        
        data
    }
}