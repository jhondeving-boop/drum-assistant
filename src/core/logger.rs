//! Logger global que escribe a stderr y a un archivo rotativo.
//!
//! Implementa el trait `log::Log` para que el resto del programa use
//! `log::info!()`, `log::warn!()`, `log::error!()`, etc.
//! La rotación trunca el archivo cuando supera los 5 MB.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024;

struct FileLogger {
    path: PathBuf,
}

impl log::Log for FileLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let level = record.level();
        let msg = record.args();

        eprintln!("{level}: {msg}");

        let Some(parent) = self.path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }

        let truncate = fs::metadata(&self.path)
            .map(|m| m.len() > MAX_LOG_SIZE)
            .unwrap_or(false);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .append(!truncate)
            .truncate(truncate)
            .open(&self.path)
        {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(file, "[{ts}] {level}: {msg}");
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

/// Inicializa el logger global. Debe llamarse una sola vez al inicio de `main()`.
pub fn init() {
    let path = log_path().unwrap_or_else(|| PathBuf::from("/tmp/battery-assistant.log"));

    let logger = Box::new(FileLogger { path });

    if log::set_logger(Box::leak(logger)).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

fn log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".local/state/battery-assistant/battery-assistant.log"),
    )
}
