#!/usr/bin/env bash
# ==============================================================================
# Script Global de Empaquetado y Distribución de Daemons Multiplataforma
# Confidential Vault
# ==============================================================================

set -euo pipefail

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="${SCRIPT_DIR}/dist"

echo -e "${CYAN}====================================================${NC}"
echo -e "${CYAN} Empaquetador de Distribución de Daemons Vault      ${NC}"
echo -e "${CYAN}====================================================${NC}"

mkdir -p "${DIST_DIR}"

# 1. Empaquetar Linux
echo -e "${CYAN}[1/3] Generando paquete de distribución para Linux (systemd)...${NC}"
LINUX_DIR="${DIST_DIR}/confidential-vault-daemon-linux-x86_64"
mkdir -p "${LINUX_DIR}"
cp "${SCRIPT_DIR}/linux/"* "${LINUX_DIR}/"
chmod +x "${LINUX_DIR}/"*.sh
tar -czf "${DIST_DIR}/confidential-vault-daemon-linux-x86_64.tar.gz" -C "${DIST_DIR}" "confidential-vault-daemon-linux-x86_64"
rm -rf "${LINUX_DIR}"
echo -e "${GREEN}  + Paquete creado: ${DIST_DIR}/confidential-vault-daemon-linux-x86_64.tar.gz${NC}"

# 2. Empaquetar macOS
echo -e "${CYAN}[2/3] Generando paquete de distribución para macOS (launchd)...${NC}"
MACOS_DIR="${DIST_DIR}/confidential-vault-daemon-macos-universal"
mkdir -p "${MACOS_DIR}"
cp "${SCRIPT_DIR}/macos/"* "${MACOS_DIR}/"
chmod +x "${MACOS_DIR}/"*.sh
tar -czf "${DIST_DIR}/confidential-vault-daemon-macos-universal.tar.gz" -C "${DIST_DIR}" "confidential-vault-daemon-macos-universal"
rm -rf "${MACOS_DIR}"
echo -e "${GREEN}  + Paquete creado: ${DIST_DIR}/confidential-vault-daemon-macos-universal.tar.gz${NC}"

# 3. Empaquetar Windows
echo -e "${CYAN}[3/3] Generando paquete de distribución para Windows (PowerShell SCM)...${NC}"
WIN_DIR="${DIST_DIR}/confidential-vault-daemon-windows-x64"
mkdir -p "${WIN_DIR}"
cp "${SCRIPT_DIR}/windows/"* "${WIN_DIR}/"
if command -v zip &>/dev/null; then
    (cd "${DIST_DIR}" && zip -r "confidential-vault-daemon-windows-x64.zip" "confidential-vault-daemon-windows-x64")
    rm -rf "${WIN_DIR}"
    echo -e "${GREEN}  + Paquete creado: ${DIST_DIR}/confidential-vault-daemon-windows-x64.zip${NC}"
else
    echo -e "${YELLOW}  + Estructura generada en: ${WIN_DIR}${NC}"
fi

echo -e "${GREEN}====================================================${NC}"
echo -e "${GREEN}  ¡Empaquetado de distribución finalizado!          ${NC}"
echo -e "${GREEN}====================================================${NC}"
ls -lh "${DIST_DIR}"
