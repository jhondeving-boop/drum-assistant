mod audio;
mod battery;
mod config;
mod core;
mod notification;
mod signal;

use audio::{AudioEvent, AudioService, AudioWorker};
use battery::monitor::{AlertEvent, BatteryMonitor, MonitorConfig};
use battery::service::{BatteryService, SysfsBatteryService};

use config::Config;
use futures_util::stream::StreamExt;
use log::{error, info, warn};
use notification::{DesktopNotification, NotificationService, Urgency};
use std::time::Duration;
use tokio::time;
use zbus::proxy;
use zbus::Connection;

// ──────────────────────────────────────────────
// Constantes de temporización
// ──────────────────────────────────────────────

/// Intervalo de polling rápido tras un evento D-Bus (400ms).
const FAST_POLL_INTERVAL_MS: u64 = 400;

/// Número de ciclos rápidos tras un evento D-Bus.
const FAST_POLL_TICKS: u32 = 10;

/// Intervalo de polling cuando no hay baterías detectadas.
const NO_BATTERY_POLL_SECS: u64 = 60;

/// Intervalo de polling en zona segura (lejos de umbrales).
const SAFE_INTERVAL_SECS: u64 = 30;

// ──────────────────────────────────────────────
// Proxy D-Bus para UPower
// ──────────────────────────────────────────────

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    #[zbus(property)]
    fn on_battery(&self) -> zbus::Result<bool>;
}

// ──────────────────────────────────────────────
// Bucle principal
// ──────────────────────────────────────────────

/// Inicializa servicios y ejecuta el bucle híbrido event-driven + polling.
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    let mut battery_service = SysfsBatteryService::new();
    let audio: Box<dyn AudioService> = Box::new(AudioWorker::new(config.volume));
    let notification: Box<dyn NotificationService> = Box::new(DesktopNotification::new());

    let mut monitors: Vec<BatteryMonitor> = Vec::new();
    let mut no_battery_warning_sent = false;

    let monitor_config = MonitorConfig::new(
        config.low_threshold,
        config.high_threshold,
        config.cooldown_secs,
    );

    info!(
        "Battery Assistant started. Low: {:.0}%, High: {:.0}%, Cooldown: {}s, Volume: {:.0}%",
        config.low_threshold,
        config.high_threshold,
        config.cooldown_secs,
        config.volume * 100.0
    );

    let connection = match Connection::system().await {
        Ok(c) => Some(c),
        Err(e) => {
            warn!("D-Bus unavailable: {e}. Falling back to polling.");
            None
        }
    };

    let upower_proxy = if let Some(ref conn) = connection {
        match UPowerProxy::new(conn).await {
            Ok(p) => Some(p),
            Err(e) => {
                warn!("UPower proxy failed: {e}. Falling back to polling.");
                None
            }
        }
    } else {
        None
    };

    let mut property_stream = if let Some(ref proxy) = upower_proxy {
        Some(proxy.receive_on_battery_changed().await)
    } else {
        None
    };

    process_batteries(
        &mut battery_service,
        &mut monitors,
        &monitor_config,
        &mut no_battery_warning_sent,
        &*audio,
        &*notification,
    );

    let mut fast_poll_remaining = 0u32;

    loop {
        let sleep = process_batteries(
            &mut battery_service,
            &mut monitors,
            &monitor_config,
            &mut no_battery_warning_sent,
            &*audio,
            &*notification,
        );

        if fast_poll_remaining > 0 {
            let wait = Duration::from_millis(FAST_POLL_INTERVAL_MS);
            fast_poll_remaining -= 1;
            tokio::select! {
                _ = time::sleep(wait) => {}
                Some(_) = async {
                    property_stream.as_mut()?.next().await
                } => {
                    fast_poll_remaining = FAST_POLL_TICKS;
                }
            }
        } else if let Some(ref mut stream) = property_stream {
            tokio::select! {
                _ = time::sleep(sleep) => {}
                Some(_) = stream.next() => {
                    fast_poll_remaining = FAST_POLL_TICKS;
                }
            }
        } else {
            time::sleep(sleep).await;
        }
    }
}

// ──────────────────────────────────────────────
// Procesamiento de baterías
// ──────────────────────────────────────────────

