use std::fmt::{Display, Formatter};
use std::process::Command;
use camino::Utf8PathBuf;
use crate::contexts::EmulatorContext;
use crate::Error;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GensVersion {
    Ver11A,
    Ver11B,
    GitA2425B5,
    Unknown,
}
impl Display for GensVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Ver11A => "11a",
            Self::Ver11B => "11b",
            Self::GitA2425B5 => "a2425B5",
            Self::Unknown => "unknown",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GensContext {
    pub version: GensVersion,
    pub start_paused: bool,
    pub rom: Option<Utf8PathBuf>,
    pub movie: Option<Utf8PathBuf>,
    pub lua: Option<Utf8PathBuf>,
    pub working_dir: Utf8PathBuf,
}
impl From<GensContext> for EmulatorContext {
    fn from(value: GensContext) -> Self {
        Self::Gens(value)
    }
}

impl GensContext {
    /// Creates a new Context with default options.
    /// 
    /// If the path does not point to a directory, or a file within a directory, which contains `Gens.exe`,
    /// an error will be returned.
    pub fn new<P: Into<Utf8PathBuf>>(working_dir: P, version: GensVersion) -> Result<Self, Error> {
        let mut working_dir = working_dir.into();
        if working_dir.is_file() {
            working_dir.pop();
        }
        
        working_dir = working_dir.canonicalize_utf8().unwrap_or(working_dir);
        
        let mut detect_exe = working_dir.clone();
        detect_exe.push("Gens.exe");
        if working_dir.is_file() || !working_dir.exists() || !detect_exe.is_file() {
            return Err(Error::MissingExecutable(detect_exe));
        }
        
        Ok(Self {
            version,
            start_paused: false,
            rom: None,
            movie: None,
            lua: None,
            working_dir,
        })
    }
    
    pub fn with_pause(self, start_paused: bool) -> Self {
        Self {
            start_paused,
            ..self
        }
    }
    
    pub fn with_rom<P: Into<Utf8PathBuf>>(self, rom: P) -> Self {
        Self {
            rom: Some(rom.into()),
            ..self
        }
    }
    
    pub fn with_movie<P: Into<Utf8PathBuf>>(self, movie: P) -> Self {
        Self {
            movie: Some(movie.into()),
            ..self
        }
    }
    
    pub fn with_lua<P: Into<Utf8PathBuf>>(self, lua: P) -> Self {
        Self {
            lua: Some(lua.into()),
            ..self
        }
    }
    
    pub fn build_command(&mut self) -> Result<Command, Error> {
        // Gens has inconsistent requirements for where files exist
        
        if let Some(rom) = self.rom.as_ref() {
            if !rom.is_file() {
                return Err(Error::MissingRom(rom.clone()));
            }
            let mut dest = self.working_dir.clone();
            dest.push(rom.file_name().unwrap());
            
            std::fs::copy(rom, dest)?;
            
            self.rom = Some(rom.file_name().unwrap().into());
        }
        
        if let Some(movie) = self.movie.as_ref() {
            if !movie.is_file() {
                return Err(Error::MissingMovie(movie.clone()));
            }
            
            // movie path can be outside working dir, but must be absolute
            if !movie.is_absolute() {
                let mut dest = self.working_dir.clone();
                dest.push(movie.file_name().unwrap());
                
                std::fs::copy(movie, dest)?;
                
                self.movie = Some(movie.file_name().unwrap().into());
            }
        }
        
        if let Some(lua) = self.lua.as_ref() {
            if !lua.is_file() {
                return Err(Error::MissingLua(lua.clone()));
            }
            
            // lua path can be outside working dir, but must be absolute
            if !lua.is_absolute() {
                let mut dest = self.working_dir.clone();
                dest.push(lua.file_name().unwrap());
                
                std::fs::copy(lua, dest)?;
                
                self.lua = Some(lua.file_name().unwrap().into());
            }
        }
        
        let mut cmd = Command::new(self.cmd_name());
        cmd.args(self.args())
            .envs(self.envs())
            .current_dir(&self.working_dir);
        
        Ok(cmd)
    }
    
    pub fn version(&self) -> GensVersion {
        self.version
    }
    
    fn cmd_name(&self) -> &'static str {
        #[cfg(target_family = "unix")]
        { "wine".into() }
        
        #[cfg(target_family = "windows")]
        { "Gens.exe".into() }
    }
    
    fn args(&self) -> Vec<String> {
        let mut args = Vec::with_capacity(5);
        
        #[cfg(target_family = "unix")]
        {
            args.push(self.working_dir.join("Gens.exe").to_string());
        }
        
        use GensVersion::*;
        match self.version {
            Ver11A | Ver11B | GitA2425B5 | Unknown => { // TODO: verify for correctness
                if self.start_paused {
                    args.push("-pause".into());
                    args.push("0".into());
                }
                if let Some(rom) = self.rom.as_ref() {
                    args.push("-rom".into());
                    args.push(rom.to_string());
                }
                if let Some(movie) = self.movie.as_ref() {
                    args.push("-play".into());
                    args.push(movie.to_string());
                }
                if let Some(lua) = self.lua.as_ref() {
                    args.push("-lua".into());
                    args.push(lua.to_string());
                }
            },
        }
        
        args
    }
    
    fn envs(&self) -> Vec<(String, String)> {
        let mut vars = vec![];

        #[cfg(target_family = "unix")]
        {
            let prefix = self.working_dir.join(".wine/");
            
            vars.push(("WINEPREFIX".into(), prefix.to_string()));
        }
        
        vars
    }
}