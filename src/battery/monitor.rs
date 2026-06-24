use super::types::{BatteryData, BatteryState};
use std::time::{Duration, Instant};

/// Configuración de umbrales y cooldown para un monitor de batería.
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub low_threshold: f32,
    pub high_threshold: f32,
    pub cooldown: Duration,
}

impl MonitorConfig {
    pub fn new(low_threshold: f32, high_threshold: f32, cooldown_secs: u64) -> Self {
        Self {
            low_threshold,
            high_threshold,
            cooldown: Duration::from_secs(cooldown_secs),
        }
    }
}

/// Evento de alerta producido por `BatteryMonitor::process_cycle`.
/// El caller (main.rs) decide qué hacer con cada evento (notificar, audio, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum AlertEvent {
    ChargerConnected,
    ChargerDisconnected,
    LowBattery {
        percentage: f32,
        time_to_empty: Option<Duration>,
    },
    HighBattery {
        percentage: f32,
        time_to_full: Option<Duration>,
    },
}

/// Máquina de estados por batería.
/// Lógica pura: recibe `BatteryData`, emite `Vec<AlertEvent>`.
/// No tiene side effects, lo que la hace fácil de testear.
#[derive(Debug)]
pub struct BatteryMonitor {
    previous_state: BatteryState,
    last_low_alert: Option<Instant>,
    last_high_alert: Option<Instant>,
    config: MonitorConfig,
    initialized: bool,
}

impl BatteryMonitor {
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            previous_state: BatteryState::Unknown,
            last_low_alert: None,
            last_high_alert: None,
            config,
            initialized: false,
        }
    }

    /// Indica si `initialize()` ya fue llamado.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Inicializa el monitor con el primer estado conocido.
    /// Suprime alertas falsas al arranque marcando los umbrales como "ya notificados".
    pub fn initialize(&mut self, data: &BatteryData) {
        self.previous_state = data.state;

        if self.previous_state == BatteryState::Unknown {
            if data.time_to_full.is_some() {
                self.previous_state = BatteryState::Charging;
            } else if data.time_to_empty.is_some() {
                self.previous_state = BatteryState::Discharging;
            }
        }

        if data.percentage <= self.config.low_threshold {
            self.last_low_alert = Some(Instant::now());
        }
        if data.percentage >= self.config.high_threshold {
            self.last_high_alert = Some(Instant::now());
        }

        self.initialized = true;
    }

    /// Ciclo principal de evaluación.
    /// 1. Detecta cambio de estado (conectado/desconectado) → evento prioritario.
    /// 2. Si no hubo cambio, verifica niveles (batería baja / carga completa).
    /// Retorna lista de eventos a procesar.
    pub fn process_cycle(&mut self, data: &BatteryData) -> Vec<AlertEvent> {
        let mut events = Vec::new();

        let state_changed =
            data.state != BatteryState::Unknown && data.state != self.previous_state;

        if state_changed {
            if let Some(event) = self.handle_state_change(data.state) {
                events.push(event);
            }
            self.previous_state = data.state;
        }

        if events.is_empty() {
            events.extend(self.check_levels(data));
        }

        events
    }

    /// Calcula el intervalo de polling dinámico para ahorrar CPU.
    /// Retorna 5s si está cerca de un umbral, 30s si está en zona segura.
    pub fn dynamic_sleep(&self, data: &BatteryData) -> Duration {
        let pct = data.percentage;
        let near_low = pct <= self.config.low_threshold + 3.0;
        let near_high = pct >= self.config.high_threshold - 3.0;

        if near_low || near_high {
            Duration::from_secs(5)
        } else {
            Duration::from_secs(30)
        }
    }

    /// Maneja transiciones de estado del cargador.
    fn handle_state_change(&mut self, new_state: BatteryState) -> Option<AlertEvent> {
        match new_state {
            BatteryState::Charging
                if self.previous_state == BatteryState::Discharging
                    || self.previous_state == BatteryState::Unknown =>
            {
                self.last_low_alert = None;
                Some(AlertEvent::ChargerConnected)
            }
            BatteryState::Discharging
                if self.previous_state == BatteryState::Charging
                    || self.previous_state == BatteryState::Full =>
            {
                self.last_high_alert = None;
                Some(AlertEvent::ChargerDisconnected)
            }
            _ => None,
        }
    }

    /// Verifica si se deben emitir alertas de nivel (bajo o alto).
    /// Respeta el cooldown entre notificaciones.
    fn check_levels(&mut self, data: &BatteryData) -> Vec<AlertEvent> {
        let mut events = Vec::new();

        if should_alert_low(data.state, data.percentage, self.config.low_threshold) {
            if cooldown_expired(self.last_low_alert, self.config.cooldown) {
                events.push(AlertEvent::LowBattery {
                    percentage: data.percentage,
                    time_to_empty: data.time_to_empty,
                });
                self.last_low_alert = Some(Instant::now());
            }
        } else if data.state == BatteryState::Charging {
            self.last_low_alert = None;
        }

        if should_alert_high(data.state, data.percentage, self.config.high_threshold) {
            if cooldown_expired(self.last_high_alert, self.config.cooldown) {
                events.push(AlertEvent::HighBattery {
                    percentage: data.percentage,
                    time_to_full: data.time_to_full,
                });
                self.last_high_alert = Some(Instant::now());
            }
        } else if data.state == BatteryState::Discharging {
            self.last_high_alert = None;
        }

        events
    }
}

