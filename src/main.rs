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
    let manager = Manager::new()?;
    let config = ConfigApp::cargar();
    let mut monitores: Vec<MonitorBateria> = Vec::new();
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
                            asegurar_monitor_en_indice(&mut monitores, ok_index, config, &bateria);
                            ok_index += 1;
                        }
                        Err(err) => {
                            advertir(&format!("No se pudo leer el estado de una bateria: {err}"));
                        }
                    }
                }

                recortar_monitores_activos(&mut monitores, ok_index);

                if ok_index == 0 {
                    if !aviso_sin_bateria_emitido {
                        advertir("No se detectaron baterias en el sistema.");
                        aviso_sin_bateria_emitido = true;
                    }
                } else {
                    aviso_sin_bateria_emitido = false;
                }
            }
            Err(err) => {
                advertir(&format!("No se pudo enumerar baterias del sistema: {err}"));
            }
        }

        // Revisamos cada 5 segundos (eficiente en CPU)
        thread::sleep(Duration::from_secs(5));
    }
}

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
