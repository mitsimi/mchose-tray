use crate::platform;
use std::{fmt, fs, io::Write, path::PathBuf};

const MAX_LOG_SIZE: u64 = 256 * 1024;

pub(crate) enum Level {
    Info,
    Warn,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        })
    }
}

pub(crate) fn write(level: Level, message: &str) {
    let path = log_path();
    rotate_if_needed(&path);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{} [{level}] {message}", platform::local_timestamp());
    }
}

fn log_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("app.log")))
        .unwrap_or_else(|| PathBuf::from("app.log"))
}

fn rotate_if_needed(path: &PathBuf) {
    if path
        .metadata()
        .is_ok_and(|metadata| metadata.len() > MAX_LOG_SIZE)
    {
        let previous = path.with_extension("log.old");
        let _ = fs::remove_file(&previous);
        let _ = fs::rename(path, previous);
    }
}
