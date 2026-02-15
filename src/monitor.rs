use crate::audio::{AudioPaths, play_sound};
use crate::config::AppConfig;
use crate::logger;
use battery::units::ratio::percent;
use battery::{Battery, State};
use notify_rust::Notification;
use std::time::{Duration, Instant};

pub struct BatteryMonitor {
    estado_anterior: State,
    ultimo_aviso_baja: Option<Instant>,
    ultimo_aviso_cargada: Option<Instant>,
    umbral_baja: f32,
    umbral_alta: f32,
    cooldown: Duration,
    paths: AudioPaths,
}

impl BatteryMonitor {
    pub fn new(config: AppConfig) -> Self {
        Self {
            estado_anterior: State::Unknown,
            ultimo_aviso_baja: None,
            ultimo_aviso_cargada: None,
            umbral_baja: config.umbral_baja,
            umbral_alta: config.umbral_alta,
            cooldown: Duration::from_secs(config.cooldown_segundos),
            paths: AudioPaths::new(),
        }
    }

    /// Inicializa el estado para evitar alertas falsas al arrancar
    pub fn init(&mut self, bateria: &Battery) {
        self.estado_anterior = bateria.state();
        let porcentaje = bateria.state_of_charge().get::<percent>();

        // Si arrancamos en estado crítico, marcamos como "ya avisado" para esperar 1 min
        if porcentaje <= self.umbral_baja {
            self.ultimo_aviso_baja = Some(Instant::now());
        }
        if porcentaje >= self.umbral_alta {
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
            }
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
            }
            _ => false,
        }
    }

    /// Verifica si se deben emitir alertas de nivel
    fn verificar_niveles(&mut self, estado: State, porcentaje: f32) {
        // A. Bateria Baja
        if debe_alertar_baja(estado, porcentaje, self.umbral_baja) {
            if self.debe_avisar(self.ultimo_aviso_baja) {
                println!("⚠️ Batería Crítica ({:.0}%)", porcentaje);
                Self::notificar(
                    "Batería Baja",
                    &format!("Nivel crítico: {:.0}%. Conecta el cargador.", porcentaje),
                );
                play_sound(&self.paths.baja);
                self.ultimo_aviso_baja = Some(Instant::now());
            }
        } else if estado == State::Charging {
            self.ultimo_aviso_baja = None;
        }

        // B. Carga Alta
        if debe_alertar_alta(estado, porcentaje, self.umbral_alta) {
            if self.debe_avisar(self.ultimo_aviso_cargada) {
                println!("✅ Carga Suficiente ({:.0}%)", porcentaje);
                Self::notificar(
                    "Carga Suficiente",
                    &format!("Nivel: {:.0}%. Desconecta el cargador.", porcentaje),
                );
                play_sound(&self.paths.cargada);
                self.ultimo_aviso_cargada = Some(Instant::now());
            }
        } else if estado == State::Discharging {
            self.ultimo_aviso_cargada = None;
        }
    }

    fn debe_avisar(&self, ultimo_aviso: Option<Instant>) -> bool {
        cooldown_vencido(ultimo_aviso, Instant::now(), self.cooldown)
    }

    fn notificar(titulo: &str, cuerpo: &str) {
        if let Err(err) = Notification::new()
            .summary(titulo)
            .body(cuerpo)
            .icon("battery")
            .show()
        {
            logger::warn(&format!("No se pudo mostrar notificacion: {err}"));
        }
    }
}

fn cooldown_vencido(ultimo_aviso: Option<Instant>, ahora: Instant, cooldown: Duration) -> bool {
    match ultimo_aviso {
        None => true,
        Some(instante) => ahora.duration_since(instante) >= cooldown,
    }
}

fn debe_alertar_baja(estado: State, porcentaje: f32, umbral_baja: f32) -> bool {
    estado == State::Discharging && porcentaje <= umbral_baja
}

fn debe_alertar_alta(estado: State, porcentaje: f32, umbral_alta: f32) -> bool {
    (estado == State::Charging || estado == State::Full) && porcentaje >= umbral_alta
}

#[cfg(test)]
mod tests {
    use super::{cooldown_vencido, debe_alertar_alta, debe_alertar_baja};
    use battery::State;
    use std::time::{Duration, Instant};

    #[test]
    fn umbral_bajo_es_inclusivo() {
        assert!(debe_alertar_baja(State::Discharging, 20.0, 20.0));
        assert!(!debe_alertar_baja(State::Charging, 10.0, 20.0));
    }

    #[test]
    fn umbral_alto_es_inclusivo() {
        assert!(debe_alertar_alta(State::Charging, 80.0, 80.0));
        assert!(debe_alertar_alta(State::Full, 80.0, 80.0));
        assert!(!debe_alertar_alta(State::Discharging, 95.0, 80.0));
    }

    #[test]
    fn cooldown_no_repite_antes_de_tiempo() {
        let base = Instant::now();
        let ahora = base + Duration::from_secs(59);
        assert!(!cooldown_vencido(
            Some(base),
            ahora,
            Duration::from_secs(60)
        ));
    }

    #[test]
    fn cooldown_repite_al_cumplir_tiempo() {
        let base = Instant::now();
        let ahora = base + Duration::from_secs(60);
        assert!(cooldown_vencido(Some(base), ahora, Duration::from_secs(60)));
    }
}
