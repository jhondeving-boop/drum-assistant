use crate::logger;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfigApp {
    pub umbral_baja: f32,
    pub umbral_alta: f32,
    pub cooldown_segundos: u64,
}

impl Default for ConfigApp {
    fn default() -> Self {
        Self {
            umbral_baja: 20.0,
            umbral_alta: 80.0,
            cooldown_segundos: 60,
        }
    }
}

#[derive(Deserialize)]
struct ConfigAppParcial {
    umbral_baja: Option<f32>,
    umbral_alta: Option<f32>,
    cooldown_segundos: Option<u64>,
}

impl ConfigApp {
    pub fn cargar() -> Self {
        let default_cfg = Self::default();
        let Some(path) = ruta_config() else {
            return default_cfg;
        };

        if !path.exists() {
            return default_cfg;
        }

        let contenido = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(err) => {
                logger::advertir(&format!("No se pudo leer {}: {}", path.display(), err));
                return default_cfg;
            }
        };

        let parcial = match toml::from_str::<ConfigAppParcial>(&contenido) {
            Ok(cfg) => cfg,
            Err(err) => {
                logger::advertir(&format!("Config invalida en {}: {}", path.display(), err));
                return default_cfg;
            }
        };

        Self::desde_parcial(parcial, default_cfg)
    }

    fn desde_parcial(parcial: ConfigAppParcial, default_cfg: ConfigApp) -> Self {
        let mut cfg = Self {
            umbral_baja: parcial.umbral_baja.unwrap_or(default_cfg.umbral_baja),
            umbral_alta: parcial.umbral_alta.unwrap_or(default_cfg.umbral_alta),
            cooldown_segundos: parcial
                .cooldown_segundos
                .unwrap_or(default_cfg.cooldown_segundos),
        };

        if cfg.umbral_baja < 0.0 || cfg.umbral_alta > 100.0 || cfg.umbral_baja >= cfg.umbral_alta {
            logger::advertir(
                "Config fuera de rango: umbral_baja debe ser < umbral_alta y ambos entre 0..=100. Usando valores por defecto.",
            );
            cfg = default_cfg;
        }

        if cfg.cooldown_segundos == 0 {
            logger::advertir("cooldown_segundos no puede ser 0. Usando valor por defecto (60).");
            cfg.cooldown_segundos = default_cfg.cooldown_segundos;
        }

        cfg
    }
}

fn ruta_config() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/battery-assistant/config.toml"))
}

#[cfg(test)]
mod tests {
    use super::{ConfigApp, ConfigAppParcial};

    #[test]
    fn desde_parcial_usa_defaults_si_faltan_campos() {
        let cfg = ConfigApp::desde_parcial(
            ConfigAppParcial {
                umbral_baja: None,
                umbral_alta: None,
                cooldown_segundos: None,
            },
            ConfigApp::default(),
        );

        assert_eq!(cfg, ConfigApp::default());
    }

    #[test]
    fn desde_parcial_acepta_valores_validos() {
        let cfg = ConfigApp::desde_parcial(
            ConfigAppParcial {
                umbral_baja: Some(15.0),
                umbral_alta: Some(90.0),
                cooldown_segundos: Some(120),
            },
            ConfigApp::default(),
        );

        assert_eq!(
            cfg,
            ConfigApp {
                umbral_baja: 15.0,
                umbral_alta: 90.0,
                cooldown_segundos: 120,
            }
        );
    }

    #[test]
    fn desde_parcial_restablece_default_si_umbral_invalido() {
        let default_cfg = ConfigApp::default();
        let cfg = ConfigApp::desde_parcial(
            ConfigAppParcial {
                umbral_baja: Some(85.0),
                umbral_alta: Some(80.0),
                cooldown_segundos: Some(10),
            },
            default_cfg,
        );

        assert_eq!(cfg, default_cfg);
    }

    #[test]
    fn desde_parcial_reemplaza_cooldown_cero_con_default() {
        let default_cfg = ConfigApp::default();
        let cfg = ConfigApp::desde_parcial(
            ConfigAppParcial {
                umbral_baja: Some(20.0),
                umbral_alta: Some(80.0),
                cooldown_segundos: Some(0),
            },
            default_cfg,
        );

        assert_eq!(cfg.cooldown_segundos, default_cfg.cooldown_segundos);
    }
}
