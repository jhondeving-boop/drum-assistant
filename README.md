# 🔋 Battery Assistant

Asistente de batería para Linux con notificaciones de audio.

## ✨ Características

- 🔌 Aviso al conectar el cargador
- 🔋 Aviso al desconectar el cargador  
- ⚠️ Alerta de batería baja (< 20%)
- ✅ Aviso de carga completa (> 95%)
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
