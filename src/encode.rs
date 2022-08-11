use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::Write;
use clap::ArgMatches;
use log::info;

pub fn handle(matches: &ArgMatches) {
    let inputs: Vec<PathBuf> = matches.values_of_lossy("input").unwrap()
        .iter().map(|input| input.into()).collect();
    let mut output = PathBuf::from(matches.value_of("output").unwrap());
    let trim = matches.value_of("trim");
    
    if inputs.len() == 1 {
        let result = Command::new("ffmpeg")
            .args(&["-i", &inputs[0].to_string_lossy(), "-vcodec", "h264", "-preset", "superfast", "-b:v", "5000K", "-acodec", "libmp3lame", "-ac", "1", "-b:a", "160k"])
            .arg(output.as_os_str())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();
        info!("{:?}", result);
    } else {
        let mut tmp = File::create("tmp.txt").unwrap();
        for input in inputs {
            writeln!(tmp, "file '{}'", input.to_string_lossy()).unwrap();
        }
        tmp.flush().unwrap();
        
        let result = Command::new("ffmpeg")
            .args(&["-f", "concat", "-safe", "0", "-i", &Path::new("tmp.txt").to_string_lossy(), "-vcodec", "h264", "-preset", "superfast", "-b:v", "5000K", "-acodec", "libmp3lame", "-ac", "1", "-b:a", "160k"])
            .arg(output.as_os_str())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();
        info!("{:?}", result);
    }
    
    if output.is_file() && trim.is_some() {
        let input = output.clone();
        let ext = input.extension().unwrap();
        output.set_file_name(format!("{}-trimmed", output.file_stem().unwrap().to_string_lossy()));
        output.set_extension(ext);
        
        let result = Command::new("ffmpeg")
            .args(&["-i", &input.to_string_lossy(), "-to", trim.unwrap(), "-c", "copy"])
            .arg(output.as_os_str())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();
        info!("{:?}", result);
    }
}