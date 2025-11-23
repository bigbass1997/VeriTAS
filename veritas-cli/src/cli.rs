use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use tracing::level_filters::LevelFilter;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(long, short)]
    pub verbose: Option<LevelFilter>,
    
    /// Specifies the directory VeriTAS should use for its local cache. Default: `./cache/`
    #[arg(long)]
    pub cache: Option<Utf8PathBuf>,
    
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Dump(DumpArgs),
    Replay(ReplayArgs),
    Encode(EncodeArgs),
}

#[derive(Debug, Parser)]
pub struct EncodeArgs {
    #[arg(long)]
    pub upload: bool,
    
    #[arg(long, short)]
    pub trim: Option<String>,
    
    //#[arg(required = true)]
    pub inputs: Vec<Utf8PathBuf>,
    
    pub output: Option<Utf8PathBuf>,
}

#[derive(Debug, Parser)]
pub struct DumpArgs {
    /// Refreshes the ROM cache using all files found at the specified path.
    #[arg(short, long)]
    pub refresh: Option<Utf8PathBuf>,
    
    /// Adds to the list of available emulators for use when dumping.
    /// 
    /// Each entry is parsed as `path,version` where `path` is a file path to the root directory or
    /// executable of an emulator, and `version` is the version of the emulator.
    /// 
    /// Example:
    /// - `/path/to/bizhawk/,2.9.1` would register an emulator with version `2.9.1` located at `/path/to/bizhawk/`.
    #[arg(short, long, help = None, verbatim_doc_comment)]
    pub emulator: Vec<String>,
    
    /// The number of parallel threads used to dump movies.
    #[arg(short, long, default_value = "4")]
    pub threads: usize,
    
    /// List of movies to dump.
    /// 
    /// Each entry is parsed as `source[=override]` where `source` is one of the following:
    /// - Local: A local filepath to an existing file (excluding directories)
    /// - Userfile: A number, optionally prefixed with a `#` symbol (`#1234567890123456789`)
    /// - Submission: A number followed by the letter `S` or `s` (`1234S`)
    /// - Publication: A number followed by the letter `M` or `m` (`5678M`)
    /// 
    /// Optionally, an `=` followed by a file path may be included immediately after the `source` to
    /// force the dumper to use the provided file as the ROM for that movie.
    /// 
    /// Examples:
    /// - `5195M=/path/to/SuperfastMarioBros.nes` would download the 5195M publication from TASVideos,
    ///   and use a local ROM file (`SuperfastMarioBros.nes`) during the dump process.
    /// - `/path/to/movie.bk2` would retrieve the expected ROM hash from the movie, and look for a
    ///   matching ROM in the cache.
    #[arg(long, short, num_args = 1.., help = None, verbatim_doc_comment)]
    pub fetch: Vec<String>,
}
impl DumpArgs {
    pub fn fetch(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fetch
            .iter()
            .map(|s| s.split_once('=').unwrap_or((s, "")))
    }
}

#[derive(Debug, Parser)]
pub struct ReplayArgs {
    #[arg(long, short)]
    pub movie: Option<Utf8PathBuf>,
    
    #[arg(long, short)]
    pub device: Option<String>,
    
    #[arg(short, long)]
    pub list: bool,
    
    #[arg(long)]
    pub manual: Option<String>,
    
    #[arg(long)]
    pub latch_filter: Option<u32>,
    
    #[arg(long)]
    pub disable_reset: bool,
    
    #[arg(long)]
    pub n8auto: Option<Utf8PathBuf>,
}