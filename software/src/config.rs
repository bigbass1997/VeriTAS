use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

pub trait SaveLoad: Serialize + DeserializeOwned + Default {
    fn save<P: AsRef<Path>>(&self, path: P) {
        std::fs::write(path, toml::to_string(&self).unwrap()).unwrap();
    }
    
    fn load<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if path.exists() {
            toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
        } else {
            let config = Self::default();
            config.save(path);
            
            config
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DumperSection {
    pub rom_directory: PathBuf,
    pub fceux_path: PathBuf,
    pub bizhawk_path: PathBuf,
}
impl Default for DumperSection {
    fn default() -> Self { Self {
        rom_directory: PathBuf::from("roms"),
        fceux_path: PathBuf::from(""),
        bizhawk_path: PathBuf::from(""),
    }}
}

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct VeritasConfig {
    pub useragent: String,
    pub dumper: DumperSection,
}
impl SaveLoad for VeritasConfig {}



/*#[derive(Default, Clone, Deserialize, Serialize)]
pub struct HashCache {
    pub hashes: HashMap<String, PathBuf>
}
impl Config for HashCache {}
impl HashCache {
    pub fn refresh<P: AsRef<Path>>(&mut self, path: Option<P>) {
        self.hashes.retain(|_, path| path.is_file());
        
        if let Some(path) = path {
            let path = path.as_ref();
            if path.is_file() {
                let data = std::fs::read(&path).unwrap_or_default();
                
                // NES
                if &data[0..4] == &[0x4E, 0x45, 0x53, 0x1A] {
                    self.hashes.insert(hex::encode_upper(md5::compute(&data[16..]).0), path.to_path_buf());
                    self.hashes.insert(hex::encode_upper(md5::compute(&data).0), path.to_path_buf());
                    if (data[7] & 0x08) != 0 { // NES2.0
                        let mut data = data.clone();
                        (&mut data[7..16]).fill(0);
                        self.hashes.insert(hex::encode_upper(md5::compute(&data).0), path.to_path_buf());
                    }
                }
                
                // Generic
                self.hashes.insert(hex::encode_upper(md5::compute(data).0), path.to_path_buf());
                
                debug!("Calculated hash for: {}", path.display());
            }
        }
    }
}*/

/*pub fn save<C: Config, P: AsRef<Path>>(config: C, path: P) {
    std::fs::write(path, toml::to_string_pretty(&config).unwrap()).unwrap_or_default();
}

pub fn load<C: Config, P: AsRef<Path>>(path: P) -> C {
    let path = path.as_ref();
    if path.exists() {
        toml::from_str(&std::fs::read_to_string(path).unwrap_or_default()).unwrap_or_default()
    } else {
        C::default()
    }
}*/