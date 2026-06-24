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
echo "🗑️  Eliminando ejecutable..."
rm -f "$HOME/.local/bin/battery-assistant"
echo "   ✓ Ejecutable eliminado"

# Eliminar audios legacy (por si existían en una versión anterior)
rm -rf "$HOME/.local/share/battery-assistant"
echo "   ✓ Recursos compartidos (legacy) eliminados"

# Eliminar autostart (legacy)
rm -f ~/.config/autostart/battery-assistant.desktop
echo "   ✓ Autostart (legacy) eliminado"

echo ""
echo "✅ Battery Assistant desinstalado correctamente"
