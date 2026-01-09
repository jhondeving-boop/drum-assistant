use battery::units::ratio::percent;
use battery::{Manager, State};
use notify_rust::Notification;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

/// Gestiona las rutas de los archivos de audio con fallbacks
struct AudioPaths {
    conectado: PathBuf,
    desconectado: PathBuf,
    baja: PathBuf,
    cargada: PathBuf,
}

impl AudioPaths {
    fn new() -> Self {
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

/// Monitor de estado de batería
struct BatteryMonitor {
    estado_anterior: State,
    ultimo_aviso_baja: Option<Instant>,
    ultimo_aviso_cargada: Option<Instant>,
    paths: AudioPaths,
}

impl BatteryMonitor {
    fn new() -> Self {
        Self {
            estado_anterior: State::Unknown,
            ultimo_aviso_baja: None,
            ultimo_aviso_cargada: None,
            paths: AudioPaths::new(),
        }
    }

    /// Inicializa el estado para evitar alertas falsas al arrancar
    fn init(&mut self, bateria: &battery::Battery) {
        self.estado_anterior = bateria.state();
        let porcentaje = bateria.state_of_charge().get::<percent>();
        
        // Si arrancamos en estado crítico, marcamos como "ya avisado" para esperar 1 min
        if porcentaje <= 20.0 {
            self.ultimo_aviso_baja = Some(Instant::now());
        }
        if porcentaje >= 80.0 {
            self.ultimo_aviso_cargada = Some(Instant::now());
        }
    }

    /// Ciclo principal de verificación
    fn procesar_ciclo(&mut self, bateria: &battery::Battery) {
        let estado_actual = bateria.state();
        let porcentaje = bateria.state_of_charge().get::<percent>();
        let mut hubo_evento_prioritario = false;

        // 1. Verificar cambio de cable (Prioridad Alta)
        if estado_actual != self.estado_anterior {
            hubo_evento_prioritario = self.manejar_cambio_estado(estado_actual);
            self.estado_anterior = estado_actual;
        }

        // 2. Verificar niveles (solo si no hubo cambio de cable reciente para evitar choques)
        if !hubo_evento_prioritario {
            self.verificar_niveles(estado_actual, porcentaje);
        }
    }

    /// Maneja la conexión/desconexión del cargador
    fn manejar_cambio_estado(&mut self, estado: State) -> bool {
        match estado {
            State::Charging => {
                println!("🔌 Cargador Conectado");
                Self::notificar("Energía", "Cargador conectado");
                Self::reproducir(&self.paths.conectado);
                self.ultimo_aviso_baja = None; // Reset alerta baja
                true
            },
            State::Discharging => {
                // Solo notificar si antes estaba cargando (ignorar parpadeos o unknown)
                if self.estado_anterior == State::Charging || self.estado_anterior == State::Full {
                    println!("🔋 Cargador Desconectado");
                    Self::notificar("Energía", "Usando batería");
                    Self::reproducir(&self.paths.desconectado);
                    self.ultimo_aviso_cargada = None; // Reset alerta cargada
                    true
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    /// Verifica si se deben emitir alertas de nivel
    fn verificar_niveles(&mut self, estado: State, porcentaje: f32) {
        // A. Batería Baja (< 20%)
        if estado == State::Discharging && porcentaje <= 20.0 {
            if self.debe_avisar(self.ultimo_aviso_baja) {
                println!("⚠️ Batería Crítica ({:.0}%)", porcentaje);
                Self::notificar("Batería Baja", &format!("Nivel crítico: {:.0}%. Conecta el cargador.", porcentaje));
                Self::reproducir(&self.paths.baja);
                self.ultimo_aviso_baja = Some(Instant::now());
            }
        } else if estado == State::Charging {
            self.ultimo_aviso_baja = None;
        }

        // B. Carga Alta (> 80%)
        if (estado == State::Charging || estado == State::Full) && porcentaje >= 80.0 {
            if self.debe_avisar(self.ultimo_aviso_cargada) {
                println!("✅ Carga Suficiente ({:.0}%)", porcentaje);
                Self::notificar("Carga Suficiente", &format!("Nivel: {:.0}%. Desconecta el cargador.", porcentaje));
                Self::reproducir(&self.paths.cargada);
                self.ultimo_aviso_cargada = Some(Instant::now());
            }
        } else if estado == State::Discharging {
            self.ultimo_aviso_cargada = None;
        }
    }

    fn debe_avisar(&self, ultimo_aviso: Option<Instant>) -> bool {
        match ultimo_aviso {
            None => true,
            // Repetir cada 60 segundos
            Some(instante) => instante.elapsed() >= Duration::from_secs(60),
        }
    }

    fn reproducir(ruta: &PathBuf) {
        let ruta_archivo = ruta.clone();
        thread::spawn(move || {
            if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
                if let Ok(sink) = Sink::try_new(&stream_handle) {
                    if let Ok(file) = File::open(&ruta_archivo) {
                        if let Ok(source) = Decoder::new(BufReader::new(file)) {
                            sink.append(source);
                            sink.sleep_until_end();
                        }
                    } else {
                         // Fallo silencioso o log de debug si fuera necesario
                         // eprintln!("Error audio: No encontrado {:?}", ruta_archivo);
                    }
                }
            }
        });
    }

    fn notificar(titulo: &str, cuerpo: &str) {
        let _ = Notification::new()
            .summary(titulo)
            .body(cuerpo)
            .icon("battery")
            .show();
    }
}

fn main() -> Result<(), battery::Error> {
    let manager = Manager::new()?;
    let mut monitor = BatteryMonitor::new();
    let mut es_inicio = true;

    println!("🔊 Asistente de batería activado...");

    loop {
        // Iteramos sobre las baterías (normalmente solo hay 1)
        if let Some(Ok(battery)) = manager.batteries()?.next() {
            if es_inicio {
                monitor.init(&battery);
                es_inicio = false;
            } else {
                monitor.procesar_ciclo(&battery);
            }
        }

        // Revisamos cada 5 segundos (eficiente en CPU)
        thread::sleep(Duration::from_secs(5));
    }
}