/// Retorna `true` si la batería está descargando y su porcentaje es <= al umbral.
pub fn should_alert_low(state: BatteryState, percentage: f32, threshold: f32) -> bool {
    state == BatteryState::Discharging && percentage <= threshold
}

/// Retorna `true` si la batería está cargando/llena y su porcentaje es >= al umbral.
pub fn should_alert_high(state: BatteryState, percentage: f32, threshold: f32) -> bool {
    (state == BatteryState::Charging || state == BatteryState::Full) && percentage >= threshold
}

/// Retorna `true` si el cooldown ha expirado desde la última alerta.
fn cooldown_expired(last_alert: Option<Instant>, cooldown: Duration) -> bool {
    match last_alert {
        None => true,
        Some(instant) => instant.elapsed() >= cooldown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battery(pct: f32, state: BatteryState) -> BatteryData {
        BatteryData {
            state,
            percentage: pct,
            time_to_full: None,
            time_to_empty: None,
        }
    }

    #[test]
    fn initialize_sets_previous_state() {
        let mut monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        monitor.initialize(&battery(50.0, BatteryState::Discharging));
        assert_eq!(monitor.previous_state, BatteryState::Discharging);
    }

    #[test]
    fn initialize_suppresses_initial_low_alert() {
        let mut monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        monitor.initialize(&battery(15.0, BatteryState::Discharging));
        assert!(monitor.last_low_alert.is_some());
    }

    #[test]
    fn connecting_charger_emits_event() {
        let mut monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        monitor.initialize(&battery(50.0, BatteryState::Discharging));

        let events = monitor.process_cycle(&battery(50.0, BatteryState::Charging));
        assert_eq!(events, vec![AlertEvent::ChargerConnected]);
    }

    #[test]
    fn disconnecting_charger_emits_event() {
        let mut monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        monitor.initialize(&battery(50.0, BatteryState::Charging));

        let events = monitor.process_cycle(&battery(50.0, BatteryState::Discharging));
        assert_eq!(events, vec![AlertEvent::ChargerDisconnected]);
    }

    #[test]
    fn low_battery_alert() {
        let mut monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        monitor.initialize(&battery(50.0, BatteryState::Discharging));

        let events = monitor.process_cycle(&battery(15.0, BatteryState::Discharging));
        assert_eq!(
            events,
            vec![AlertEvent::LowBattery {
                percentage: 15.0,
                time_to_empty: None
            }]
        );
    }

    #[test]
    fn high_battery_alert() {
        let mut monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        monitor.initialize(&battery(50.0, BatteryState::Charging));

        let events = monitor.process_cycle(&battery(85.0, BatteryState::Charging));
        assert_eq!(
            events,
            vec![AlertEvent::HighBattery {
                percentage: 85.0,
                time_to_full: None
            }]
        );
    }

    #[test]
    fn no_duplicate_events_within_cooldown() {
        let mut monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        monitor.initialize(&battery(50.0, BatteryState::Discharging));

        let events1 = monitor.process_cycle(&battery(15.0, BatteryState::Discharging));
        assert!(!events1.is_empty());

        let events2 = monitor.process_cycle(&battery(14.0, BatteryState::Discharging));
        assert!(events2.is_empty());
    }

    #[test]
    fn unknown_state_does_not_trigger_state_change_event() {
        let mut monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        monitor.initialize(&battery(50.0, BatteryState::Charging));

        let events = monitor.process_cycle(&battery(50.0, BatteryState::Unknown));
        assert!(events.is_empty());
    }

    #[test]
    fn dynamic_sleep_returns_long_in_safe_zone() {
        let monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        assert_eq!(monitor.dynamic_sleep(&battery(50.0, BatteryState::Discharging)), Duration::from_secs(30));
    }

    #[test]
    fn dynamic_sleep_returns_short_near_threshold() {
        let monitor = BatteryMonitor::new(MonitorConfig::new(20.0, 80.0, 60));
        assert_eq!(monitor.dynamic_sleep(&battery(22.0, BatteryState::Discharging)), Duration::from_secs(5));
        assert_eq!(monitor.dynamic_sleep(&battery(78.0, BatteryState::Charging)), Duration::from_secs(5));
    }

    #[test]
    fn should_alert_low_checks_state_and_threshold() {
        assert!(should_alert_low(BatteryState::Discharging, 20.0, 20.0));
        assert!(!should_alert_low(BatteryState::Charging, 20.0, 20.0));
        assert!(!should_alert_low(BatteryState::Discharging, 21.0, 20.0));
    }

    #[test]
    fn should_alert_high_checks_state_and_threshold() {
        assert!(should_alert_high(BatteryState::Charging, 80.0, 80.0));
        assert!(should_alert_high(BatteryState::Full, 80.0, 80.0));
        assert!(!should_alert_high(BatteryState::Discharging, 80.0, 80.0));
    }
}
