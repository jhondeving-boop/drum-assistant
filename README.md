# 🔋 Battery Assistant

Asistente de batería para Linux con notificaciones de audio.

## ✨ Características

- 🔌 Aviso al conectar el cargador
- 🔋 Aviso al desconectar el cargador  
- ⚠️ Alerta de batería baja (< 20%)
- ✅ Aviso de carga completa (> 95%)
- 🚀 Se inicia automáticamente con el sistema

## 📦 Instalación

Requiere [Rust](https://rustup.rs/) instalado.

```bash
git clone https://github.com/tu-usuario/battery-assistant.git
cd battery-assistant
cargo build --release
sudo ./install.sh
```

## 🗑️ Desinstalar

```bash
sudo rm /usr/local/bin/battery-assistant
sudo rm -rf /usr/share/battery-assistant
rm ~/.config/autostart/battery-assistant.desktop
```

## 📄 Licencia

MIT
