mod audio;
mod config;
mod logger;
mod monitor;

use crate::config::ConfigApp;
use crate::logger::advertir;
use crate::monitor::MonitorBateria;
use battery::Manager;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), battery::Error> {
    // Inicializar el gestor de batería, lee de /sys/class/power_supply en Linux
    let manager = Manager::new()?;
    // Cargar la configuración desde ~/.config/battery-assistant/config.toml
    let config = ConfigApp::cargar();

    // Almacena el estado interno de cada batería conectada al sistema (normalmente solo 1)
    let mut monitores: Vec<MonitorBateria> = Vec::new();
    let mut aviso_sin_bateria_emitido = false;

    println!("🔊 Asistente de Batería activado (Modo Premium / Bajo Consumo)...");
    println!(
        "Configuración: Baja <= {:.0}%, Alta >= {:.0}%, Repetición cada {}s",
        config.umbral_baja, config.umbral_alta, config.cooldown_segundos
    );

    loop {
        // Asignamos un tiempo por defecto seguro en caso de error,
        // pero luego calcularemos dinámicamente el menor tiempo de espera.
        let mut sleep_duration = Duration::from_secs(30);

        match manager.batteries() {
            Ok(baterias) => {
                let mut ok_index = 0usize;

                for battery_result in baterias {
                    match battery_result {
                        Ok(bateria) => {
                            // Inicializa el monitor si es nuevo o procesa el ciclo si ya existe
                            asegurar_monitor_en_indice(&mut monitores, ok_index, config, &bateria);

                            // Obtenemos el tiempo recomendado de sleep para esta batería
                            // (si está cerca de un umbral, pedirá despertar en 5s en lugar de 30s)
                            let current_sleep =
                                monitores[ok_index].tiempo_espera_dinamico(&bateria);
                            if current_sleep < sleep_duration {
                                sleep_duration = current_sleep;
                            }

                            ok_index += 1;
                        }
                        Err(err) => {
                            advertir(&format!(
                                "No se pudo leer el estado de una bateria: {}",
                                err
                            ));
                        }
                    }
                }

                // Limpiar monitores de baterías que fueron desconectadas físicamente
                recortar_monitores_activos(&mut monitores, ok_index);

                // Advertir solo una vez si el equipo es de escritorio (no tiene batería)
                if ok_index == 0 {
                    if !aviso_sin_bateria_emitido {
                        advertir("No se detectaron baterías en el sistema.");
                        aviso_sin_bateria_emitido = true;
                    }
                } else {
                    aviso_sin_bateria_emitido = false;
                }
            }
            Err(err) => {
                advertir(&format!(
                    "No se pudo enumerar las baterías del sistema: {}",
                    err
                ));
            }
        }

        // Feature: Polling Dinámico
        // En lugar de despertar la CPU cada 5 segundos ciegamente, dormimos según las necesidades
        // reales del estado de la batería (5s cerca del peligro, 30s en estado seguro).
        thread::sleep(sleep_duration);
    }
}

/// Garantiza que exista un `MonitorBateria` inicializado para el índice actual,
/// y ejecuta el ciclo de validación de umbrales.
fn asegurar_monitor_en_indice(
    monitores: &mut Vec<MonitorBateria>,
    index: usize,
    config: ConfigApp,
    bateria: &battery::Battery,
) {
    if monitores.len() <= index {
        let mut monitor = MonitorBateria::new(config);
        monitor.inicializar(bateria);
        monitores.push(monitor);
    } else {
        monitores[index].procesar_ciclo(bateria);
    }
}

/// Remueve del Vector los monitores de baterías que ya no están presentes (ej. baterías extraíbles).
fn recortar_monitores_activos<T>(items: &mut Vec<T>, active_count: usize) {
    if items.len() > active_count {
        items.truncate(active_count);
    }
}

#[cfg(test)]
mod tests {
    use super::recortar_monitores_activos;

    #[test]
    fn recortar_monitores_activos_recorta_exceso() {
        let mut values = vec![1, 2, 3, 4];
        recortar_monitores_activos(&mut values, 2);
        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn recortar_monitores_activos_conserva_si_no_hay_exceso() {
        let mut values = vec![1, 2];
        recortar_monitores_activos(&mut values, 2);
        assert_eq!(values, vec![1, 2]);

        recortar_monitores_activos(&mut values, 3);
        assert_eq!(values, vec![1, 2]);
    }
}
