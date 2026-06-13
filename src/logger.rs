use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Simple file logger that writes to %APPDATA%\vwi\vwi.log.
/// Also mirrors output to stdout/stderr so you still see it when running
/// from a terminal.

struct Logger {
    file: std::fs::File,
}

static LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

/// Open (or create) the log file and prepare the logger.
/// Should be called once at startup before any log_* calls.
pub fn init() {
    let mut path = dirs::config_dir()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    path.push("vwi");
    std::fs::create_dir_all(&path).ok();
    path.push("vwi.log");

    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => {
            *LOGGER.lock().unwrap() = Some(Logger { file });
        }
        Err(e) => {
            eprintln!("Failed to open log file {:?}: {}", path, e);
        }
    }
}

/// Log an informational message.
pub fn info(msg: &str) {
    println!("{}", msg);
    write("INFO", msg);
}

/// Log an error message.
pub fn error(msg: &str) {
    eprintln!("{}", msg);
    write("ERROR", msg);
}

fn write(level: &str, msg: &str) {
    if let Ok(mut guard) = LOGGER.lock() {
        if let Some(ref mut logger) = *guard {
            let line = format!("[{}] {}\n", level, msg);
            let _ = logger.file.write_all(line.as_bytes());
            let _ = logger.file.flush();
        }
    }
}
