#!/bin/bash

# Script para crear el paquete distribuible
# Genera: battery-assistant-linux-x64.tar.gz

set -e

VERSION="1.0.0"
PACKAGE_NAME="battery-assistant-$VERSION-linux-x64"

echo "📦 Creando paquete distribuible..."

# Compilar en release si no existe
if [ ! -f "target/release/battery_assistant" ]; then
    echo "🔨 Compilando..."
    cargo build --release
fi

# Crear directorio temporal
rm -rf "dist/$PACKAGE_NAME"
mkdir -p "dist/$PACKAGE_NAME"

# Copiar archivos
cp target/release/battery_assistant "dist/$PACKAGE_NAME/battery-assistant"
cp *.mp3 "dist/$PACKAGE_NAME/"
cp dist/install.sh "dist/$PACKAGE_NAME/"
chmod +x "dist/$PACKAGE_NAME/install.sh"
chmod +x "dist/$PACKAGE_NAME/battery-assistant"

# Crear README
cat > "dist/$PACKAGE_NAME/README.txt" << 'EOF'
╔══════════════════════════════════════════════════════════╗
║           🔋 BATTERY ASSISTANT v1.0.0                    ║
╠══════════════════════════════════════════════════════════╣
║                                                          ║
║  Asistente de batería con notificaciones de audio        ║
║                                                          ║
║  INSTALACIÓN:                                            ║
║  ────────────                                            ║
║  1. Extrae este archivo                                  ║
║  2. Abre una terminal en la carpeta extraída             ║
║  3. Ejecuta: sudo ./install.sh                           ║
║                                                          ║
║  CARACTERÍSTICAS:                                        ║
║  ─────────────────                                       ║
║  🔌 Aviso al conectar el cargador                        ║
║  🔋 Aviso al desconectar el cargador                     ║
║  ⚠️  Alerta de batería baja (< 20%)                       ║
║  ✅ Aviso de carga completa (> 95%)                       ║
║                                                          ║
║  Se inicia automáticamente con el sistema                ║
║                                                          ║
╚══════════════════════════════════════════════════════════╝
EOF

# Crear archivo tar.gz
cd dist
tar -czvf "$PACKAGE_NAME.tar.gz" "$PACKAGE_NAME"
rm -rf "$PACKAGE_NAME"

echo ""
echo "✅ Paquete creado: dist/$PACKAGE_NAME.tar.gz"
echo ""
echo "📤 Para distribuir, comparte este archivo."
echo "   Los usuarios solo necesitan:"
echo "   1. Extraer: tar -xzf $PACKAGE_NAME.tar.gz"
echo "   2. Instalar: cd $PACKAGE_NAME && sudo ./install.sh"
