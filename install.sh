#!/bin/bash

set -euo pipefail

TOTAL_STEPS=8
CURRENT_STEP=0
STEP_TIMER=0
INSTALL_TIMER=0
AUTO_YES=0
ALLOW_RUST_INSTALL=1
ALLOW_DEPS_INSTALL=1

usage() {
    cat <<'EOF'
Uso: ./install.sh [opciones]

Opciones:
  -y, --yes               Aceptar prompts automaticamente
      --skip-rust-install No instalar Rust automaticamente
      --skip-deps-install No instalar dependencias ALSA automaticamente
  -h, --help              Mostrar esta ayuda
EOF
}

on_error() {
    local exit_code=$?
    echo ""
    echo "❌ Error en la linea ${1}. Abortando (codigo ${exit_code})."
    exit "$exit_code"
}

trap 'on_error $LINENO' ERR

step() {
    CURRENT_STEP=$((CURRENT_STEP + 1))
    STEP_TIMER=$SECONDS
    echo ""
    echo "[$CURRENT_STEP/$TOTAL_STEPS] $1"
}

ok() {
    echo "   ✓ $1"
}

step_done() {
    local elapsed=$((SECONDS - STEP_TIMER))
    echo "   ⏱ Completado en ${elapsed}s"
}

confirm() {
    local message="$1"
    local answer
    if [ "$AUTO_YES" -eq 1 ]; then
        return 0
    fi

    read -r -p "$message (y/n): " answer
    [[ "$answer" =~ ^[Yy]$ ]]
}

detect_package_manager() {
    if command -v pacman >/dev/null 2>&1; then
        echo "pacman"
    elif command -v apt >/dev/null 2>&1; then
        echo "apt"
    elif command -v dnf >/dev/null 2>&1; then
        echo "dnf"
    else
        echo "unknown"
    fi
}

install_dependencies() {
    local pm
    pm=$(detect_package_manager)

    echo "📦 Instalando dependencias del sistema..."

    case "$pm" in
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

create_default_config_if_missing() {
    mkdir -p "$HOME/.config/battery-assistant"
    if [ ! -f "$HOME/.config/battery-assistant/config.toml" ]; then
        cat > "$HOME/.config/battery-assistant/config.toml" <<'EOF'
umbral_baja = 20
umbral_alta = 80
cooldown_segundos = 60
EOF
        ok "Configuracion creada en ~/.config/battery-assistant/config.toml"
    else
        ok "Configuracion existente preservada"
    fi
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        -y|--yes)
            AUTO_YES=1
            ;;
        --skip-rust-install)
            ALLOW_RUST_INSTALL=0
            ;;
        --skip-deps-install)
            ALLOW_DEPS_INSTALL=0
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "❌ Opcion desconocida: $1"
            usage
            exit 1
            ;;
    esac
    shift
done

echo "🔋 Battery Assistant - Instalador"
echo "=================================="
echo ""
INSTALL_TIMER=$SECONDS

step "Verificando dependencias"
if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ Rust no esta instalado."

    if [ "$ALLOW_RUST_INSTALL" -eq 0 ]; then
        echo "   Instala Rust manualmente o ejecuta sin --skip-rust-install"
        exit 1
    fi

    if confirm "Deseas instalar Rust"; then
        echo "📥 Instalando Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
        ok "Rust instalado"
    else
        echo "❌ Rust es necesario para compilar."
        exit 1
    fi
else
    ok "Rust instalado"
fi
step_done

step "Verificando dependencias de audio (ALSA)"
if ! pkg-config --exists alsa 2>/dev/null; then
    echo "❌ Dependencias ALSA no encontradas."

    if [ "$ALLOW_DEPS_INSTALL" -eq 0 ]; then
        echo "   Instalalas manualmente o ejecuta sin --skip-deps-install"
        exit 1
    fi

    if confirm "Deseas instalar las dependencias de audio"; then
        install_dependencies
        ok "Dependencias de audio instaladas"
    else
        echo "❌ Las dependencias de audio son necesarias."
        exit 1
    fi
else
    ok "Dependencias de audio instaladas"
fi
step_done

step "Compilando en modo release"
cargo build --release
ok "Compilacion finalizada"
step_done

step "Instalando en el sistema"
echo "   ...deteniendo servicio actual si existe"
systemctl --user stop battery-assistant 2>/dev/null || true
ok "Servicio detenido (si existia)"

echo "   ...creando directorio de audios (requiere sudo)"
sudo mkdir -p /usr/share/battery-assistant
ok "Directorio de audios listo"

echo "   ...copiando ejecutable (requiere sudo)"
sudo install -m 755 target/release/battery_assistant /usr/local/bin/battery-assistant.new
sudo mv /usr/local/bin/battery-assistant.new /usr/local/bin/battery-assistant
ok "Ejecutable instalado"

echo "   ...copiando audios (requiere sudo)"
sudo cp assets/*.mp3 /usr/share/battery-assistant/
ok "Archivos de audio instalados"
step_done

step "Configurando servicio systemd"
mkdir -p "$HOME/.config/systemd/user"
cp battery-assistant.service "$HOME/.config/systemd/user/"
ok "Archivo de servicio copiado"
create_default_config_if_missing
step_done

step "Recargando daemon de usuario"
systemctl --user daemon-reload
ok "Daemon recargado"
step_done

step "Habilitando e iniciando servicio"
systemctl --user enable --now battery-assistant
ok "Servicio habilitado e iniciado"
step_done

step "Verificando estado del servicio"
ok "Instalacion completada"
step_done

echo ""
echo "✅ Instalacion completada"
echo "⏱ Tiempo total: $((SECONDS - INSTALL_TIMER))s"
echo ""
echo "📍 Ejecutable: /usr/local/bin/battery-assistant"
echo "🔊 Audios: /usr/share/battery-assistant/"
echo "⚙️  Servicio: $HOME/.config/systemd/user/battery-assistant.service"
echo ""
echo "Estado del servicio:"
systemctl --user --no-pager --lines=5 status battery-assistant
