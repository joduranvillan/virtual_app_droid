#!/usr/bin/env bash
# ==============================================================================
# Script de Instalación Automatizada de Daemons para macOS (Launchd)
# Confidential Vault - Multiplatform Backend Services
# ==============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

BIN_DIR="/usr/local/bin"
LAUNCH_DAEMONS_DIR="/Library/LaunchDaemons"
LOG_DIR="/var/log/vault"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

echo -e "${CYAN}====================================================${NC}"
echo -e "${CYAN}  Instalador de Confidential Vault Daemons (macOS)  ${NC}"
echo -e "${CYAN}====================================================${NC}"

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] Este script debe ejecutarse con sudo o root.${NC}"
    exit 1
fi

# 1. Localizar binarios
HOST_BIN="${SCRIPT_DIR}/vault-host"
RUNTIME_BIN="${SCRIPT_DIR}/vault-runtime"

if [ ! -f "$HOST_BIN" ] || [ ! -f "$RUNTIME_BIN" ]; then
    echo -e "${YELLOW}[INFO] Buscando binarios en target/release de Rust...${NC}"
    HOST_BIN="${PROJECT_ROOT}/rust/target/release/vault-host"
    RUNTIME_BIN="${PROJECT_ROOT}/rust/target/release/vault-runtime"
fi

if [ ! -f "$HOST_BIN" ] || [ ! -f "$RUNTIME_BIN" ]; then
    echo -e "${YELLOW}[INFO] Compilando binarios de Rust en modo release...${NC}"
    if command -v cargo &>/dev/null; then
        (cd "${PROJECT_ROOT}/rust" && cargo build --release)
    else
        echo -e "${RED}[ERROR] Cargo no instalado y no se hallaron binarios precompilados.${NC}"
        exit 1
    fi
fi

# 2. Crear carpeta de logs
echo -e "${CYAN}[1/4] Creando directorio de logs en ${LOG_DIR}...${NC}"
mkdir -p "$LOG_DIR"
chmod 755 "$LOG_DIR"

# 3. Copiar binarios
echo -e "${CYAN}[2/4] Instalando binarios en ${BIN_DIR}...${NC}"
mkdir -p "$BIN_DIR"
cp -f "$HOST_BIN" "${BIN_DIR}/vault-host"
cp -f "$RUNTIME_BIN" "${BIN_DIR}/vault-runtime"
chmod 755 "${BIN_DIR}/vault-host" "${BIN_DIR}/vault-runtime"
chown root:wheel "${BIN_DIR}/vault-host" "${BIN_DIR}/vault-runtime"

# 4. Copiar archivos .plist a /Library/LaunchDaemons
echo -e "${CYAN}[3/4] Instalando ficheros LaunchDaemon (.plist)...${NC}"
cp -f "${SCRIPT_DIR}/com.example.vault-runtime.plist" "${LAUNCH_DAEMONS_DIR}/"
cp -f "${SCRIPT_DIR}/com.example.vault-host.plist" "${LAUNCH_DAEMONS_DIR}/"

chown root:wheel "${LAUNCH_DAEMONS_DIR}/com.example.vault-*.plist"
chmod 644 "${LAUNCH_DAEMONS_DIR}/com.example.vault-*.plist"

# 5. Cargar e iniciar daemons mediante launchctl
echo -e "${CYAN}[4/4] Cargando e iniciando daemons en launchd...${NC}"

# Descargar versiones anteriores si existieran
launchctl unload "${LAUNCH_DAEMONS_DIR}/com.example.vault-host.plist" 2>/dev/null || true
launchctl unload "${LAUNCH_DAEMONS_DIR}/com.example.vault-runtime.plist" 2>/dev/null || true

launchctl load -w "${LAUNCH_DAEMONS_DIR}/com.example.vault-runtime.plist"
launchctl load -w "${LAUNCH_DAEMONS_DIR}/com.example.vault-host.plist"

echo -e "${GREEN}====================================================${NC}"
echo -e "${GREEN}  ¡Instalación en macOS completada exitosamente!   ${NC}"
echo -e "${GREEN}====================================================${NC}"
echo -e "Para monitorear los registros:"
echo -e "  tail -f ${LOG_DIR}/vault-runtime.log"
echo -e "  tail -f ${LOG_DIR}/vault-host.log"
