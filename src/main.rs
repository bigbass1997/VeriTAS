use std::path::Path;
use clap::{AppSettings, Arg, Command};
use log::LevelFilter;
use crate::config::{Config, VeritasConfig};

mod config;
mod dumping;
mod encode;
mod logger;

fn main() {
    /*let config = Config::builder()
        .add_source(File::with_name("veritas-default.toml"))
        .set_default("rom_directory", "roms").unwrap()
        .set_default("fceux_path", "").unwrap()
        .set_default("bizhawk_path", "").unwrap()
        .build().unwrap();*/
    let mut config = VeritasConfig::load("veritas-default.toml");
    
    std::fs::create_dir("cache").unwrap_or_default();
    std::fs::create_dir("cache/movies").unwrap_or_default();
    create_missing_file("cache/tasd-api.lua", include_bytes!("includes/tasd-api.lua"));
    create_missing_file("cache/tasd-fceux.lua", include_bytes!("includes/tasd-fceux.lua"));
    create_missing_file("cache/tasd-bizhawk.lua", include_bytes!("includes/tasd-bizhawk.lua"));
    
    let matches = Command::new("VeriTAS")
        .arg(Arg::new("config")
            .takes_value(true)
            .short('c')
            .long("config")
            .help("Path to a VeriTAS config file. If omitted, veritas-default.toml will be used instead."))
        .subcommand(Command::new("encode")
            .arg(Arg::new("input")
                .takes_value(true)
                .required(true)
                .multiple_values(true)
                .help("Path(s) to video file(s)."))
            .arg(Arg::new("output")
                .takes_value(true)
                .required(true)
                .help("Output file path."))
            .arg(Arg::new("trim")
                .takes_value(true)
                .long("trim")
                .help("Optionally specify how much to trim the final video. Time format: HH:MM:SS")))
        .subcommand(Command::new("dump")
            .arg(Arg::new("fetch")
                .takes_value(true)
                .multiple_values(true)
                .allow_invalid_utf8(true)
                .short('f')
                .long("fetch")
                .help("Fetch one or more TASVideos publications and/or submissions. (e.g. 1234M or 1234S, for publication or submission respectively)"))
            /*.arg(Arg::new("local")
                .takes_value(true)
                .long("local")
                .help("Path to a local movie file."))*/
            .arg_required_else_help(true))
        .arg(Arg::new("verbose")
            .short('v')
            .long("verbose")
            .takes_value(true)
            .possible_values(["error", "warn", "info", "debug", "trace"])
            .help("Set the console log level. Environment variable 'RUST_LOG' will override this option."))
        .global_setting(AppSettings::DeriveDisplayOrder)
        .arg_required_else_help(true)
        .dont_collapse_args_in_usage(true)
        .get_matches();
    
    if let Some(path) = matches.value_of("config") {
        config = VeritasConfig::load(path);
    }
    
    // Setup program-wide logger format
    let level = match std::env::var("RUST_LOG").unwrap_or(matches.value_of("verbose").unwrap_or("info").to_owned()).as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info
    };
    {
        let mut logbuilder = logger::builder();
        logbuilder.filter_level(level);
        logbuilder.init();
    }
    
    tasvideos_api_rs::set_user_agent("Bigbass#9631");
    
    
    match matches.subcommand_name().unwrap_or_default() {
        "encode" => encode::handle(matches.subcommand_matches("encode").unwrap()),
        "dump" => dumping::handle(matches.subcommand_matches("dump").unwrap(), config.dumper.clone()),
        _ => ()
    }
}


fn create_missing_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) {
    let path = path.as_ref();
    if !path.exists() {
        std::fs::write(path, contents).unwrap_or_default();
    }
}