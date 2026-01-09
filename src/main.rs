use battery::units::ratio::percent;
use battery::State;
use notify_rust::Notification;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::{thread, time};

fn main() -> Result<(), battery::Error> {
    let manager = battery::Manager::new()?;
    
    // --- 1. DEFINICIÓN DE LAS 4 RUTAS DE AUDIO ---
    // Buscar archivos de audio en orden de prioridad:
    // 1. /usr/share/battery-assistant (instalación del sistema)
    // 2. Junto al ejecutable
    // 3. Directorio actual (desarrollo)
    
    let system_path = std::path::PathBuf::from("/usr/share/battery-assistant");
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    let current_dir = std::env::current_dir().unwrap_or_default();
    let assets_dir = current_dir.join("assets");
    
    let audio_base = if system_path.join("conectado.mp3").exists() {
        system_path
    } else if assets_dir.join("conectado.mp3").exists() {
        assets_dir
    } else if exe_dir.join("conectado.mp3").exists() {
        exe_dir
    } else {
        current_dir
    };
    
    let path_conectado    = audio_base.join("conectado.mp3");
    let path_desconectado = audio_base.join("desconectado.mp3");
    let path_baja         = audio_base.join("baja.mp3");    // < 20%
    let path_cargada      = audio_base.join("cargada.mp3"); // > 95%

    // Variables de estado para controlar la lógica
    let mut estado_anterior = State::Unknown;
    let mut primera_vez = true;
    
    // Banderas para no repetir el audio de "Batería baja" o "Cargada" cada 2 segundos
    let mut ya_aviso_baja = false;
    let mut ya_aviso_cargada = false;

    println!("🔊 Asistente de batería con 4 eventos iniciado...");

    loop {
        if let Some(Ok(battery)) = manager.batteries()?.next() {
            let estado_actual = battery.state();
            let porcentaje = battery.state_of_charge().get::<percent>();

            // --- LÓGICA DE INICIO (Evitar falsas alarmas al abrir la app) ---
            if primera_vez {
                estado_anterior = estado_actual;
                primera_vez = false;
                // Si arrancamos y ya está baja, marcamos para no gritar inmediatamente
                if porcentaje <= 20.0 { ya_aviso_baja = true; }
                if porcentaje >= 95.0 { ya_aviso_cargada = true; }
            }

            // --- 2. DETECTAR CAMBIO DE CABLE (Conectado/Desconectado) ---
            if estado_actual != estado_anterior {
                match estado_actual {
                    State::Charging => {
                        println!("🔌 Cargador Conectado");
                        enviar_notificacion("Energía", "Cargador conectado");
                        reproducir_audio(&path_conectado);
                        
                        // Reseteamos la bandera de batería baja porque ya la pusimos a cargar
                        ya_aviso_baja = false; 
                    },
                    State::Discharging => {
                        // Solo sonar si venía de estar cargando (ignora parpadeos)
                        if estado_anterior == State::Charging || estado_anterior == State::Full {
                            println!("🔋 Cargador Desconectado");
                            enviar_notificacion("Energía", "Usando batería");
                            reproducir_audio(&path_desconectado);
                            
                            // Reseteamos la bandera de carga completa
                            ya_aviso_cargada = false;
                        }
                    },
                    _ => {}
                }
                estado_anterior = estado_actual;
            }

            // --- 3. DETECTAR NIVELES CRÍTICOS (Baja/Cargada) ---
            
            // CASO A: Batería Baja (< 20%) y descargando
            if estado_actual == State::Discharging && porcentaje <= 20.0 {
                if !ya_aviso_baja {
                    println!("⚠️ Batería Crítica");
                    enviar_notificacion("Batería Baja", &format!("Nivel crítico: {:.0}%", porcentaje));
                    reproducir_audio(&path_baja);
                    ya_aviso_baja = true; // Marcamos para no repetir hasta que se cargue de nuevo
                }
            }

            // CASO B: Batería Cargada (> 95%) y cargando
            // Nota: State::Full a veces no se activa en todos los laptops, por eso usamos > 95%
            if (estado_actual == State::Charging || estado_actual == State::Full) && porcentaje >= 95.0 {
                if !ya_aviso_cargada {
                    println!("✅ Carga Completa");
                    enviar_notificacion("Carga Completa", "Batería lista para desconectar");
                    reproducir_audio(&path_cargada);
                    ya_aviso_cargada = true; // Marcamos para no repetir
                }
            }
        }

        // Revisamos cada 2 segundos
        thread::sleep(time::Duration::from_secs(2));
    }
}

// --- Funciones auxiliares ---

fn reproducir_audio(ruta: &std::path::PathBuf) {
    let ruta_archivo = ruta.clone();
    thread::spawn(move || {
        // Manejo de errores silencioso para no cerrar la app si falta un archivo
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                if let Ok(file) = File::open(&ruta_archivo) {
                    // Decodificamos el MP3
                    if let Ok(source) = Decoder::new(BufReader::new(file)) {
                        sink.append(source);
                        sink.sleep_until_end();
                    }
                } else {
                    eprintln!("Error: No se encontró el archivo de audio: {:?}", ruta_archivo);
                }
            }
        }
    });
}

fn enviar_notificacion(titulo: &str, cuerpo: &str) {
    let _ = Notification::new()
        .summary(titulo)
        .body(cuerpo)
        .icon("battery") // Icono genérico del sistema
        .show();
}
