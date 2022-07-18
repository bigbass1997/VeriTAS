use std::collections::HashSet;
use std::fmt::Formatter;
use std::path::{Path, PathBuf};
use log::warn;
use serde::{Deserialize, Serialize};
use md5::{Md5, Digest};
use sha1::Sha1;
use crate::dumping::roms::System::*;
use crate::SaveLoad;

#[derive(Clone, Debug)]
pub enum Hash {
    Sha1([u8; 20]),
    Md5([u8; 16]),
}
impl Hash {
    pub fn to_hex(&self) -> String {
        hex::encode_upper(match self {
            Hash::Sha1(arr) => arr.as_ref(),
            Hash::Md5(arr) => arr.as_ref(),
        })
    }
    
    pub fn from_hex<S: AsRef<str>>(hex: S) -> Hash {
        let hex = hex.as_ref();
        
        match hex.len() {
            40 => Hash::Sha1(hex::decode(hex).unwrap().try_into().unwrap()),
            32 => Hash::Md5(hex::decode(hex).unwrap().try_into().unwrap()),
            _ => panic!("Provided hash string is unsupported. Incorrect length.")
        }
    }
}
impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Hash::Sha1(_) => write!(f, "SHA-1: {}", self.to_hex()),
            Hash::Md5(_) => write!(f, "MD5: {}", self.to_hex()),
        }
    }
}

mod serde_sha1 {
    use std::fmt::Formatter;
    use serde::{Deserializer, Serializer};
    use serde::de::{Error, Visitor};
    use crate::dumping::roms::Hash;

    pub fn serialize<S: Serializer>(hash: &[u8; 20], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&Hash::Sha1(*hash).to_hex())
    }
    
    struct Sha1Visitor;
    impl<'de> Visitor<'de> for Sha1Visitor {
        type Value = [u8; 20];

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            write!(formatter, "a hex string of a SHA-1 hash")
        }

        fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(match Hash::from_hex(v) {
                Hash::Sha1(hash) => hash,
                _ => panic!("Failed to parse cached SHA-1 hash. Recommend deleting cache and retrying."),
            })
        }
    }
    
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 20], D::Error> {
        deserializer.deserialize_str(Sha1Visitor)
    }
}
mod serde_md5 {
    use std::fmt::Formatter;
    use serde::{Deserializer, Serializer};
    use serde::de::{Error, Visitor};
    use crate::dumping::roms::Hash;

    pub fn serialize<S: Serializer>(hash: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&Hash::Md5(*hash).to_hex())
    }
    
    struct Md5Visitor;
    impl<'de> Visitor<'de> for Md5Visitor {
        type Value = [u8; 16];

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            write!(formatter, "a hex string of a MD5 hash")
        }

        fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(match Hash::from_hex(v) {
                Hash::Md5(hash) => hash,
                _ => panic!("Failed to parse cached MD5 hash. Recommend deleting cache and retrying."),
            })
        }
    }
    
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 16], D::Error> {
        deserializer.deserialize_str(Md5Visitor)
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub enum System {
    Nes,
    N64,
    Atari2600,
}
impl System {
    pub fn parse(data: &[u8], extension: &str) -> Option<Self> {
        // Check for a magic number
        if data.len() >= 4 {
            match data[0..4] {
                [0x4E, 0x45, 0x53, 0x1A] => return Some(Nes),
                [0x80, 0x37, 0x12, 0x40] => return Some(N64), // Only applies to official and some homebrew games, in native big-endian format
                _ => ()
            }
        }
        
        // Check file extension
        match extension {
            "nes" => return Some(Nes),
            "z64" | "n64" => return Some(N64),
            "a26" => return Some(Atari2600),
            _ => ()
        }
        
        None
    }
    
    pub fn hashes(&self, data: &[u8]) -> Vec<([u8; 20], [u8; 16])> {
        let mut hashes = vec![];
        
        match self {
            Nes => {
                hashes.push(Self::hash(data));
                
                if data.len() > 16 {
                    hashes.push(Self::hash(&data[16..]));
                    
                    if (data[7] & 0x08) != 0 { // NES2.0
                        let mut data = data.to_vec();
                        (&mut data[7..16]).fill(0);
                        
                        hashes.push(Self::hash(&data));
                    }
                    
                }
            },
            N64 => hashes.push(Self::hash(data)),
            Atari2600 => hashes.push(Self::hash(data)),
        }
        
        hashes
    }
    
    fn hash(data: &[u8]) -> ([u8; 20], [u8; 16]) {
        ( Sha1::digest(&data).try_into().unwrap(), Md5::digest(&data).try_into().unwrap() )
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct Rom {
    pub path: PathBuf,
    pub system: System,
    #[serde(with = "serde_sha1")]
    pub sha1: [u8; 20],
    #[serde(with = "serde_md5")]
    pub md5: [u8; 16],
}
impl Rom {
    pub fn with_path<P: AsRef<Path>>(path: P) -> Vec<Rom> {
        let path = path.as_ref();
        if !path.is_file() { return vec![] }
        
        let data = std::fs::read(path).unwrap();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let system = match System::parse(&data, &ext) {
            Some(system) => system,
            None => return vec![],
        };
        
        let mut roms = vec![];
        
        for (sha1, md5) in system.hashes(&data) {
            roms.push(Self {
                path: path.to_path_buf(),
                system,
                //sha1: Sha1::digest(&data).try_into().unwrap(),
                //md5: Md5::digest(&data).try_into().unwrap(),
                sha1,
                md5,
            })
        }
        
        
        roms
    }
    
    pub fn compare_hash(&self, hash: &Hash) -> bool {
        match hash {
            Hash::Sha1(hash) => self.sha1 == *hash,
            Hash::Md5(hash) => self.md5 == *hash,
        }
    }
}

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct RomCache {
    pub roms: HashSet<Rom>,
}
impl SaveLoad for RomCache {}
impl RomCache {
    pub fn refresh<P: AsRef<Path>>(&mut self, path: Option<P>) {
        self.roms.retain(|rom| rom.path.is_file());
        
        if let Some(path) = path {
            let path = path.as_ref();
            let path = path.canonicalize().unwrap_or(path.to_path_buf());
            if path.is_file() {
                warn!("Expecting a directory of roms, instead got a single file: {}", path.display());
            } else if path.is_dir() {
                for file in Self::recursive_files(path) {
                    self.roms.extend(Rom::with_path(file));
                }
            } else {
                warn!("Unable to check rom hashes; provided rom directory doesn't exist: {}", path.display());
            }
        }
    }
    
    fn recursive_files<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
        let mut files = vec![];
        
        match path.as_ref().read_dir() {
            Ok(dir) => for entry in dir {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        files.push(path);
                    } else if path.is_dir() {
                        files.extend_from_slice(&Self::recursive_files(path));
                    }
                }
            },
            Err(_) => return files,
        }
        
        files
    }
    
    pub fn search(&self, hash: &Hash) -> Option<Rom> {
        for rom in &self.roms {
            if rom.compare_hash(hash) {
                return Some(rom.clone());
            }
        }
        
        None
    }
}