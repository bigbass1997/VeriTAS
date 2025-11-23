use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::ser::PrettyFormatter;
use serde_json::Serializer;
use crate::cli::EncodeArgs;

#[derive(Debug, Default, Serialize, Deserialize)]
struct EncodeConfig {
    #[serde(skip)]
    path: Utf8PathBuf,
    
    client_id: String,
    api_key: String,
}
impl EncodeConfig {
    pub fn load(path: impl Into<Utf8PathBuf>) -> Result<Self, String> {
        let path = path.into();
        
        if !path.is_file() {
            let cfg = Self { path, ..Default::default() };
            cfg.save().map_err(|e| e.to_string())?;
            
            return Ok(cfg);
        }
        
        
        let data = std::fs::read(&path).map_err(|e| e.to_string())?;
        
        let mut cfg: EncodeConfig = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        cfg.path = path;
        
        Ok(cfg)
    }
    
    pub fn save(&self) -> Result<(), std::io::Error> {
        let mut data = Vec::with_capacity(128);
        let mut ser = Serializer::with_formatter(&mut data, PrettyFormatter::with_indent(b"    "));
        
        self.serialize(&mut ser).expect("should be serializable to JSON");
        
        std::fs::write(&self.path, data)
    }
}



pub fn handle(cache_root: Utf8PathBuf, _args: EncodeArgs) {
    let _cfg = EncodeConfig::load(cache_root.join("encode_config.json")).expect("expected valid JSON file");
    
    
}