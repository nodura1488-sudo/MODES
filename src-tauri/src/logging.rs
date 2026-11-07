use chrono::Local;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

pub fn log_path() -> PathBuf {
    crate::store::app_data_dir().join("logs").join("launcher.log")
}

pub fn init() {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    rotate(&path);
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = LOG_FILE.set(Mutex::new(file));
    }

    log(format!("TMStore {} started", env!("CARGO_PKG_VERSION")));

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log(format!("PANIC: {info}"));
        default_hook(info);
    }));
}

fn rotate(path: &PathBuf) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() < 2 * 1024 * 1024 {
        return;
    }
    let old = path.with_file_name("launcher.old.log");
    let _ = fs::remove_file(&old);
    let _ = fs::rename(path, old);
}

pub fn log(message: impl AsRef<str>) {
    let Some(lock) = LOG_FILE.get() else {
        return;
    };
    let Ok(mut file) = lock.lock() else {
        return;
    };
    let time = Local::now().format("%Y-%m-%d %H:%M:%S");
    let _ = writeln!(file, "[{time}] {}", message.as_ref());
    let _ = file.flush();
}

pub fn error(context: &str, message: impl std::fmt::Display) {
    log(format!("ERROR {context}: {message}"));
}
