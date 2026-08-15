/// File-based logging for dsh-agent.
///
/// Writes log lines to `<install dir>\logs\dsh-agent.log` so that crashes,
/// node command failures and connection errors are easy to inspect.
/// If the install dir is not writable (e.g. Program Files for a non-elevated
/// process), falls back to `<AppData>\com.dsh-agent.app\logs`.
use log::{Level, Log, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static LOGS_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Writes log records to a single append-only file.
pub struct FileLogger {
    file: Mutex<File>,
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!("[{}] [{}] {}\n", ts, record.level(), record.args());
        // try_lock: never block (and never deadlock) on the logging mutex
        if let Ok(mut f) = self.file.try_lock() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }

    fn flush(&self) {
        if let Ok(mut f) = self.file.try_lock() {
            let _ = f.flush();
        }
    }
}

/// Locate the logs directory: `<exe dir>\logs` if writable, else AppData.
fn detect_logs_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let dir = parent.join("logs");
            if std::fs::create_dir_all(&dir).is_ok() {
                let probe = dir.join(".write_probe");
                if std::fs::write(&probe, b"").is_ok() {
                    let _ = std::fs::remove_file(&probe);
                    return dir;
                }
            }
        }
    }
    let fallback = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.dsh-agent.app")
        .join("logs");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// Resolve (and cache) the logs directory.
pub fn logs_dir() -> PathBuf {
    if let Some(dir) = LOGS_DIR.get() {
        return dir.clone();
    }
    let dir = detect_logs_dir();
    let _ = LOGS_DIR.set(dir.clone());
    dir
}

/// Install the file logger and a panic hook that records crashes.
/// Returns the logs directory.
pub fn init() -> PathBuf {
    let dir = logs_dir();
    let path = dir.join("dsh-agent.log");
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
        let logger = Box::new(FileLogger {
            file: Mutex::new(file),
        });
        let _ = log::set_boxed_logger(logger);
        log::set_max_level(log::LevelFilter::Info);
    }
    log::info!(
        "========== DSH Agent {} started ==========",
        env!("CARGO_PKG_VERSION")
    );

    // Record panics directly to the log file (bypass the logger to avoid
    // deadlocking if the panic happens while the logger holds its mutex).
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "?".to_string());
        let backtrace = std::backtrace::Backtrace::force_capture();
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let entry = format!(
            "[{}] [ERROR] ===== PANIC at {} =====\n{}\nBacktrace:\n{}\n",
            ts, location, payload, backtrace
        );
        let dir = logs_dir();
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("dsh-agent.log"))
        {
            let _ = f.write_all(entry.as_bytes());
            let _ = f.flush();
        }
    }));

    dir
}

/// Tauri command: return the logs directory path (for the frontend "view logs" button).
#[tauri::command]
pub fn get_logs_dir() -> Result<String, String> {
    Ok(logs_dir().to_string_lossy().to_string())
}
