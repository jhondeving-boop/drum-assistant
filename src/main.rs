use crate::config::ConfigApp;
use crate::logger::advertir;
use crate::monitor::MonitorBateria;
use battery::Manager;
use futures_util::stream::StreamExt;
use std::time::Duration;
use zbus::Connection;
use zbus::proxy;

mod audio;
mod config;
mod logger;
mod monitor;

// Proxy para D-Bus UPower
#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    #[zbus(property)]
    fn on_battery(&self) -> zbus::Result<bool>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializar el gestor de batería
    let mut manager = match Manager::new() {
        Ok(m) => m,
        Err(e) => {
            advertir(&format!("No se pudo inicializar battery manager: {}", e));
            return Err(e.into());
        }
    };
    let config = ConfigApp::cargar();

    let mut monitores: Vec<MonitorBateria> = Vec::new();
    let mut aviso_sin_bateria_emitido = false;

    println!("🔊 Asistente de Batería activado (Modo Event-Driven UPower)...");
    println!(
        "Configuración: Baja <= {:.0}%, Alta >= {:.0}%, Repetición cada {}s",
        config.umbral_baja, config.umbral_alta, config.cooldown_segundos
    );

    // Conectar a D-Bus del sistema (System Bus) para escuchar a UPower
    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            advertir(&format!("No se pudo conectar a D-Bus (System): {}. Fallback al polling.", e));
            // Si D-Bus falla, usamos el bucle fallback de siempre
            run_fallback_loop(&mut manager, config, &mut monitores, &mut aviso_sin_bateria_emitido).await;
            return Ok(());
        }
    };

    let upower_proxy = match UPowerProxy::new(&conn).await {
        Ok(p) => p,
        Err(e) => {
            advertir(&format!("No se pudo crear proxy de UPower: {}. Fallback al polling.", e));
            run_fallback_loop(&mut manager, config, &mut monitores, &mut aviso_sin_bateria_emitido).await;
            return Ok(());
        }
    };

    // Suscribirse a cambios en las propiedades de UPower
    let mut property_stream = upower_proxy.receive_on_battery_changed().await;

    // Realizar un escaneo inicial al arrancar
    procesar_baterias(&mut manager, config, &mut monitores, &mut aviso_sin_bateria_emitido);

    let mut fast_poll_ticks = 0;

    // Bucle principal híbrido (Event-Driven + Polling Lento)
    loop {
        // En cada iteración procesamos las baterías
        let mut sleep_duration = procesar_baterias(&mut manager, config, &mut monitores, &mut aviso_sin_bateria_emitido);

        // Si estamos en modo ráfaga tras un evento D-Bus, reducimos drásticamente la espera
        if fast_poll_ticks > 0 {
            sleep_duration = Duration::from_millis(400); // 400ms para latencia casi imperceptible
            fast_poll_ticks -= 1;
        }

        // Dormimos, pero podemos ser "despertados" instantáneamente por D-Bus si conectas el cable
        tokio::select! {
            _ = tokio::time::sleep(sleep_duration) => {
                // Despertó por tiempo (polling)
            }
            evento_dbus = property_stream.next() => {
                if evento_dbus.is_some() {
                    // Despertó instantáneamente porque UPower detectó el cable.
                    // Activamos una ráfaga de lecturas rápidas para atrapar el momento exacto
                    // en que `sysfs` (el sistema de archivos de batería de Linux) se actualiza.
                    fast_poll_ticks = 10; // 10 * 400ms = 4 segundos de ráfaga
                } else {
                    // El stream de D-Bus se cerró (muy raro)
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}

/// Fallback si D-Bus o UPower no están disponibles (como BSD u otros sistemas)
async fn run_fallback_loop(
    manager: &mut Manager,
    config: ConfigApp,
    monitores: &mut Vec<MonitorBateria>,
    aviso_sin_bateria_emitido: &mut bool,
) {
    loop {
        let sleep_duration = procesar_baterias(manager, config, monitores, aviso_sin_bateria_emitido);
        tokio::time::sleep(sleep_duration).await;
    }
}

/// Lógica extraída de lectura y evaluación de estado. Retorna el tiempo que debería dormir.
fn procesar_baterias(
    manager: &mut Manager,
    config: ConfigApp,
    monitores: &mut Vec<MonitorBateria>,
    aviso_sin_bateria_emitido: &mut bool,
) -> Duration {
    // Para asegurarse de obtener datos frescos, a veces es necesario refrescar el manager entero
    // en algunas combinaciones de kernel/battery crate, pero probaremos iterando normal.
    let mut sleep_duration = Duration::from_secs(30);
    
    // Forzamos refresh al manager creándolo de nuevo para evitar caché de sysfs
    if let Ok(new_manager) = Manager::new() {
        *manager = new_manager;
    }

    match manager.batteries() {
        Ok(baterias) => {
            let mut ok_index = 0usize;

            for battery_result in baterias {
                match battery_result {
                    Ok(bateria) => {
                        asegurar_monitor_en_indice(monitores, ok_index, config, &bateria);

                        let current_sleep = monitores[ok_index].tiempo_espera_dinamico(&bateria);
                        if current_sleep < sleep_duration {
                            sleep_duration = current_sleep;
                        }

                        ok_index += 1;
                    }
                    Err(err) => {
                        advertir(&format!("No se pudo leer el estado de una bateria: {}", err));
                    }
                }
            }

            recortar_monitores_activos(monitores, ok_index);

            if ok_index == 0 {
                if !*aviso_sin_bateria_emitido {
                    advertir("No se detectaron baterías en el sistema.");
                    *aviso_sin_bateria_emitido = true;
                }
            } else {
                *aviso_sin_bateria_emitido = false;
            }
        }
        Err(err) => {
            advertir(&format!("No se pudo enumerar las baterías del sistema: {}", err));
        }
    }

    sleep_duration
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
