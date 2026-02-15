# 🔋 Battery Assistant

Asistente de batería para Linux con notificaciones de audio.

## ✨ Características

- 🔌 Aviso al conectar el cargador
- 🔋 Aviso al desconectar el cargador  
- ⚠️ Alerta de batería baja (<= 20%)
- ✅ Aviso de carga suficiente (>= 80%)
- 🧩 Soporte para equipos con múltiples baterías
- 🚀 Se inicia automáticamente con el sistema

## 💻 Sistemas compatibles

- Arch Linux / Manjaro / EndeavourOS
- Debian / Ubuntu / Linux Mint
- Fedora / RHEL

## 📦 Instalación

```bash
git clone https://github.com/jhondeving-boop/asistente_bateria.git
cd asistente_bateria
./install.sh
```

El instalador verificará e instalará las dependencias automáticamente.
Muestra progreso por pasos con tiempo por etapa y tiempo total.

Opciones útiles:

```bash
./install.sh --yes
./install.sh --yes --skip-rust-install --skip-deps-install
```

## ⚙️ Configuración opcional

Si quieres cambiar umbrales o tiempo de repetición, edita este archivo:

`~/.config/battery-assistant/config.toml`

```toml
umbral_baja = 20
umbral_alta = 80
cooldown_segundos = 60
```

- `umbral_baja`: avisa cuando la batería está por debajo o igual a ese porcentaje.
- `umbral_alta`: avisa cuando la carga está por encima o igual a ese porcentaje.
- `cooldown_segundos`: tiempo mínimo entre avisos repetidos del mismo tipo.

El instalador crea este archivo automáticamente con valores por defecto.
Si el archivo tiene valores inválidos, se usan los valores por defecto.

## 📝 Logs

Las advertencias se guardan en:

`~/.local/state/battery-assistant/battery-assistant.log`

### 🔄 Actualizar

Simplemente descarga los cambios y ejecuta el instalador de nuevo (reemplazará el ejecutable automáticamente):

```bash
git pull
./install.sh
```

## 🗑️ Desinstalar

```bash
sudo ./uninstall.sh
```

## 📄 Licencia

MIT
