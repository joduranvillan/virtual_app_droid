#!/usr/bin/env bash
# ==============================================================================
# Script de Desinstalación de Daemons para Linux (Systemd)
# Confidential Vault - Multiplatform Backend Services
# ==============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}====================================================${NC}"
echo -e "${CYAN} Desinstalador de Confidential Vault Daemons (Linux) ${NC}"
echo -e "${CYAN}====================================================${NC}"

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] Este script debe ejecutarse con privilegios de superusuario (sudo / root).${NC}"
    exit 1
fi

echo -e "${CYAN}[1/3] Deteniendo y deshabilitando servicios Systemd...${NC}"
systemctl stop vault-host.service 2>/dev/null || true
systemctl stop vault-runtime.service 2>/dev/null || true
systemctl disable vault-host.service 2>/dev/null || true
systemctl disable vault-runtime.service 2>/dev/null || true

rm -f /etc/systemd/system/vault-host.service
rm -f /etc/systemd/system/vault-runtime.service
systemctl daemon-reload

echo -e "${CYAN}[2/3] Eliminando binarios instalados...${NC}"
rm -f /usr/local/bin/vault-host
rm -f /usr/local/bin/vault-runtime

echo -e "${CYAN}[3/3] Limpiando carpetas de tiempo de ejecución y usuarios...${NC}"
rm -rf /var/lib/vault-runtime /run/vault-runtime

if getent passwd vault-runtime &>/dev/null; then
    userdel vault-runtime 2>/dev/null || true
fi
if getent group vault-runtime &>/dev/null; then
    groupdel vault-runtime 2>/dev/null || true
fi

if getent passwd vault-operator &>/dev/null; then
    userdel vault-operator 2>/dev/null || true
fi
if getent group vault-operator &>/dev/null; then
    groupdel vault-operator 2>/dev/null || true
fi

echo -e "${GREEN}Desinstalación de daemons de Linux completada con éxito.${NC}"
