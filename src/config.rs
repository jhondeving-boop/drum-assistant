//! Carga y validación de la configuración desde `~/.config/battery-assistant/config.toml`.

use log::warn;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Configuración completa del asistente de batería.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Umbral inferior para alerta de batería baja (0..100).
    pub low_threshold: f32,
    /// Umbral superior para alerta de carga completa (0..100).
    pub high_threshold: f32,
    /// Segundos de espera entre notificaciones repetidas del mismo tipo.
    pub cooldown_secs: u64,
    /// Volumen de audio (0.0 = silencio, 1.0 = máximo).
    pub volume: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            low_threshold: 20.0,
            high_threshold: 80.0,
            cooldown_secs: 60,
            volume: 1.0,
        }
    }
}

/// Versión parcial del config donde todos los campos son opcionales.
/// Se usa para deserializar TOML y combinar con defaults.
#[derive(Deserialize)]
struct PartialConfig {
    low_threshold: Option<f32>,
    high_threshold: Option<f32>,
    cooldown_secs: Option<u64>,
    volume: Option<f32>,
}

impl Config {
    /// Carga la configuración desde el archivo TOML.
    /// Si el archivo no existe o es inválido, retorna valores por defecto.
    pub fn load() -> Self {
        let defaults = Self::default();

        let Some(path) = config_path() else {
            return defaults;
        };

        if !path.exists() {
            return defaults;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read config {}: {e}", path.display());
                return defaults;
            }
        };

        let partial = match toml::from_str::<PartialConfig>(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                warn!("Invalid config in {}: {e}", path.display());
                return defaults;
            }
        };

        Self::from_partial(partial, defaults)
    }

    /// Combina valores parciales con defaults, validando rangos.
    fn from_partial(partial: PartialConfig, defaults: Config) -> Self {
        let mut cfg = Config {
            low_threshold: partial.low_threshold.unwrap_or(defaults.low_threshold),
            high_threshold: partial.high_threshold.unwrap_or(defaults.high_threshold),
            cooldown_secs: partial.cooldown_secs.unwrap_or(defaults.cooldown_secs),
            volume: partial.volume.unwrap_or(defaults.volume),
        };

        let valid_range = cfg.low_threshold >= 0.0
            && cfg.low_threshold <= 100.0
            && cfg.high_threshold >= 0.0
            && cfg.high_threshold <= 100.0
            && cfg.low_threshold < cfg.high_threshold;

        if !valid_range {
            warn!(
                "Invalid thresholds: low={}, high={}. Must be 0..=100 and low < high. Using defaults.",
                cfg.low_threshold, cfg.high_threshold
            );
            cfg.low_threshold = defaults.low_threshold;
            cfg.high_threshold = defaults.high_threshold;
        }

        if cfg.cooldown_secs == 0 {
            warn!("cooldown_secs cannot be 0. Using default (60).");
            cfg.cooldown_secs = defaults.cooldown_secs;
        }

        cfg.volume = cfg.volume.clamp(0.0, 1.0);

        cfg
    }
}

/// Retorna la ruta esperada del archivo de configuración.
fn config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/battery-assistant/config.toml"))
}

#[cfg(test)]
mod tests {
    use super::{Config, PartialConfig};

    #[test]
    fn uses_defaults_when_fields_missing() {
        let cfg = Config::from_partial(
            PartialConfig {
                low_threshold: None,
                high_threshold: None,
                cooldown_secs: None,
                volume: None,
            },
            Config::default(),
        );
        assert_eq!(cfg.low_threshold, 20.0);
        assert_eq!(cfg.high_threshold, 80.0);
        assert_eq!(cfg.cooldown_secs, 60);
        assert_eq!(cfg.volume, 1.0);
    }

    #[test]
    fn accepts_valid_values() {
        let cfg = Config::from_partial(
            PartialConfig {
                low_threshold: Some(15.0),
                high_threshold: Some(90.0),
                cooldown_secs: Some(120),
                volume: Some(0.5),
            },
            Config::default(),
        );
        assert_eq!(
            cfg,
            Config {
                low_threshold: 15.0,
                high_threshold: 90.0,
                cooldown_secs: 120,
                volume: 0.5,
            }
        );
    }

    #[test]
    fn resets_to_default_when_thresholds_invalid() {
        let defaults = Config::default();
        let cfg = Config::from_partial(
            PartialConfig {
                low_threshold: Some(85.0),
                high_threshold: Some(80.0),
                cooldown_secs: Some(10),
                volume: None,
            },
            defaults.clone(),
        );
        assert_eq!(cfg.low_threshold, defaults.low_threshold);
        assert_eq!(cfg.high_threshold, defaults.high_threshold);
    }

    #[test]
    fn rejects_low_threshold_below_zero() {
        let defaults = Config::default();
        let cfg = Config::from_partial(
            PartialConfig {
                low_threshold: Some(-5.0),
                high_threshold: Some(80.0),
                cooldown_secs: Some(60),
                volume: None,
            },
            defaults.clone(),
        );
        assert_eq!(cfg.low_threshold, defaults.low_threshold);
    }

    #[test]
    fn rejects_high_threshold_above_100() {
        let defaults = Config::default();
        let cfg = Config::from_partial(
            PartialConfig {
                low_threshold: Some(20.0),
                high_threshold: Some(150.0),
                cooldown_secs: Some(60),
                volume: None,
            },
            defaults.clone(),
        );
        assert_eq!(cfg.high_threshold, defaults.high_threshold);
    }

    #[test]
    fn clamps_volume() {
        let cfg = Config::from_partial(
            PartialConfig {
                low_threshold: None,
                high_threshold: None,
                cooldown_secs: None,
                volume: Some(2.0),
            },
            Config::default(),
        );
        assert_eq!(cfg.volume, 1.0);

        let cfg = Config::from_partial(
            PartialConfig {
                low_threshold: None,
                high_threshold: None,
                cooldown_secs: None,
                volume: Some(-1.0),
            },
            Config::default(),
        );
        assert_eq!(cfg.volume, 0.0);
    }

    #[test]
    fn replaces_zero_cooldown_with_default() {
        let defaults = Config::default();
        let cfg = Config::from_partial(
            PartialConfig {
                low_threshold: Some(20.0),
                high_threshold: Some(80.0),
                cooldown_secs: Some(0),
                volume: None,
            },
            defaults,
        );
        assert_eq!(cfg.cooldown_secs, 60);
    }
}
