use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn advertir(mensaje: &str) {
    eprintln!("WARN: {mensaje}");

    let Some(ruta) = ruta_log() else {
        return;
    };

    if let Some(carpeta) = ruta.parent() {
        let _ = fs::create_dir_all(carpeta);
    }

    if let Ok(mut archivo) = OpenOptions::new().create(true).append(true).open(&ruta) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(archivo, "[{ts}] WARN: {mensaje}");
    }
}

fn ruta_log() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".local/state/battery-assistant/battery-assistant.log"))
}
