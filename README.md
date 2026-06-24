# 🔋 Battery Assistant

Un asistente de batería para Linux ultraligero y de alto rendimiento, escrito en Rust, con notificaciones de audio, tiempo restante estimado y soporte para systemd.

## ✨ Características Premium

- **Cero I/O de disco:** Todos los recursos de audio (`.mp3`) están incrustados directamente en memoria.
- **CPU Dinámica Adaptativa:** Usa literalmente ~0.0% de CPU en reposo gracias al polling dinámico.
- **Predicción de Autonomía:** Muestra el tiempo estimado en minutos al alcanzar umbrales críticos.
- 🔌 Aviso por voz al conectar el cargador
- 🔋 Aviso por voz al desconectar el cargador
- ⚠️ Alerta de batería crítica (<= 20% configurable)
- ✅ Aviso de carga completada (>= 80% configurable para cuidar la vida útil)
- 🧩 Soporte nativo para notificaciones de escritorio

## 💻 Sistemas compatibles

Cualquier distribución Linux que soporte `systemd` y `notify-send` (PipeWire o PulseAudio):
- Arch Linux / Manjaro / EndeavourOS
- Debian / Ubuntu / Pop!_OS
- Fedora / RHEL

## 📦 Instalación Fácil

Puedes usar nuestro instalador automático que se encarga de todo el proceso en un solo comando:

```bash
git clone https://github.com/jhondeving-boop/asistente_bateria.git
cd asistente_bateria

# Ejecuta el instalador interactivo
./install.sh
```

El instalador verificará e instalará las dependencias (como `alsa-lib` y `Rust` si es necesario), compilará la versión optimizada (`release`) y configurará el servicio en `systemd`.

### Instalación Rápida / Desatendida
Si quieres que el instalador acepte todas las opciones automáticamente (ideal para scripts):
```bash
./install.sh --yes
```

## ⚙️ Configuración

Puedes personalizar el comportamiento en cualquier momento editando este archivo:
`~/.config/battery-assistant/config.toml`

```toml
umbral_baja = 20
umbral_alta = 80
cooldown_segundos = 60
```

- **`umbral_baja`**: Avisa cuando la batería está por debajo o igual a este porcentaje.
- **`umbral_alta`**: Avisa para desconectar y cuidar la batería.
- **`cooldown_segundos`**: Tiempo mínimo (en segundos) para no hacer "spam" de la misma alerta si sigues ignorándola.

*Los cambios se aplican reiniciando el servicio (`systemctl --user restart battery-assistant`).*

## 🗑️ Desinstalación

Desinstalar el asistente es igual de fácil. Hemos incluido un script que elimina todo rastro del programa:

```bash
./uninstall.sh
```

## 🎨 Modo oscuro (Notificaciones)

Las notificaciones se muestran a través del daemon de notificaciones de tu escritorio.
Para que se vean correctamente en un tema oscuro, configura tu daemon:

### Mako (Hyprland / Wayland)
Edita `~/.config/mako/config` o créalo:

```ini
font=monospace 10
background-color=#1e1e2e
text-color=#cdd6f4
border-color=#89b4fa
border-size=2
border-radius=6
padding=12
margin=12
default-timeout=5000

[appname=battery-assistant]
background-color=#1e1e2e
text-color=#cdd6f4
border-color=#f38ba8
```

### Dunst
Edita `~/.config/dunst/dunstrc`:

```ini
[global]
background = "#1e1e2e"
foreground = "#cdd6f4"
frame_color = "#89b4fa"

[battery-assistant]
appname = "battery-assistant"
background = "#1e1e2e"
foreground = "#cdd6f4"
frame_color = "#f38ba8"
```

> El `appname` **battery-assistant** ya está configurado en el código para que el daemon pueda aplicar estas reglas específicas.

## 🔄 Cómo actualizar

Si en el futuro descargas una versión más reciente con `git pull`, solo vuelve a correr el comando `./install.sh`. Se encargará de detener el servicio viejo, recompilar la aplicación más reciente y volver a arrancar sin que tú tengas que tocar nada más.

## 📄 Licencia

MIT License
