use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;
use log::info;
use crate::EncodeArgs;

pub fn handle(args: EncodeArgs) {
    let inputs = args.inputs;
    let mut output = args.output;
    
    if inputs.len() == 1 {
        let result = Command::new("ffmpeg")
            .args(&["-i", inputs[0].as_str(), "-vcodec", "h264", "-preset", "superfast", "-b:v", "12000K", "-acodec", "libmp3lame", "-ac", "1", "-b:a", "160k"])
            .arg(output.as_os_str())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();
        info!("{:?}", result);
    } else {
        let mut tmp = File::create("tmp.txt").unwrap();
        for input in inputs {
            writeln!(tmp, "file '{input}'").unwrap();
        }
        tmp.flush().unwrap();
        
        let result = Command::new("ffmpeg")
            .args(&["-f", "concat", "-safe", "0", "-i", &Path::new("tmp.txt").to_string_lossy(), "-vcodec", "h264", "-preset", "superfast", "-b:v", "12000K", "-acodec", "libmp3lame", "-ac", "1", "-b:a", "160k"])
            .arg(output.as_os_str())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();
        info!("{:?}", result);
    }
    
    if output.is_file() && args.trim.is_some() {
        let input = output.clone();
        let ext = input.extension().unwrap();
        output.set_file_name(format!("{}-trimmed", output.file_stem().unwrap()));
        output.set_extension(ext);
        
        let result = Command::new("ffmpeg")
            .args(&["-i", input.as_str(), "-to", &args.trim.unwrap(), "-c", "copy"])
            .arg(output.as_os_str())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();
        info!("{:?}", result);
    }
}