#!/bin/bash

# Desinstalador para Battery Assistant

echo "🔋 Desinstalando Battery Assistant..."

# Detener proceso si está corriendo
pkill -f battery-assistant 2>/dev/null

# Eliminar ejecutable
sudo rm -f /usr/local/bin/battery-assistant
echo "   ✓ Ejecutable eliminado"

# Eliminar audios
sudo rm -rf /usr/share/battery-assistant
echo "   ✓ Archivos de audio eliminados"

# Eliminar autostart
rm -f ~/.config/autostart/battery-assistant.desktop
echo "   ✓ Autostart eliminado"

echo ""
echo "✅ Battery Assistant desinstalado correctamente"
