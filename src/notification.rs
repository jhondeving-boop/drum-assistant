use log::warn;
use notify_rust::{Hint, Notification, Timeout};

/// Nivel de urgencia para notificaciones de escritorio.
#[derive(Debug, Clone, Copy)]
pub enum Urgency {
    #[allow(dead_code)]
    Low,
    Normal,
    Critical,
}

/// Abstracción del sistema de notificaciones de escritorio.
pub trait NotificationService: Send {
    /// Muestra una notificación con título, cuerpo y urgencia.
    fn notify(&self, title: &str, body: &str, urgency: Urgency);
}

/// Implementación real que usa el crate `notify-rust` (XDG Desktop Notifications).
pub struct DesktopNotification;

impl DesktopNotification {
    pub fn new() -> Self {
        Self
    }
}

impl NotificationService for DesktopNotification {
    fn notify(&self, title: &str, body: &str, urgency: Urgency) {
        let mut notif = Notification::new();
        notif
            .summary(title)
            .body(body)
            .appname("battery-assistant")
            .icon("battery");

        let n_urgency = match urgency {
            Urgency::Low => notify_rust::Urgency::Low,
            Urgency::Normal => notify_rust::Urgency::Normal,
            Urgency::Critical => notify_rust::Urgency::Critical,
        };
        notif.urgency(n_urgency);

        match urgency {
            Urgency::Critical => {
                notif.hint(Hint::Resident(true)).timeout(Timeout::Never);
            }
            _ => {
                notif.hint(Hint::Transient(true));
            }
        }

        if let Err(err) = notif.show() {
            warn!("Failed to show notification: {err}");
        }
    }
}