/// Lee todas las baterías, actualiza los monitores y dispara eventos.
/// Retorna el tiempo de sleep recomendado (el menor entre todas las baterías).
fn process_batteries(
    service: &mut dyn BatteryService,
    monitors: &mut Vec<BatteryMonitor>,
    config: &MonitorConfig,
    no_battery_sent: &mut bool,
    audio: &dyn AudioService,
    notification: &dyn NotificationService,
) -> Duration {
    match service.batteries() {
        Ok(batteries) if batteries.is_empty() => {
            if !*no_battery_sent {
                warn!("No batteries detected in the system.");
                *no_battery_sent = true;
            }
            Duration::from_secs(NO_BATTERY_POLL_SECS)
        }
        Ok(batteries) => {
            let count = batteries.len();
            ensure_monitor_count(monitors, count, config);

            let mut shortest_sleep = Duration::from_secs(SAFE_INTERVAL_SECS);

            for (i, data) in batteries.iter().enumerate() {
                if let Some(monitor) = monitors.get_mut(i) {
                    if !monitor.is_initialized() {
                        monitor.initialize(data);
                    }

                    let current_sleep = monitor.dynamic_sleep(data);
                    if current_sleep < shortest_sleep {
                        shortest_sleep = current_sleep;
                    }

                    let events = monitor.process_cycle(data);
                    handle_events(&events, audio, notification);
                }
            }

            *no_battery_sent = false;
            shortest_sleep
        }
        Err(e) => {
            error!("Failed to enumerate batteries: {e}");
            Duration::from_secs(NO_BATTERY_POLL_SECS)
        }
    }
}

/// Ajusta el vector de monitores para que tenga exactamente `count` entradas.
fn ensure_monitor_count(monitors: &mut Vec<BatteryMonitor>, count: usize, config: &MonitorConfig) {
    while monitors.len() < count {
        monitors.push(BatteryMonitor::new(config.clone()));
    }
    monitors.truncate(count);
}

/// Procesa una lista de eventos: envía notificación de escritorio y reproduce audio.
fn handle_events(
    events: &[AlertEvent],
    audio: &dyn AudioService,
    notification: &dyn NotificationService,
) {
    for event in events {
        match *event {
            AlertEvent::ChargerConnected => {
                info!("Charger connected");
                notification.notify("Power", "Charger connected", Urgency::Normal);
                audio.play(AudioEvent::Connected);
            }
            AlertEvent::ChargerDisconnected => {
                info!("Charger disconnected");
                notification.notify("Power", "Running on battery power", Urgency::Normal);
                audio.play(AudioEvent::Disconnected);
            }
            AlertEvent::LowBattery {
                percentage,
                time_to_empty,
            } => {
                let remaining = time_to_empty
                    .map(|d| {
                        format!(
                            " Estimated {:.0} min remaining.",
                            d.as_secs_f64() / 60.0
                        )
                    })
                    .unwrap_or_default();
                let msg =
                    format!("Critical level: {percentage:.0}%. Connect the charger.{remaining}");
                info!("{msg}");
                notification.notify("Low Battery", &msg, Urgency::Critical);
                audio.play(AudioEvent::LowBattery);
            }
            AlertEvent::HighBattery {
                percentage,
                time_to_full,
            } => {
                let eta = time_to_full
                    .filter(|d| d.as_secs() > 0)
                    .map(|d| format!(" {:.0} min to full.", d.as_secs_f64() / 60.0))
                    .unwrap_or_default();
                let msg = format!(
                    "Level reached: {percentage:.0}%. Unplug to preserve battery life.{eta}"
                );
                info!("{msg}");
                notification.notify("Battery Full", &msg, Urgency::Normal);
                audio.play(AudioEvent::HighBattery);
            }
        }
    }
}

// ──────────────────────────────────────────────
// Entry point
// ──────────────────────────────────────────────

#[tokio::main]
async fn main() {
    core::logger::init();
    info!("Starting Battery Assistant...");

    let handle = tokio::spawn(async {
        if let Err(e) = run().await {
            error!("Fatal error: {e}");
        }
    });

    signal::wait_for_shutdown().await;
    info!("Shutting down...");
    handle.abort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_monitor_count_grows_correctly() {
        let config = MonitorConfig::new(20.0, 80.0, 60);
        let mut monitors = Vec::new();
        ensure_monitor_count(&mut monitors, 2, &config);
        assert_eq!(monitors.len(), 2);

        ensure_monitor_count(&mut monitors, 1, &config);
        assert_eq!(monitors.len(), 1);
    }
}
