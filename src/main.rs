mod audio;
mod monitor;

use battery::Manager;
use std::thread;
use std::time::Duration;
use crate::monitor::BatteryMonitor;

fn main() -> Result<(), battery::Error> {
    let manager = Manager::new()?;
    let mut monitor = BatteryMonitor::new();
    let mut es_inicio = true;

    println!("🔊 Asistente de batería activado...");

    loop {
        // Iteramos sobre las baterías (normalmente solo hay 1)
        if let Some(Ok(battery)) = manager.batteries()?.next() {
            if es_inicio {
                monitor.init(&battery);
                es_inicio = false;
            } else {
                monitor.procesar_ciclo(&battery);
            }
        }

        // Revisamos cada 5 segundos (eficiente en CPU)
        thread::sleep(Duration::from_secs(5));
    }
}
