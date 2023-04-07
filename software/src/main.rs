use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
use log::LevelFilter;
use crate::config::{SaveLoad, VeritasConfig};

mod config;
mod dumping;
mod encode;
mod logger;
mod replay;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(long, short)]
    pub config: Option<PathBuf>,
    
    #[arg(long, short)]
    pub useragent: Option<String>,
    
    #[arg(long, short)]
    pub verbose: Option<LevelFilter>,
    
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Encode(EncodeArgs),
    Dump(DumpArgs),
    Replay(ReplayArgs),
}

#[derive(Debug, Parser)]
pub struct EncodeArgs {
    #[arg(long, short)]
    pub trim: Option<String>,
    
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
    
    pub output: PathBuf,
}

#[derive(Debug, Parser)]
pub struct DumpArgs {
    #[arg(long, short)]
    pub fetch: Vec<String>,
    #[arg(long)]
    pub local: Option<PathBuf>,
    #[arg(long = "override", value_name = "OVERRIDE")]
    pub local_override: Option<PathBuf>,
    #[arg(long)]
    pub refresh: bool,
}

#[derive(Debug, Parser)]
pub struct ReplayArgs {
    #[arg(long, short)]
    pub movie: Option<PathBuf>,
    
    #[arg(long, short)]
    pub device: Option<String>,
    
    #[arg(long)]
    pub list_devices: bool,
    
    #[arg(long)]
    pub manual: bool,
}

fn main() {
    let args = Args::parse();
    
    let mut config = VeritasConfig::load("veritas-default.toml");
    
    
    if let Some(path) = args.config {
        config = VeritasConfig::load(path);
    }
    
    // Setup program-wide logger format
    {
        let mut logbuilder = logger::builder();
        logbuilder.filter_level(args.verbose.unwrap_or(LevelFilter::Info));
        logbuilder.init();
    }
    
    let useragent = args.useragent.unwrap_or(config.useragent.clone());
    tasvideos_api_rs::set_user_agent(Box::leak(useragent.into_boxed_str())); // safe as long as `useragent` variable exists till end of program
    
    match args.command {
        Command::Encode(args) => encode::handle(args),
        Command::Dump(args) => {
            initialize_cache();
            dumping::handle(args, config.dumper)
        },
        Command::Replay(args) => replay::handle(args),
    }
}

fn initialize_cache() {
    std::fs::create_dir("cache").unwrap_or_default();
    std::fs::create_dir("cache/movies").unwrap_or_default();
    create_missing_file("cache/tasd-api.lua", include_bytes!("includes/tasd-api.lua"));
    create_missing_file("cache/tasd-fceux.lua", include_bytes!("includes/tasd-fceux.lua"));
    create_missing_file("cache/tasd-bizhawk.lua", include_bytes!("includes/tasd-bizhawk.lua"));
    create_missing_file("cache/config-bizhawk.ini", include_bytes!("includes/config-bizhawk.ini"));
}

fn create_missing_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) {
    let path = path.as_ref();
    if !path.exists() {
        std::fs::write(path, contents).unwrap_or_default();
    }
}