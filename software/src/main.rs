use camino::Utf8PathBuf;
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
    pub inputs: Vec<Utf8PathBuf>,
    
    pub output: Utf8PathBuf,
}

#[derive(Debug, Parser)]
pub struct DumpArgs {
    #[arg(long, short, num_args = 1..)]
    pub fetch: Vec<String>,
    
    #[arg(long = "override", value_name = "OVERRIDE")]
    pub rom_override: Option<Utf8PathBuf>,
    
    #[arg(long)]
    pub refresh: bool,
    
    #[arg(long, hide = true)]
    pub all: bool,
}

#[derive(Debug, Parser)]
pub struct ReplayArgs {
    #[arg(long, short)]
    pub movie: Option<Utf8PathBuf>,
    
    #[arg(long, short)]
    pub device: Option<String>,
    
    #[arg(long)]
    pub list_devices: bool,
    
    #[arg(long)]
    pub manual: Option<String>,
    
    #[arg(long)]
    pub latch_filter: Option<u32>,
    
    #[arg(long)]
    pub disable_reset: bool,
}

fn main() {
    let args = Args::parse();
    
    let config = VeritasConfig::load();
    
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
        Command::Dump(args) => dumping::handle(args, config.dumper),
        Command::Replay(args) => replay::handle(args),
    }
}
