#!/usr/bin/env bash
# ==============================================================================
# Script de Instalación Automatizada de Daemons para Linux (Systemd)
# Confidential Vault - Multiplatform Backend Services
# ==============================================================================

set -euo pipefail

RED='\030[0;31m'
GREEN='\032[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

BIN_DIR="/usr/local/bin"
SYSTEMD_DIR="/etc/systemd/system"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

echo -e "${CYAN}====================================================${NC}"
echo -e "${CYAN}  Instalador de Confidential Vault Daemons (Linux)  ${NC}"
echo -e "${CYAN}====================================================${NC}"

# 1. Comprobar privilegios de root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] Este script debe ejecutarse con privilegios de superusuario (sudo / root).${NC}"
    exit 1
fi

# 2. Localizar binarios
HOST_BIN="${SCRIPT_DIR}/vault-host"
RUNTIME_BIN="${SCRIPT_DIR}/vault-runtime"

if [ ! -f "$HOST_BIN" ] || [ ! -f "$RUNTIME_BIN" ]; then
    echo -e "${YELLOW}[INFO] Binarios no encontrados en ${SCRIPT_DIR}. Buscando en target/release de Rust...${NC}"
    HOST_BIN="${PROJECT_ROOT}/rust/target/release/vault-host"
    RUNTIME_BIN="${PROJECT_ROOT}/rust/target/release/vault-runtime"
fi

if [ ! -f "$HOST_BIN" ] || [ ! -f "$RUNTIME_BIN" ]; then
    echo -e "${YELLOW}[INFO] Compilando binarios de Rust en modo release...${NC}"
    if command -v cargo &>/dev/null; then
        (cd "${PROJECT_ROOT}/rust" && cargo build --release)
    else
        echo -e "${RED}[ERROR] Cargo no encontrado y binarios pre-compilados no disponibles.${NC}"
        echo -e "${RED}Por favor compila los binarios o colócalos en ${SCRIPT_DIR}.${NC}"
        exit 1
    fi
fi

# 3. Crear usuarios y grupos del sistema dedicados
echo -e "${CYAN}[1/5] Configurando usuarios y grupos con privilegios mínimos...${NC}"
if ! getent group vault-runtime &>/dev/null; then
    groupadd -r vault-runtime
    echo -e "${GREEN}  + Grupo 'vault-runtime' creado.${NC}"
fi

if ! getent passwd vault-runtime &>/dev/null; then
    useradd -r -g vault-runtime -d /var/lib/vault-runtime -s /sbin/nologin vault-runtime
    echo -e "${GREEN}  + Usuario 'vault-runtime' creado.${NC}"
fi

if ! getent group vault-operator &>/dev/null; then
    groupadd -r vault-operator
    echo -e "${GREEN}  + Grupo 'vault-operator' creado.${NC}"
fi

if ! getent passwd vault-operator &>/dev/null; then
    useradd -r -g vault-operator -s /sbin/nologin vault-operator
    echo -e "${GREEN}  + Usuario 'vault-operator' creado.${NC}"
fi

# Crear directorio de trabajo/socket para vault-runtime
mkdir -p /var/lib/vault-runtime /run/vault-runtime
chown -R vault-runtime:vault-runtime /var/lib/vault-runtime /run/vault-runtime
chmod 750 /var/lib/vault-runtime /run/vault-runtime

# 4. Copiar binarios
echo -e "${CYAN}[2/5] Instalando binarios en ${BIN_DIR}...${NC}"
cp -f "$HOST_BIN" "${BIN_DIR}/vault-host"
cp -f "$RUNTIME_BIN" "${BIN_DIR}/vault-runtime"
chmod 755 "${BIN_DIR}/vault-host" "${BIN_DIR}/vault-runtime"
chown root:root "${BIN_DIR}/vault-host" "${BIN_DIR}/vault-runtime"
echo -e "${GREEN}  + Binarios copiados exitosamente.${NC}"

# 5. Configurar Firewall (ufw / iptables si existen)
echo -e "${CYAN}[3/5] Verificando puerto de red (TCP 7443)...${NC}"
if command -v ufw &>/dev/null && ufw status | grep -q "active"; then
    ufw allow 7443/tcp comment 'Confidential Vault Host TCP Port'
    echo -e "${GREEN}  + Regla UFW añadida para TCP 7443.${NC}"
fi

# 6. Copiar unidades de Systemd
echo -e "${CYAN}[4/5] Instalando unidades de Systemd...${NC}"
cp -f "${SCRIPT_DIR}/vault-runtime.service" "${SYSTEMD_DIR}/"
cp -f "${SCRIPT_DIR}/vault-host.service" "${SYSTEMD_DIR}/"
chmod 644 "${SYSTEMD_DIR}/vault-runtime.service" "${SYSTEMD_DIR}/vault-host.service"

systemctl daemon-reload
echo -e "${GREEN}  + Systemd recargado.${NC}"

# 7. Activar e iniciar servicios
echo -e "${CYAN}[5/5] Activando e iniciando servicios...${NC}"
systemctl enable vault-runtime.service
systemctl enable vault-host.service

systemctl restart vault-runtime.service
systemctl restart vault-host.service

echo -e "${GREEN}====================================================${NC}"
echo -e "${GREEN}  ¡Instalación completada exitosamente!            ${NC}"
echo -e "${GREEN}====================================================${NC}"
echo -e "Estado de los servicios:"
systemctl status vault-runtime.service --no-pager || true
systemctl status vault-host.service --no-pager || true
