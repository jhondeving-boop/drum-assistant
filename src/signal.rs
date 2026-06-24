//! Manejo de señales del sistema para apagado graceful.
//!
//! Escucha SIGTERM (systemctl stop) y SIGINT (Ctrl+C) para cerrar
//! el programa ordenadamente sin cortar audio a medio reproducir.

use tokio::signal::unix::{signal, SignalKind};

/// Espera hasta que se reciba SIGTERM o SIGINT, luego retorna.
/// El caller debe encargarse de la limpieza (abortar tareas, etc.).
pub async fn wait_for_shutdown() {
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }
}
