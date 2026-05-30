use crate::logger;
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::thread;

/// Definimos los posibles eventos de audio que el sistema puede emitir.
pub enum EventoAudio {
    Conectado,
    Desconectado,
    BateriaBaja,
    BateriaCargada,
}

/// Embebemos los archivos de audio en el binario compilado.
/// Esto elimina por completo el I/O de disco (0% uso de disco) al momento de reproducir alertas,
/// mejorando radicalmente la latencia y previniendo fallos si los archivos son movidos.
const AUDIO_CONECTADO: &[u8] = include_bytes!("../assets/conectado.mp3");
const AUDIO_DESCONECTADO: &[u8] = include_bytes!("../assets/desconectado.mp3");
const AUDIO_BAJA: &[u8] = include_bytes!("../assets/baja.mp3");
const AUDIO_CARGADA: &[u8] = include_bytes!("../assets/cargada.mp3");

/// Reproduce un sonido basado en el evento, procesado totalmente en memoria (RAM).
pub fn reproducir_sonido(evento: EventoAudio) {
    // Seleccionamos los bytes correctos según el evento
    let bytes_audio = match evento {
        EventoAudio::Conectado => AUDIO_CONECTADO,
        EventoAudio::Desconectado => AUDIO_DESCONECTADO,
        EventoAudio::BateriaBaja => AUDIO_BAJA,
        EventoAudio::BateriaCargada => AUDIO_CARGADA,
    };

    // Lanzamos un hilo ligero para no bloquear la ejecución del monitor principal
    thread::spawn(move || {
        // Inicializamos la salida de audio del sistema (PipeWire / PulseAudio / ALSA)
        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok(v) => v,
            Err(err) => {
                logger::advertir(&format!("No se pudo inicializar salida de audio: {}", err));
                return;
            }
        };

        let sink = match Sink::try_new(&stream_handle) {
            Ok(s) => s,
            Err(err) => {
                logger::advertir(&format!("No se pudo crear sink de audio: {}", err));
                return;
            }
        };

        // Envolvemos los bytes estáticos en un Cursor, que simula un archivo pero en memoria RAM
        let cursor = Cursor::new(bytes_audio);

        let source = match Decoder::new(cursor) {
            Ok(s) => s,
            Err(err) => {
                logger::advertir(&format!(
                    "No se pudo decodificar el audio en memoria: {}",
                    err
                ));
                return;
            }
        };

        sink.append(source);
        // El hilo espera hasta que el sonido termine de reproducirse antes de morir
        sink.sleep_until_end();
    });
}
