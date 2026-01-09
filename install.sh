#!/bin/bash

# Instalador para Battery Assistant
# Ejecutar con: sudo ./install.sh

set -e

echo "🔋 Instalando Battery Assistant..."

# Compilar si no existe
if [ ! -f "target/release/battery_assistant" ]; then
    echo "🔨 Compilando..."
    cargo build --release
fi

# Crear directorio para archivos de audio
sudo mkdir -p /usr/share/battery-assistant

# Copiar el ejecutable
sudo cp target/release/battery_assistant /usr/local/bin/battery-assistant
sudo chmod +x /usr/local/bin/battery-assistant

# Copiar archivos de audio desde assets/
sudo cp assets/*.mp3 /usr/share/battery-assistant/

# Crear archivo .desktop para autostart
mkdir -p ~/.config/autostart

cat > ~/.config/autostart/battery-assistant.desktop << EOF
[Desktop Entry]
Type=Application
Name=Battery Assistant
Comment=Asistente de batería con notificaciones de audio
Exec=/usr/local/bin/battery-assistant
Icon=battery
Terminal=false
Categories=Utility;System;
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF

echo "✅ Instalación completada!"
echo ""
echo "📍 Ejecutable: /usr/local/bin/battery-assistant"
echo "🔊 Audios: /usr/share/battery-assistant/"
echo "🚀 Autostart: ~/.config/autostart/battery-assistant.desktop"
echo ""
echo "Puedes ejecutar ahora con: battery-assistant"
