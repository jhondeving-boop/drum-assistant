#!/bin/bash

# Desinstalador para Battery Assistant

echo "🔋 Desinstalando Battery Assistant..."

# Detener servicio si existe
echo "🛑 Deteniendo servicio..."
systemctl --user stop battery-assistant 2>/dev/null
systemctl --user disable battery-assistant 2>/dev/null
rm -f ~/.config/systemd/user/battery-assistant.service
echo "   ✓ Servicio eliminado"

# Detener proceso si está corriendo (fallback)
pkill -f battery-assistant 2>/dev/null

# Eliminar ejecutable
echo "🗑️  Eliminando archivos del sistema (requiere sudo)..."
sudo rm -f /usr/local/bin/battery-assistant
echo "   ✓ Ejecutable eliminado"

# Eliminar audios
sudo rm -rf /usr/share/battery-assistant
echo "   ✓ Archivos de audio eliminados"

# Eliminar autostart (legacy)
rm -f ~/.config/autostart/battery-assistant.desktop
echo "   ✓ Autostart (legacy) eliminado"

echo ""
echo "✅ Battery Assistant desinstalado correctamente"
