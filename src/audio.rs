use log::warn;
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::mpsc;
use std::thread;

// ──────────────────────────────────────────────
// Assets de audio embebidos en el binario
// ──────────────────────────────────────────────

const AUDIO_CONNECTED: &[u8] = include_bytes!("../assets/conectado.mp3");
const AUDIO_DISCONNECTED: &[u8] = include_bytes!("../assets/desconectado.mp3");
const AUDIO_LOW: &[u8] = include_bytes!("../assets/baja.mp3");
const AUDIO_FULL: &[u8] = include_bytes!("../assets/cargada.mp3");

/// Evento de audio que puede ser reproducido.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    Connected,
    Disconnected,
    LowBattery,
    HighBattery,
}

/// Abstracción del sistema de audio.
pub trait AudioService: Send {
    /// Solicita la reproducción de un evento de audio (no bloqueante).
    fn play(&self, event: AudioEvent);
}

/// Worker de audio con un hilo dedicado y cola interna.
/// Evita crear un thread por cada evento (antes usaba `thread::spawn` por alerta).
#[derive(Clone)]
pub struct AudioWorker {
    sender: mpsc::Sender<AudioEvent>,
}

impl AudioWorker {
    /// Crea el worker, lanza el hilo interno y retorna el handle.
    /// `volume` debe estar entre 0.0 y 1.0.
    pub fn new(volume: f32) -> Self {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            Self::run_worker(receiver, volume);
        });
        Self { sender }
    }

    /// Bucle interno del worker: procesa eventos en cola secuencialmente.
    fn run_worker(rx: mpsc::Receiver<AudioEvent>, volume: f32) {
        let Ok((_stream, handle)) = OutputStream::try_default() else {
            warn!("Failed to initialize audio output. Audio disabled.");
            return;
        };

        for event in rx {
            let bytes = match event {
                AudioEvent::Connected => AUDIO_CONNECTED,
                AudioEvent::Disconnected => AUDIO_DISCONNECTED,
                AudioEvent::LowBattery => AUDIO_LOW,
                AudioEvent::HighBattery => AUDIO_FULL,
            };

            let cursor = Cursor::new(bytes);
            let Ok(source) = Decoder::new(cursor) else {
                warn!("Failed to decode audio for event {event:?}");
                continue;
            };

            let Ok(sink) = Sink::try_new(&handle) else {
                warn!("Failed to create audio sink");
                continue;
            };

            sink.set_volume(volume);
            sink.append(source);
            sink.detach();
        }
    }
}

impl AudioService for AudioWorker {
    fn play(&self, event: AudioEvent) {
        let _ = self.sender.send(event);
    }
}
