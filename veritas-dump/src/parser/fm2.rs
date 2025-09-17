use std::collections::HashMap;
use std::io::BufRead;
use crate::parser::MovieFormat;

#[derive(Debug, Clone, PartialEq)]
pub struct Fm2 {
    /// Version of the movie file format.
    pub file_version: i32,
    
    /// Version of the emulator.
    pub emu_version: i32,
    
    /// Number of rerecords in this movie.
    pub rerecords: Option<i32>,
    
    /// True if the movie uses PAL timing.
    pub pal: Option<bool>,
    
    /// True if the movie uses FCEUX's NewPPU feature.
    pub newppu: Option<bool>,
    
    /// True if movie was recorded on a Famicom Disk System game.
    pub fds: Option<bool>,
    
    /// True if a fourscore was used. If false, [port0](Fm2::port0) and [Fm2::port1] _should_ be `Some`.
    pub fourscore: Option<bool>,
    
    pub port0: Option<i32>,
    pub port1: Option<i32>,
    pub port2: i32,
    
    /// True if the input log was stored in binary rather than plaintext.
    pub binary: Option<bool>,
    
    /// Number of frames in the input log.
    /// 
    /// If `Some` and non-negative, any data after this number of
    /// input frames is ignored. This can be used to store additional data after the input log.
    pub length: Option<i32>,
    
    /// Name of the ROM file used to record this movie.
    pub rom_filename: String,
    
    /// Ordered list of comments.
    pub comments: Vec<String>,
    
    /// Ordered list of subtitles.
    /// 
    /// Each element contains the entire value, including the integer value
    /// which by convention _should_ be the first word in the string.
    pub subtitles: Vec<String>,
    
    /// A unique identifier for a movie, generated when the movie was first created.
    /// 
    /// Used by FCEUX when loading savestates to ensure it belongs to this movie.
    /// 
    /// ## Example
    /// `452DE2C3-EF43-2FA9-77AC-0677FC51543B`
    pub guid: String,
    
    /// A base64 encoded hexadecimal string of an MD5 hash.
    pub rom_checksum: String,
    
    //savestate: Option<Fcs>, //TODO: Implement parser for FCEUX savestates (https://fceux.com/web/help/fcs.html)
    
    pub extra_pairs: HashMap<String, String>,
    
    //TODO: Implement input log parsing. Not necessary until moved to separate crate.
    //gamepad_inputs: Vec<[u8; 4]>,
    //zapper_inputs: Vec<[u8; 12]>,
}
impl From<Fm2> for MovieFormat {
    fn from(value: Fm2) -> Self {
        Self::Fm2(value)
    }
}
impl Fm2 {
    pub fn parse(data: &[u8]) -> Option<Self> { //TODO: Change returned type to a `Result` once moved to separate crate.
        let input_index = data.iter()
            .enumerate()
            .find(|(_, byte)| **byte == b'|')
            .map(|(i, _)| i)
            .unwrap_or(data.len());
        
        let mut comments = vec![];
        let mut subtitles = vec![];
        let mut pairs: HashMap<String, String> = data[0..input_index].lines()
            .filter_map(Result::ok)
            .filter_map(|line| line.split_once(' ')
                .map(|(key, val)| (key.to_string(), val.trim().to_string()) )
            )
            .filter(|(key, val)| match key.as_str() {
                "comment" => {
                    comments.push(val.clone());
                    false
                },
                "subtitle" => {
                    subtitles.push(val.clone());
                    false
                },
                _ => true
            })
            .collect();
        
        Some(Self {
            file_version: pairs.remove("version")?.parse().ok()?,
            emu_version: pairs.remove("emuVersion")?.parse().ok()?,
            rerecords: pairs.remove("rerecordCount").map(|x| x.parse().ok()).flatten(),
            pal: pairs.remove("palFlag").map(|x| x.parse().ok()).flatten(),
            newppu: pairs.remove("NewPPU").map(|x| x.parse().ok()).flatten(),
            fds: pairs.remove("FDS").map(|x| x.parse().ok()).flatten(),
            fourscore: pairs.remove("fourscore").map(|x| x.parse().ok()).flatten(),
            port0: pairs.remove("port0").map(|x| x.parse().ok()).flatten(),
            port1: pairs.remove("port1").map(|x| x.parse().ok()).flatten(),
            port2: pairs.remove("port2")?.parse().ok()?,
            binary: pairs.remove("binary").map(|x| x.parse().ok()).flatten(),
            length: pairs.remove("length").map(|x| x.parse().ok()).flatten(),
            rom_filename: pairs.remove("romFilename").or_else(|| {
                data[0..input_index].split(|b| *b == b'\n')
                    .map(|line| line.trim_ascii_end())
                    .find(|line| line.starts_with(b"romFilename"))
                    .map(|line| line.splitn(2, |b| *b == b' ').nth(1))
                    .flatten()
                    .map(|ver| String::from_utf8_lossy(ver).to_string())
            })?,
            comments,
            subtitles,
            guid: pairs.remove("guid")?,
            rom_checksum: pairs.remove("romChecksum")?,
            //savestate: Option<Fcs>, //TODO: Implement parser for FCEUX savestates (https://fceux.com/web/help/fcs.html)
            extra_pairs: pairs,
        })
    }
    
    pub fn version_string(&self) -> String {
        let major = self.emu_version / 10000;
        let minor = (self.emu_version / 100) % 100;
        let patch = self.emu_version % 100;
        
        format!("{major}.{minor}.{patch}")
    }
}
