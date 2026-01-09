#!/bin/bash

# Instalador universal para Battery Assistant
# Solo necesita ejecutar: ./install.sh

set -e

INSTALL_DIR="/usr/local/bin"
AUDIO_DIR="/usr/share/battery-assistant"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "🔋 Battery Assistant - Instalador"
echo "=================================="
echo ""

# Verificar si se ejecuta como root
if [ "$EUID" -ne 0 ]; then
    echo "⚠️  Este instalador necesita permisos de administrador."
    echo "   Ejecuta: sudo $0"
    exit 1
fi

# Verificar que existen los archivos necesarios
if [ ! -f "$SCRIPT_DIR/battery-assistant" ]; then
    echo "❌ Error: No se encontró el ejecutable 'battery-assistant'"
    exit 1
fi

echo "📦 Instalando archivos..."

# Crear directorio de audio
mkdir -p "$AUDIO_DIR"

# Copiar ejecutable
cp "$SCRIPT_DIR/battery-assistant" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/battery-assistant"
echo "   ✓ Ejecutable instalado en $INSTALL_DIR/battery-assistant"

# Copiar archivos de audio
for audio in conectado.mp3 desconectado.mp3 baja.mp3 cargada.mp3; do
    if [ -f "$SCRIPT_DIR/$audio" ]; then
        cp "$SCRIPT_DIR/$audio" "$AUDIO_DIR/"
        echo "   ✓ Audio: $audio"
    else
        echo "   ⚠ Advertencia: $audio no encontrado"
    fi
done

# Obtener el usuario real (no root)
REAL_USER="${SUDO_USER:-$USER}"
REAL_HOME=$(getent passwd "$REAL_USER" | cut -d: -f6)

# Crear archivo .desktop para autostart
AUTOSTART_DIR="$REAL_HOME/.config/autostart"
mkdir -p "$AUTOSTART_DIR"

cat > "$AUTOSTART_DIR/battery-assistant.desktop" << EOF
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

chown "$REAL_USER:$REAL_USER" "$AUTOSTART_DIR/battery-assistant.desktop"
echo "   ✓ Autostart configurado"

echo ""
echo "✅ ¡Instalación completada!"
echo ""
echo "🚀 Para iniciar ahora: battery-assistant &"
echo "📍 Se iniciará automáticamente con el sistema"
echo ""
echo "🗑️  Para desinstalar:"
echo "   sudo rm /usr/local/bin/battery-assistant"
echo "   sudo rm -rf /usr/share/battery-assistant"
echo "   rm ~/.config/autostart/battery-assistant.desktop"
