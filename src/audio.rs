use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::thread;

pub struct AudioPaths {
    pub conectado: PathBuf,
    pub desconectado: PathBuf,
    pub baja: PathBuf,
    pub cargada: PathBuf,
}

impl AudioPaths {
    pub fn new() -> Self {
        // Orden de prioridad:
        // 1. /usr/share (Sistema Instalado)
        // 2. assets/ (Desarrollo)
        // 3. Junto al ejecutable (Portable)
        let system_path = PathBuf::from("/usr/share/battery-assistant");
        let current_dir = std::env::current_dir().unwrap_or_default();
        let assets_dir = current_dir.join("assets");
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();

        // Determinar la base buscando un archivo clave
        let base = if system_path.join("conectado.mp3").exists() {
            system_path
        } else if assets_dir.join("conectado.mp3").exists() {
            assets_dir
        } else if exe_dir.join("conectado.mp3").exists() {
            exe_dir
        } else {
            current_dir
        };

        Self {
            conectado: base.join("conectado.mp3"),
            desconectado: base.join("desconectado.mp3"),
            baja: base.join("baja.mp3"),
            cargada: base.join("cargada.mp3"),
        }
    }
}

pub fn play_sound(ruta: &PathBuf) {
    let ruta_archivo = ruta.clone();
    thread::spawn(move || {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                if let Ok(file) = File::open(&ruta_archivo) {
                    if let Ok(source) = Decoder::new(BufReader::new(file)) {
                        sink.append(source);
                        sink.sleep_until_end();
                    }
                }
            }
        }
    });
}
