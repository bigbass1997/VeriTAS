use std::io::Write;
use log::Level::*;
use env_logger::Builder;
use env_logger::fmt::Color::*;

pub fn builder() -> Builder {
    let mut builder = Builder::new();
    
    builder.format(|f, record| {
        let time = f.timestamp_millis().to_string().split_once("T").unwrap_or_default().1.replace("Z", "");
        
        let mut style = f.style();
        let level = match record.level() {
            Trace => style.set_color(Magenta).value("TRACE"),
            Debug => style.set_color(Blue).value("DEBUG"),
            Info => style.set_color(Green).value("INFO"),
            Warn => style.set_color(Yellow).value("WARN"),
            Error => style.set_color(Red).value("ERROR"),
        };
        
        let mut style = f.style();
        let target = style.set_bold(true).value(record.target());
        
        writeln!(f, "[{}][{}] {} > {}",
            time,
            level,
            target,
            record.args()
        )
    });
    
    builder
}