#!/bin/bash

# Instalador para Battery Assistant

set -e

echo "🔋 Battery Assistant - Instalador"
echo "=================================="
echo ""

# Función para detectar el gestor de paquetes
detect_package_manager() {
    if command -v pacman &> /dev/null; then
        echo "pacman"
    elif command -v apt &> /dev/null; then
        echo "apt"
    elif command -v dnf &> /dev/null; then
        echo "dnf"
    else
        echo "unknown"
    fi
}

# Función para instalar dependencias
install_dependencies() {
    local pm=$(detect_package_manager)
    
    echo "📦 Instalando dependencias del sistema..."
    
    case $pm in
        pacman)
            sudo pacman -S --needed --noconfirm alsa-lib base-devel
            ;;
        apt)
            sudo apt update
            sudo apt install -y libasound2-dev build-essential
            ;;
        dnf)
            sudo dnf install -y alsa-lib-devel gcc
            ;;
        *)
            echo "❌ No se pudo detectar el gestor de paquetes."
            echo "   Instala manualmente: alsa-lib (desarrollo) y gcc"
            exit 1
            ;;
    esac
}

# Verificar si se ejecuta como root solo cuando sea necesario
check_root_for_install() {
    if [ "$EUID" -ne 0 ]; then
        echo "⚠️  La instalación del ejecutable necesita sudo."
        echo "   Ejecuta: sudo ./install.sh"
        exit 1
    fi
}

# --- 1. Verificar Rust ---
echo "🔍 Verificando dependencias..."
echo ""

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust no está instalado."
    echo ""
    read -p "¿Deseas instalar Rust? (y/n): " install_rust
    
    if [[ "$install_rust" =~ ^[Yy]$ ]]; then
        echo "📥 Instalando Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        echo "✅ Rust instalado"
    else
        echo "❌ Rust es necesario para compilar. Abortando."
        exit 1
    fi
else
    echo "   ✓ Rust instalado"
fi

# --- 2. Verificar dependencias de audio (ALSA) ---
if ! pkg-config --exists alsa 2>/dev/null; then
    echo "❌ Dependencias de audio (ALSA) no encontradas."
    echo ""
    read -p "¿Deseas instalar las dependencias de audio? (y/n): " install_deps
    
    if [[ "$install_deps" =~ ^[Yy]$ ]]; then
        install_dependencies
        echo "✅ Dependencias instaladas"
    else
        echo "❌ Las dependencias de audio son necesarias. Abortando."
        exit 1
    fi
else
    echo "   ✓ Dependencias de audio instaladas"
fi

echo ""

# --- 3. Compilar ---
echo "🔨 Compilando..."
cargo build --release

echo ""

# --- 4. Instalar en el sistema ---
echo "📥 Instalando en el sistema..."

# Crear directorio para archivos de audio
sudo mkdir -p /usr/share/battery-assistant

# Copiar el ejecutable
sudo cp target/release/battery_assistant /usr/local/bin/battery-assistant
sudo chmod +x /usr/local/bin/battery-assistant
echo "   ✓ Ejecutable instalado"

# Copiar archivos de audio desde assets/
sudo cp assets/*.mp3 /usr/share/battery-assistant/
echo "   ✓ Archivos de audio instalados"

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
echo "   ✓ Autostart configurado"

echo ""
echo "✅ ¡Instalación completada!"
echo ""
echo "📍 Ejecutable: /usr/local/bin/battery-assistant"
echo "🔊 Audios: /usr/share/battery-assistant/"
echo "🚀 Autostart: ~/.config/autostart/battery-assistant.desktop"
echo ""
echo "Ejecuta ahora: battery-assistant &"
