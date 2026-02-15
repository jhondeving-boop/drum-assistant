mod audio;
mod config;
mod logger;
mod monitor;

use crate::config::AppConfig;
use crate::logger::warn;
use crate::monitor::BatteryMonitor;
use battery::Manager;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), battery::Error> {
    let manager = Manager::new()?;
    let config = AppConfig::load();
    let mut monitores: Vec<BatteryMonitor> = Vec::new();
    let mut aviso_sin_bateria_emitido = false;

    println!("🔊 Asistente de batería activado...");
    println!(
        "Configuracion: baja <= {:.0}%, alta >= {:.0}%, repeticion cada {}s",
        config.umbral_baja, config.umbral_alta, config.cooldown_segundos
    );

    loop {
        match manager.batteries() {
            Ok(baterias) => {
                let mut ok_index = 0usize;

                for battery_result in baterias {
                    match battery_result {
                        Ok(bateria) => {
                            ensure_monitor_slot(&mut monitores, ok_index, config, &bateria);
                            ok_index += 1;
                        }
                        Err(err) => {
                            warn(&format!("No se pudo leer el estado de una bateria: {err}"));
                        }
                    }
                }

                trim_to_active_monitors(&mut monitores, ok_index);

                if ok_index == 0 {
                    if !aviso_sin_bateria_emitido {
                        warn("No se detectaron baterias en el sistema.");
                        aviso_sin_bateria_emitido = true;
                    }
                } else {
                    aviso_sin_bateria_emitido = false;
                }
            }
            Err(err) => {
                warn(&format!("No se pudo enumerar baterias del sistema: {err}"));
            }
        }

        // Revisamos cada 5 segundos (eficiente en CPU)
        thread::sleep(Duration::from_secs(5));
    }
}

fn ensure_monitor_slot(
    monitores: &mut Vec<BatteryMonitor>,
    index: usize,
    config: AppConfig,
    bateria: &battery::Battery,
) {
    if monitores.len() <= index {
        let mut monitor = BatteryMonitor::new(config);
        monitor.init(bateria);
        monitores.push(monitor);
    } else {
        monitores[index].procesar_ciclo(bateria);
    }
}

fn trim_to_active_monitors<T>(items: &mut Vec<T>, active_count: usize) {
    if items.len() > active_count {
        items.truncate(active_count);
    }
}

#[cfg(test)]
mod tests {
    use super::trim_to_active_monitors;

    #[test]
    fn trim_to_active_monitors_reduces_excess_items() {
        let mut values = vec![1, 2, 3, 4];
        trim_to_active_monitors(&mut values, 2);
        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn trim_to_active_monitors_keeps_vector_when_equal_or_smaller() {
        let mut values = vec![1, 2];
        trim_to_active_monitors(&mut values, 2);
        assert_eq!(values, vec![1, 2]);

        trim_to_active_monitors(&mut values, 3);
        assert_eq!(values, vec![1, 2]);
    }
}
