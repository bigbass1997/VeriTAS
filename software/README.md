[![License: BSD 2-Clause](https://img.shields.io/badge/License-BSD%202--Clause-blue)](LICENSE)
### Replaying
One function of this software is to interface with the VeriTAS replay device hardware. This interface allows
the user to stream or upload input data intended for replays, or to manually feed controller inputs on-the-fly.
Testing and status functions will also be available.

### Encoding
Intended for personal use, there are a few commands to assist with transcoding video recordings, including
combining multi-file footage into one video, and trimming the end.

### Dumping
The VeriTAS software also includes a dump automation tool for TAS (Tool-Assisted-Superplays/Speedruns) movies. After some configuration, this tool will allow you to automatically dump either a local movie file, or a TASVideos publication/submission, for use in console verifications.

Dump scripts are provided automatically by the tool. Configuration consists of providing paths to emulators and game roms.

#### Dump Format
The lua scripts currently dump to an unreleased WIP dump format (.tasd). *Thus, this tool won't be useful for the public until the dump format is finalized.* At that time, [TASD-Edit](https://github.com/bigbass1997/TASD-Edit) will also be available to convert these dumps back to legacy formats.

### Building
If you wish to build from source, for your own system, Rust is integrated with the `cargo` build system. To install Rust and `cargo`, just follow [these instructions](https://doc.rust-lang.org/cargo/getting-started/installation.html). Once installed, while in the project directory, run `cargo build --release` to build, or use `cargo run --release` to run directly. The built binary will be available at `./target/release/veritas`

To cross-compile builds for other operating systems, you can use [rust-embedded/cross](https://github.com/rust-embedded/cross).