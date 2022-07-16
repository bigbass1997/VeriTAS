[![License: BSD 2-Clause](https://img.shields.io/badge/License-BSD%202--Clause-blue)](LICENSE)
### Description
VeriTAS is a dump automation tool for TAS (Tool-Assisted-Superplays/Speedruns) movies. After some configuration, this tool will allow you to automatically dump either a local movie file, or a TASVideos publication/submission, for use in console verifications.

Dump scripts are provided automatically by the tool. Configuration consists of providing paths to emulators and game roms.

### Building
If you wish to build from source, for your own system, Rust is integrated with the `cargo` build system. To install Rust and `cargo`, just follow [these instructions](https://doc.rust-lang.org/cargo/getting-started/installation.html). Once installed, while in the project directory, run `cargo build --release` to build, or use `cargo run --release` to run directly. The built binary will be available at `./target/release/veritas`

To cross-compile builds for other operating systems, you can use [rust-embedded/cross](https://github.com/rust-embedded/cross).