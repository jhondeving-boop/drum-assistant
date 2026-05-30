use crate::audio::{EventoAudio, reproducir_sonido};
use crate::config::ConfigApp;
use crate::logger;
use battery::units::ratio::percent;
use battery::units::time::minute;
use battery::{Battery, State};
use notify_rust::{Notification, Timeout, Urgency};
use std::time::{Duration, Instant};

/// Gestiona el estado y las alertas para una batería específica
pub struct MonitorBateria {
    estado_anterior: State,
    ultimo_aviso_baja: Option<Instant>,
    ultimo_aviso_cargada: Option<Instant>,
    umbral_baja: f32,
    umbral_alta: f32,
    cooldown: Duration,
}

impl MonitorBateria {
    pub fn new(config: ConfigApp) -> Self {
        Self {
            estado_anterior: State::Unknown,
            ultimo_aviso_baja: None,
            ultimo_aviso_cargada: None,
            umbral_baja: config.umbral_baja,
            umbral_alta: config.umbral_alta,
            cooldown: Duration::from_secs(config.cooldown_segundos),
        }
    }

    /// Inicializa el estado para evitar alertas falsas al arrancar
    pub fn inicializar(&mut self, bateria: &Battery) {
        self.estado_anterior = bateria.state();
        let porcentaje = bateria.state_of_charge().get::<percent>();

        // Si arrancamos en estado crítico, marcamos como "ya avisado" para esperar al cooldown
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

        // Ignorar el estado 'Unknown' si ya teníamos un estado válido. 
        // Esto evita que el programa se confunda si el Kernel de Linux parpadea la lectura 1 milisegundo.
        if estado_actual != State::Unknown && estado_actual != self.estado_anterior {
            hubo_evento_prioritario = self.manejar_cambio_estado(estado_actual);
            self.estado_anterior = estado_actual;
        }

        // 2. Verificar niveles (solo si no hubo cambio reciente para evitar cruces de notificaciones)
        if !hubo_evento_prioritario {
            self.verificar_niveles(bateria, estado_actual, porcentaje);
        }
    }

    /// Calcula el tiempo dinámico para dormir el hilo y ahorrar CPU
    /// Si está lejos de los umbrales, duerme más tiempo. Si está cerca, despierta más rápido.
    pub fn tiempo_espera_dinamico(&self, bateria: &Battery) -> Duration {
        let porcentaje = bateria.state_of_charge().get::<percent>();

        // Si la batería está en rango de alerta o muy cerca (a 3% de los límites),
        // revisamos cada 5 segundos para precisión exacta.
        if (porcentaje <= self.umbral_baja + 3.0) || (porcentaje >= self.umbral_alta - 3.0) {
            Duration::from_secs(5)
        } else {
            // Zona segura (ej: entre 25% y 75%). Dormimos 30 segundos ahorrando ciclos de CPU.
            Duration::from_secs(30)
        }
    }

    /// Maneja la conexión/desconexión del cargador (Alta prioridad)
    fn manejar_cambio_estado(&mut self, estado: State) -> bool {
        match estado {
            State::Charging => {
                // Solo anunciamos que se conectó si antes estaba descargándose o al inicio (Unknown)
                // Evitamos avisar si solo bajó de "Full" (100%) a "Charging" (99%) mientras sigue conectado.
                if self.estado_anterior == State::Discharging || self.estado_anterior == State::Unknown {
                    println!("🔌 Cargador Conectado");
                    Self::notificar("Energía", "Cargador conectado al sistema", Urgency::Normal);
                    reproducir_sonido(EventoAudio::Conectado);
                }
                self.ultimo_aviso_baja = None; // Siempre reseteamos la alerta de batería baja
                true
            }
            State::Discharging if self.estado_anterior == State::Charging || self.estado_anterior == State::Full => {
                // Solo notificar si antes estaba cargando/lleno (ignorar lecturas erróneas 'Unknown')
                println!("🔋 Cargador Desconectado");
                Self::notificar("Energía", "Usando energía de la batería", Urgency::Normal);
                reproducir_sonido(EventoAudio::Desconectado);
                self.ultimo_aviso_cargada = None; // Reseteamos la alerta de carga alta
                true
            }
            _ => false,
        }
    }

    /// Verifica si se deben emitir alertas de nivel e incorpora inteligencia (tiempo restante)
    fn verificar_niveles(&mut self, bateria: &Battery, estado: State, porcentaje: f32) {
        // A. Bateria Baja
        if debe_alertar_baja(estado, porcentaje, self.umbral_baja) {
            if self.debe_avisar(self.ultimo_aviso_baja) {
                // Feature Premium: Estimar tiempo restante
                let tiempo_texto = match bateria.time_to_empty() {
                    Some(tiempo) => format!(
                        " Tiempo estimado restante: {:.0} min.",
                        tiempo.get::<minute>()
                    ),
                    None => String::new(),
                };

                let mensaje = format!(
                    "Nivel crítico: {:.0}%. Conecta el cargador.{}",
                    porcentaje, tiempo_texto
                );
                println!("⚠️ Batería Crítica ({:.0}%)", porcentaje);
                Self::notificar("Batería Baja", &mensaje, Urgency::Critical);
                reproducir_sonido(EventoAudio::BateriaBaja);

                self.ultimo_aviso_baja = Some(Instant::now());
            }
        } else if estado == State::Charging {
            self.ultimo_aviso_baja = None; // Si enchufó pero sigue en rango bajo, cancelamos alertas
        }

        // B. Carga Alta
        if debe_alertar_alta(estado, porcentaje, self.umbral_alta) {
            if self.debe_avisar(self.ultimo_aviso_cargada) {
                // Feature Premium: Estimar tiempo para carga completa
                let tiempo_texto = match bateria.time_to_full() {
                    Some(tiempo) if tiempo.get::<minute>() > 0.0 => {
                        format!(" Faltan {:.0} min para el 100%.", tiempo.get::<minute>())
                    }
                    _ => String::new(),
                };

                let mensaje = format!(
                    "Nivel alcanzado: {:.0}%. Desconecta el cargador para cuidar la vida útil.{}",
                    porcentaje, tiempo_texto
                );
                println!("✅ Carga Suficiente ({:.0}%)", porcentaje);
                Self::notificar("Carga Suficiente", &mensaje, Urgency::Normal);
                reproducir_sonido(EventoAudio::BateriaCargada);

                self.ultimo_aviso_cargada = Some(Instant::now());
            }
        } else if estado == State::Discharging {
            self.ultimo_aviso_cargada = None;
        }
    }

    /// Comprueba si ha pasado el tiempo de gracia (cooldown) desde el último aviso
    fn debe_avisar(&self, ultimo_aviso: Option<Instant>) -> bool {
        cooldown_vencido(ultimo_aviso, Instant::now(), self.cooldown)
    }

    /// Envia notificación de escritorio
    fn notificar(titulo: &str, cuerpo: &str, urgencia: Urgency) {
        let mut notif = Notification::new();
        notif
            .summary(titulo)
            .body(cuerpo)
            .icon("battery") // Intenta usar icono nativo del sistema
            .urgency(urgencia);

        // Si es crítica (Hyprland / Mako), forzamos que no desaparezca sola
        if urgencia == Urgency::Critical {
            notif.timeout(Timeout::Never);
        }

        if let Err(err) = notif.show() {
            logger::advertir(&format!("No se pudo mostrar la notificación: {}", err));
        }
    }
}

// --- Funciones auxiliares puras (fáciles de testear) ---

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
