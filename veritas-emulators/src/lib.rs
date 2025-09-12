use std::process::Output;
use camino::Utf8PathBuf;
use crate::contexts::{BizHawkContext, FceuxContext, GensContext};

pub mod configs;
pub mod contexts;

#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    MissingExecutable(Utf8PathBuf),
    MissingBash(Utf8PathBuf),
    MissingConfig(Utf8PathBuf),
    MissingRom(Utf8PathBuf),
    MissingMovie(Utf8PathBuf),
    MissingLua(Utf8PathBuf),
    IncompatibleEmuVersion,
    AbsolutePathFailed,
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::StdIo(value)
    }
}