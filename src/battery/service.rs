use super::types::{BatteryData, BatteryState};
use battery::Manager;
use log::warn;
use std::time::Duration;

/// Abstracción sobre el origen de datos de batería.
/// Permite testear el monitor sin hardware real.
pub trait BatteryService {
    /// Retorna el estado actual de todas las baterías del sistema.
    fn batteries(&mut self) -> Result<Vec<BatteryData>, String>;
}

/// Implementación real que lee baterías desde sysfs vía el crate `battery`.
pub struct SysfsBatteryService;

impl SysfsBatteryService {
    pub fn new() -> Self {
        Self
    }
}

impl BatteryService for SysfsBatteryService {
    fn batteries(&mut self) -> Result<Vec<BatteryData>, String> {
        let manager = Manager::new().map_err(|e| format!("battery manager: {e}"))?;
        let batteries = manager.batteries().map_err(|e| format!("battery list: {e}"))?;

        let mut result = Vec::new();
        for battery_result in batteries {
            match battery_result {
                Ok(battery) => {
                    let percentage =
                        battery.state_of_charge().get::<battery::units::ratio::percent>();

                    let state = match battery.state() {
                        battery::State::Charging => BatteryState::Charging,
                        battery::State::Discharging => BatteryState::Discharging,
                        battery::State::Full => BatteryState::Full,
                        battery::State::Empty => BatteryState::Empty,
                        _ => BatteryState::Unknown,
                    };

                    let time_to_full = battery
                        .time_to_full()
                        .map(|t| Duration::from_secs(t.get::<battery::units::time::second>() as u64));
                    let time_to_empty = battery
                        .time_to_empty()
                        .map(|t| Duration::from_secs(t.get::<battery::units::time::second>() as u64));

                    result.push(BatteryData {
                        state,
                        percentage,
                        time_to_full,
                        time_to_empty,
                    });
                }
                Err(e) => {
                    warn!("Failed to read individual battery: {e}");
                }
            }
        }

        Ok(result)
    }
}
