use std::time::Duration;

/// Estado operativo de una batería.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    Empty,
    Unknown,
}

/// Datos planos de una batería en un instante dado.
/// Se usa como DTO entre `BatteryService` y `BatteryMonitor`.
#[derive(Debug, Clone)]
pub struct BatteryData {
    pub state: BatteryState,
    pub percentage: f32,
    pub time_to_full: Option<Duration>,
    pub time_to_empty: Option<Duration>,
}
