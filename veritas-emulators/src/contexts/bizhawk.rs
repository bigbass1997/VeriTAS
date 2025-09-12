use std::process::Command;
use camino::Utf8PathBuf;
use include_dir::{include_dir, Dir};
use crate::contexts::EmulatorContext;
use crate::Error;

static INCLUDES: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/includes/bizhawk/");

#[derive(Debug, Clone, PartialEq)]
pub struct BizHawkContext {
    pub version: String,
    pub config: Option<Utf8PathBuf>,
    pub movie: Option<Utf8PathBuf>,
    pub lua: Option<Utf8PathBuf>,
    pub rom: Option<Utf8PathBuf>,
    pub working_dir: Utf8PathBuf,
}
impl From<BizHawkContext> for EmulatorContext {
    fn from(value: BizHawkContext) -> Self {
        Self::BizHawk(value)
    }
}

impl BizHawkContext {
    /// Creates a new Context with default options.
    /// 
    /// If the path does not point to a directory, or a file within a directory, which contains `EmuHawk.exe`,
    /// an error will be returned.
    pub fn new<P: Into<Utf8PathBuf>, S: Into<String>>(working_dir: P, version: S) -> Result<Self, Error> {
        let mut working_dir = working_dir.into();
        if working_dir.is_file() {
            working_dir.pop();
        }
        
        working_dir = working_dir.canonicalize_utf8().unwrap_or(working_dir);
        
        let mut detect_exe = working_dir.clone();
        detect_exe.push("EmuHawk.exe");
        if working_dir.is_file() || !working_dir.exists() || !detect_exe.is_file() {
            return Err(Error::MissingExecutable(detect_exe));
        }
        
        Ok(Self {
            version: version.into(),
            config: None,
            movie: None,
            lua: None,
            rom: None,
            working_dir,
        })
    }
    
    pub fn with_config<P: Into<Utf8PathBuf>>(self, config: P) -> Self {
        let config = config.into();
        Self {
            config: Some(config.canonicalize_utf8().unwrap_or_else(|_| config)),
            ..self
        }
    }
    
    pub fn with_movie<P: Into<Utf8PathBuf>>(self, movie: P) -> Self {
        let movie = movie.into();
        Self {
            movie: Some(movie.canonicalize_utf8().unwrap_or_else(|_| movie)),
            ..self
        }
    }
    
    pub fn with_lua<P: Into<Utf8PathBuf>>(self, lua: P) -> Self {
        let lua = lua.into();
        Self {
            lua: Some(lua.canonicalize_utf8().unwrap_or_else(|_| lua)),
            ..self
        }
    }
    
    pub fn with_rom<P: Into<Utf8PathBuf>>(self, rom: P) -> Self {
        let rom = rom.into();
        Self {
            rom: Some(rom.canonicalize_utf8().unwrap_or_else(|_| rom)),
            ..self
        }
    }
    
    pub fn build_command(&mut self) -> Result<Command, Error> {
        // BizHawk accepts configs/movies/scripts/roms from anywhere,
        // so we only need to verify they exist.
        // However, since we change the working directory, and there's no
        // easy way to test if file exists relative to a different dir,
        // the paths _should_ be absolute, either originally or via the with_* functions.
        
        if let Some(config) = self.config.as_ref() {
            if !config.is_file() {
                return Err(Error::MissingConfig(config.clone()));
            }
            if !config.is_absolute() {
                return Err(Error::AbsolutePathFailed);
            }
        }
        if let Some(movie) = self.movie.as_ref() {
            if !movie.is_file() {
                return Err(Error::MissingMovie(movie.clone()));
            }
            if !movie.is_absolute() {
                return Err(Error::AbsolutePathFailed);
            }
        }
        if let Some(lua) = self.lua.as_ref() {
            if !lua.is_file() {
                return Err(Error::MissingLua(lua.clone()));
            }
            if !lua.is_absolute() {
                return Err(Error::AbsolutePathFailed);
            }
        }
        if let Some(rom) = self.rom.as_ref() {
            if !rom.is_file() {
                return Err(Error::MissingRom(rom.clone()));
            }
            if !rom.is_absolute() {
                return Err(Error::AbsolutePathFailed);
            }
        }
        
        // If unix, copy bash script and check for incompatible versions
        #[cfg(target_family = "unix")]
        {
            let bash = match self.version.as_str() {
                "2.8" | "2.8-rc1"
                    | "2.7"
                    | "2.6.3" | "2.6.2" | "2.6.1" | "2.6"
                    | "2.5.2" | "2.5.1" | "2.5.0" => INCLUDES.get_file("start-bizhawk-pre290.sh").expect("file should exist in binary"),
                
                "2.4.2" | "2.4.1" | "2.4" | "2.3.3"
                    | "2.3.2" | "2.3.1" | "2.3" | "2.2.2" | "2.2.1" | "2.2" | "2.1.1"
                    | "2.1.0" | "1.13.2" | "1.9.2" | "1.6.1" => return Err(Error::IncompatibleEmuVersion),
                
                _ => INCLUDES.get_file("start-bizhawk.sh").expect("file should exist in binary"),
            };
            
            let path = self.working_dir.join("start-bizhawk.sh");
            if !path.exists() {
                std::fs::write(path, bash.contents())?;
            }
        }
        
        let mut cmd = Command::new(self.cmd_name());
        cmd.args(self.args())
            .current_dir(&self.working_dir);
        
        Ok(cmd)
    }
    
    pub fn version(&self) -> &str {
        &self.version
    }
    
    fn cmd_name(&self) -> &'static str {
        #[cfg(target_family = "unix")]
        { "bash" }
        
        #[cfg(target_family = "windows")]
        { "EmuHawk.exe" }
    }
    
    fn args(&self) -> Vec<String> {
        let mut args = Vec::with_capacity(5);
        
        #[cfg(target_family = "unix")]
        {
            args.push("start-bizhawk.sh".into());
        }
        
        if let Some(config) = self.config.as_ref() {
            args.push(format!("--config={config}"));
        }
        if let Some(movie) = self.movie.as_ref() {
            args.push(format!("--movie={movie}"));
        }
        if let Some(lua) = self.lua.as_ref() {
            args.push(format!("--lua={lua}"));
        }
        if let Some(rom) = self.rom.as_ref() {
            args.push(rom.to_string());
        }
        
        args
    }
}
