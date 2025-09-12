use std::process::Command;
use camino::Utf8PathBuf;
use crate::contexts::EmulatorContext;
use crate::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct FceuxContext {
    pub version: String,
    pub config: Option<Utf8PathBuf>,
    pub movie: Option<Utf8PathBuf>,
    pub lua: Option<Utf8PathBuf>,
    pub rom: Option<Utf8PathBuf>,
    
    /// If set, forces Old (false) or New (true) PPU mode.
    /// 
    /// **Note:** Only used when executable is `fceux` (linux binary) or `qfceux.exe`
    pub ppu_mode: Option<bool>,
    pub working_dir: Utf8PathBuf,
}
impl From<FceuxContext> for EmulatorContext {
    fn from(value: FceuxContext) -> Self {
        Self::Fceux(value)
    }
}
impl FceuxContext {
    /// Creates a new Context with default options.
    /// 
    /// If the path does not point to a directory, or a file within a directory, which contains a valid FCEUX executable,
    /// an error will be returned.
    pub fn new<P: Into<Utf8PathBuf>, S: Into<String>>(working_dir: P, version: S) -> Result<Self, Error> {
        let mut working_dir = working_dir.into();
        if working_dir.is_file() {
            working_dir.pop();
        }
        
        working_dir = working_dir.canonicalize_utf8().unwrap_or(working_dir);
        
        if working_dir.is_file() || !working_dir.exists() {
            return Err(Error::MissingExecutable(working_dir));
        }
        
        let mut found = false;
        for exe in ["fceux.exe", "fceux64.exe", "qfceux.exe", "fceux"] {
            let mut path = working_dir.clone();
            path.push(exe);
            
            if path.is_file() {
                found = true;
                break;
            }
        }
        if !found {
            let mut path = working_dir.clone();
            path.push("fceux");
            return Err(Error::MissingExecutable(path));
        }
        
        Ok(Self {
            version: version.into(),
            config: None,
            movie: None,
            lua: None,
            rom: None,
            ppu_mode: None,
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
    
    pub fn with_ppu_mode(self, ppu_mode: bool) -> Self {
        Self {
            ppu_mode: Some(ppu_mode),
            ..self
        }
    }
    
    pub fn build_command(&mut self) -> Result<Command, Error> {
        // FCEUX accepts configs/movies/scripts/roms from anywhere,
        // so we only need to verify they exist.
        // However, since we change the working directory, and there's no
        // easy way to test if file exists relative to a different dir,
        // the paths _should_ be absolute, either originally or via the with_* functions.
        
        #[cfg(target_family = "windows")]
        {
            if cmd_name == "./fceux" {
                return Err(Error::IncompatibleOSVersion);
            }
        }
        
        if let Some(config) = self.config.as_ref() {
            // Preparing the config file is extremely messy.
            // - win32/win64 provides a CLI argument that is used.
            // - win64-QtSLD uses the fceux.cfg located beside the executable.
            // - compiled linux builds use $HOME/.fceux/fceux.cfg.
            //     (if $HOME isn't set, it's unclear what FCEUX does)
            
            if !config.is_file() {
                return Err(Error::MissingConfig(config.clone()));
            }
            
            if let Some(config) = self.config.as_ref() {
                if !config.is_file() {
                    return Err(Error::MissingConfig(config.clone()));
                }
                
                if let Some(exe) = self.determine_executable() {
                    let mut dest = self.working_dir.clone();
                    if exe == "fceux" {
                        dest.push(".fceux/");
                        if !dest.is_dir() {
                            std::fs::create_dir_all(&dest)?;
                        }
                        dest.push("fceux.cfg");
                        
                        std::fs::copy(config, dest)?;
                    } else if exe == "qfceux.exe" {
                        dest.push("fceux.cfg");
                        
                        std::fs::copy(config, dest)?;
                    } else if !config.is_absolute() {
                        return Err(Error::AbsolutePathFailed);
                    }
                }
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
        
        let mut cmd = Command::new(self.cmd_name());
        cmd.args(self.args())
            .envs(self.envs())
            .current_dir(&self.working_dir);
        
        Ok(cmd)
    }
    
    pub fn version(&self) -> &str {
        &self.version
    }
    
    fn cmd_name(&self) -> &'static str {
        #[cfg(target_family = "unix")]
        {
            match self.determine_executable() {
                Some(exe) if exe == "fceux" => "./fceux",
                Some(_) => "wine",
                None => "./fceux",
            }
        }
        
        #[cfg(target_family = "windows")]
        {
            match self.determine_executable() {
                Some(exe) => exe,
                None => "fceux.exe",
            }
        }
    }
    
    fn args(&self) -> Vec<String> {
        let mut args = Vec::with_capacity(5);
        
        #[cfg(target_family = "unix")]
        {
            if self.cmd_name() == "wine" {
                args.push(self.determine_executable().unwrap().to_string());
            }
        }
        
        match self.determine_executable() {
            Some(exe) => match exe {
                "fceux.exe" | "fceux64.exe" => {
                    if let Some(config) = self.config.as_ref() {
                        args.push("-cfg".into());
                        args.push(config.to_string());
                    }
                    if let Some(movie) = self.movie.as_ref() {
                        args.push("-playmovie".into());
                        args.push(movie.to_string());
                    }
                    if let Some(lua) = self.lua.as_ref() {
                        args.push("-lua".into());
                        args.push(lua.to_string());
                    }
                },
                "fceux" | "qfceux.exe" => {
                    if let Some(movie) = self.movie.as_ref() {
                        args.push("--playmov".into());
                        args.push(movie.to_string());
                    }
                    if let Some(lua) = self.lua.as_ref() {
                        args.push("--loadlua".into());
                        args.push(lua.to_string());
                    }
                    if let Some(ppu_mode) = self.ppu_mode.as_ref() {
                        args.push("--newppu".into());
                        args.push(if *ppu_mode { "1".into() } else { "0".into() });
                    }
                },
                _ => ()
            },
            None => ()
        }
        
        if let Some(rom) = self.rom.as_ref() {
            args.push(rom.to_string());
        }
        
        args
    }
    
    fn envs(&self) -> Vec<(String, String)> {
        let mut vars = vec![];

        #[cfg(target_family = "unix")]
        {
            if self.cmd_name() == "wine" {
                let prefix = self.working_dir.join(".wine/");
                
                vars.push(("WINEPREFIX".into(), prefix.to_string()));
            }
        }
        
        let home = self.working_dir.join(".fceux/");
        vars.push(("HOME".into(), home.to_string()));
        
        vars
    }
    
    fn determine_executable(&self) -> Option<&'static str> {
        let mut path = self.working_dir.clone();
        path.push("fceux");
        if path.is_file() {
            return Some("fceux")
        }
        
        path.pop();
        path.push("fceux.exe");
        if path.is_file() {
            return Some("fceux.exe")
        }
        
        path.pop();
        path.push("fceux64.exe");
        if path.is_file() {
            return Some("fceux64.exe")
        }
        
        path.pop();
        path.push("qfceux.exe");
        if path.is_file() {
            return Some("qfceux.exe")
        }
        
        None
    }
}