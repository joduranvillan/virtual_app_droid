#!/usr/bin/env bash
# ==============================================================================
# Script de Desinstalación de Daemons para macOS (Launchd)
# Confidential Vault - Multiplatform Backend Services
# ==============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

LAUNCH_DAEMONS_DIR="/Library/LaunchDaemons"

echo -e "${CYAN}====================================================${NC}"
echo -e "${CYAN} Desinstalador de Confidential Vault Daemons (macOS) ${NC}"
echo -e "${CYAN}====================================================${NC}"

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] Este script debe ejecutarse con sudo o root.${NC}"
    exit 1
fi

echo -e "${CYAN}[1/3] Descargando y deteniendo daemons en launchd...${NC}"
launchctl unload "${LAUNCH_DAEMONS_DIR}/com.example.vault-host.plist" 2>/dev/null || true
launchctl unload "${LAUNCH_DAEMONS_DIR}/com.example.vault-runtime.plist" 2>/dev/null || true

rm -f "${LAUNCH_DAEMONS_DIR}/com.example.vault-host.plist"
rm -f "${LAUNCH_DAEMONS_DIR}/com.example.vault-runtime.plist"

echo -e "${CYAN}[2/3] Eliminando binarios de /usr/local/bin...${NC}"
rm -f /usr/local/bin/vault-host
rm -f /usr/local/bin/vault-runtime

echo -e "${CYAN}[3/3] Limpiando carpetas de logs...${NC}"
rm -rf /var/log/vault-*.log /var/log/vault-*.err /var/log/vault

echo -e "${GREEN}Desinstalación en macOS completada con éxito.${NC}"
