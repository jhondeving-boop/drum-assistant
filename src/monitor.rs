use battery::units::ratio::percent;
use battery::{State, Battery};
use notify_rust::Notification;
use std::time::{Duration, Instant};
use crate::audio::{AudioPaths, play_sound};

pub struct BatteryMonitor {
    estado_anterior: State,
    ultimo_aviso_baja: Option<Instant>,
    ultimo_aviso_cargada: Option<Instant>,
    paths: AudioPaths,
}

impl BatteryMonitor {
    pub fn new() -> Self {
        Self {
            estado_anterior: State::Unknown,
            ultimo_aviso_baja: None,
            ultimo_aviso_cargada: None,
            paths: AudioPaths::new(),
        }
    }

    /// Inicializa el estado para evitar alertas falsas al arrancar
    pub fn init(&mut self, bateria: &Battery) {
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
    pub fn procesar_ciclo(&mut self, bateria: &Battery) {
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
                play_sound(&self.paths.conectado);
                self.ultimo_aviso_baja = None; // Reset alerta baja
                true
            },
            State::Discharging => {
                // Solo notificar si antes estaba cargando (ignorar parpadeos o unknown)
                if self.estado_anterior == State::Charging || self.estado_anterior == State::Full {
                    println!("🔋 Cargador Desconectado");
                    Self::notificar("Energía", "Usando batería");
                    play_sound(&self.paths.desconectado);
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
                play_sound(&self.paths.baja);
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
                play_sound(&self.paths.cargada);
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

    fn notificar(titulo: &str, cuerpo: &str) {
        let _ = Notification::new()
            .summary(titulo)
            .body(cuerpo)
            .icon("battery")
            .show();
    }
}
