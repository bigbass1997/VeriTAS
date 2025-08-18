use std::collections::HashMap;
use std::io::{BufRead, Cursor, Read};
use zip::ZipArchive;
use crate::parser::MovieFormat;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Bk2 {
    pub movie_version: Option<String>,
    pub author: Option<String>,
    pub rerecords: Option<u64>,
    pub emu_version: Option<String>,
    pub platform: Option<String>,
    pub game_name: Option<String>,
    
    /// Checksum of the game associated with this movie.
    /// 
    /// Contains either a CRC32, MD5, or most commonly a SHA1 hash. In all cases, it should be a
    /// hex-encoded string, _without_ any prefix (i.e. `0x`).
    pub checksum: Option<String>,
    
    pub board_name: Option<String>,
    pub core_name: Option<String>,
    
    /// Other key-value pairs from the header.
    pub extra_pairs: HashMap<String, String>,
    
    /// Contents of the input log text file.
    pub input_log: Option<String>,
    
    pub comments: Option<Vec<u8>>,
    pub subtitles: Option<Vec<String>>,
    
    /// JSON blob of all core settings required for sync.
    pub sync_settings: Option<Vec<u8>>,
    
    /// The initial core state of a savestate-anchored movie.
    pub core_state: Option<Vec<u8>>,
    
    /// Other files contained in the movie archive.
    pub extra_files: HashMap<String, Vec<u8>>,
}
impl From<Bk2> for MovieFormat {
    fn from(value: Bk2) -> Self {
        Self::Bk2(value)
    }
}
impl Bk2 {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let mut zip = ZipArchive::new(Cursor::new(data)).ok()?;
        let mut files = HashMap::new();
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).unwrap();
            let name = file.enclosed_name()?.file_name()?.to_string_lossy().to_string();
            let mut buf = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buf).ok()?;
            
            files.insert(name, buf);
        }
        
        let mut bk2 = Self::default();
        
        if let Some(header) = files.remove("Header.txt").or_else(|| files.remove("Header")) {
            let mut pairs: HashMap<String, String> = header.lines()
                .filter_map(Result::ok)
                .filter_map(|line| line.split_once(' ')
                    .map(|(key, val)| (key.to_string(), val.trim().to_string()) )
                )
                .collect();
            
            bk2.movie_version = pairs.remove("MovieVersion");
            bk2.author = pairs.remove("Author");
            bk2.rerecords = pairs.remove("rerecordCount").map(|x| x.parse().ok()).flatten();
            bk2.emu_version = pairs.remove("emuVersion");
            bk2.platform = pairs.remove("Platform");
            bk2.game_name = pairs.remove("GameName");
            bk2.checksum = pairs.remove("SHA1");
            bk2.board_name = pairs.remove("BoardName");
            bk2.core_name = pairs.remove("Core");
            bk2.extra_pairs = pairs;
        }
        
        if let Some(log) = files.remove("Input Log.txt").or_else(|| files.remove("Input Log")) {
            if let Ok(log) = String::from_utf8(log) {
                bk2.input_log = Some(log);
            }
        }
        
        bk2.comments = files.remove("Comments.txt").or_else(|| files.remove("Comments"));
        
        if let Some(subtitles) = files.remove("Subtitles.txt").or_else(|| files.remove("Subtitles")) {
            bk2.subtitles = subtitles
                .lines()
                .collect::<Result<Vec<String>, _>>()
                .ok();
        }
        
        bk2.sync_settings = files.remove("SyncSettings.txt").or_else(|| files.remove("SyncSettings"));
        
        Some(bk2)
    }
}