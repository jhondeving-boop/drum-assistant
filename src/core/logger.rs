use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_SIZE_BYTES: u64 = 5 * 1024 * 1024; // 5 MB

pub fn advertir(mensaje: &str) {
    eprintln!("WARN: {mensaje}");

    let Some(ruta) = ruta_log() else {
        return;
    };

    if let Some(carpeta) = ruta.parent() {
        let _ = fs::create_dir_all(carpeta);
    }

    // Rotación básica: Si el archivo es mayor a 5MB, lo truncamos (borramos el contenido viejo)
    let truncate = if let Ok(metadata) = fs::metadata(&ruta) {
        metadata.len() > MAX_LOG_SIZE_BYTES
    } else {
        false
    };

    if let Ok(mut archivo) = OpenOptions::new()
        .create(true)
        .write(true)
        .append(!truncate)
        .truncate(truncate)
        .open(&ruta)
    {
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
