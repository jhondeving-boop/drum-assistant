use crate::logger;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppConfig {
    pub umbral_baja: f32,
    pub umbral_alta: f32,
    pub cooldown_segundos: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            umbral_baja: 20.0,
            umbral_alta: 80.0,
            cooldown_segundos: 60,
        }
    }
}

#[derive(Deserialize)]
struct AppConfigParcial {
    umbral_baja: Option<f32>,
    umbral_alta: Option<f32>,
    cooldown_segundos: Option<u64>,
}

impl AppConfig {
    pub fn load() -> Self {
        let default_cfg = Self::default();
        let Some(path) = config_path() else {
            return default_cfg;
        };

        if !path.exists() {
            return default_cfg;
        }

        let contenido = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(err) => {
                logger::warn(&format!("No se pudo leer {}: {}", path.display(), err));
                return default_cfg;
            }
        };

        let parcial = match toml::from_str::<AppConfigParcial>(&contenido) {
            Ok(cfg) => cfg,
            Err(err) => {
                logger::warn(&format!("Config invalida en {}: {}", path.display(), err));
                return default_cfg;
            }
        };

        Self::from_partial(parcial, default_cfg)
    }

    fn from_partial(parcial: AppConfigParcial, default_cfg: AppConfig) -> Self {
        let mut cfg = Self {
            umbral_baja: parcial.umbral_baja.unwrap_or(default_cfg.umbral_baja),
            umbral_alta: parcial.umbral_alta.unwrap_or(default_cfg.umbral_alta),
            cooldown_segundos: parcial
                .cooldown_segundos
                .unwrap_or(default_cfg.cooldown_segundos),
        };

        if cfg.umbral_baja < 0.0 || cfg.umbral_alta > 100.0 || cfg.umbral_baja >= cfg.umbral_alta {
            logger::warn(
                "Config fuera de rango: umbral_baja debe ser < umbral_alta y ambos entre 0..=100. Usando valores por defecto.",
            );
            cfg = default_cfg;
        }

        if cfg.cooldown_segundos == 0 {
            logger::warn("cooldown_segundos no puede ser 0. Usando valor por defecto (60).");
            cfg.cooldown_segundos = default_cfg.cooldown_segundos;
        }

        cfg
    }
}

fn config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/battery-assistant/config.toml"))
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, AppConfigParcial};

    #[test]
    fn from_partial_uses_defaults_when_missing_fields() {
        let cfg = AppConfig::from_partial(
            AppConfigParcial {
                umbral_baja: None,
                umbral_alta: None,
                cooldown_segundos: None,
            },
            AppConfig::default(),
        );

        assert_eq!(cfg, AppConfig::default());
    }

    #[test]
    fn from_partial_accepts_valid_custom_values() {
        let cfg = AppConfig::from_partial(
            AppConfigParcial {
                umbral_baja: Some(15.0),
                umbral_alta: Some(90.0),
                cooldown_segundos: Some(120),
            },
            AppConfig::default(),
        );

        assert_eq!(
            cfg,
            AppConfig {
                umbral_baja: 15.0,
                umbral_alta: 90.0,
                cooldown_segundos: 120,
            }
        );
    }

    #[test]
    fn from_partial_resets_to_default_for_invalid_thresholds() {
        let default_cfg = AppConfig::default();
        let cfg = AppConfig::from_partial(
            AppConfigParcial {
                umbral_baja: Some(85.0),
                umbral_alta: Some(80.0),
                cooldown_segundos: Some(10),
            },
            default_cfg,
        );

        assert_eq!(cfg, default_cfg);
    }

    #[test]
    fn from_partial_replaces_zero_cooldown_with_default() {
        let default_cfg = AppConfig::default();
        let cfg = AppConfig::from_partial(
            AppConfigParcial {
                umbral_baja: Some(20.0),
                umbral_alta: Some(80.0),
                cooldown_segundos: Some(0),
            },
            default_cfg,
        );

        assert_eq!(cfg.cooldown_segundos, default_cfg.cooldown_segundos);
    }
}
