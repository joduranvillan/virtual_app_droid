# Adaptadores de Confidential Vault como Servicios Nativos de Fondo (Daemons)

Este documento detalla la implementación, empaquetado y despliegue de los componentes `vault-host` y `vault-runtime` como servicios de fondo nativos y seguros en las tres plataformas principales: **Linux (Systemd)**, **Windows (Windows Service Control Manager)** y **macOS (Launchd)**.

---

## 1. Diseño y Arquitectura de Daemons

Los adaptadores del host principal se dividen en dos binarios independientes por razones de seguridad (privilegios mínimos y sandbox):
1. **`vault-host` (Blind Forwarder)**: Expone el puerto de red `TCP 7443` hacia el exterior (dispositivo Android). No posee acceso a las llaves de sesión ni visibilidad del tráfico descifrado.
2. **`vault-runtime` (Core Secure Decryptor/RPC Dispatcher)**: Escucha únicamente en la interfaz de bucle local (`127.0.0.1` o sockets Unix locales `/run/vault-runtime.sock`), procesa la criptografía Noise_XX, valida identidades, y despacha las peticiones RPC del sistema.

---

## 2. Despliegue en Linux (Systemd)

Systemd es el estándar industrial en entornos de servidor y estaciones de trabajo basadas en Linux.

### Archivos de Configuración Proporcionados:
- **`daemons/linux/vault-runtime.service`**: Lanza el núcleo seguro de la bóveda.
- **`daemons/linux/vault-host.service`**: Lanza el relay ciego que expone el puerto público.

### Pasos de Despliegue:
1. Crear los usuarios y grupos dedicados con privilegios mínimos en el sistema:
   ```bash
   sudo groupadd -r vault-runtime
   sudo useradd -r -g vault-runtime -d /var/lib/vault-runtime -s /sbin/nologin vault-runtime
   
   sudo groupadd -r vault-operator
   sudo useradd -r -g vault-operator -s /sbin/nologin vault-operator
   ```
2. Mover los binarios compilados a `/usr/local/bin`:
   ```bash
   sudo cp target/release/vault-host /usr/local/bin/
   sudo cp target/release/vault-runtime /usr/local/bin/
   sudo chmod 755 /usr/local/bin/vault-*
   ```
3. Instalar las unidades de servicio de Systemd:
   ```bash
   sudo cp daemons/linux/*.service /etc/systemd/system/
   sudo systemctl daemon-reload
   ```
4. Iniciar y habilitar los servicios para que arranquen con el sistema:
   ```bash
   sudo systemctl enable --now vault-runtime.service
   sudo systemctl enable --now vault-host.service
   ```
5. Monitorear el estado y logs en tiempo real:
   ```bash
   sudo systemctl status vault-host.service
   sudo journalctl -u vault-runtime.service -f
   ```

---

## 3. Despliegue en Windows (Windows Service)

Se ha integrado soporte de primer nivel con el **Windows Service Control Manager (SCM)** utilizando la biblioteca nativa `windows-service` en Rust.

### Lógica de Ejecución Dual:
Los ejecutables de Windows detectan de forma automática si se lanzan con el flag `--service` y actúan como servicios nativos de Windows regulados por el SCM.

### Pasos de Despliegue:
1. Abrir una consola de **PowerShell como Administrador**.
2. Ejecutar el script automatizado proporcionado:
   ```powershell
   Set-ExecutionPolicy Bypass -Scope Process
   .\daemons\windows\setup.ps1
   ```
3. El instalador:
   - Creará el directorio de instalación segura en `C:\Program Files\ConfidentialVault`.
   - Registrará ambos servicios (`ConfidentialVaultRuntime` y `ConfidentialVaultHost`) en el sistema.
   - Configurará dependencias para que el host no inicie hasta que el runtime esté corriendo limpiamente.
   - Definirá políticas de recuperación automática de fallos en el SCM.
4. Para iniciar manualmente los servicios:
   ```powershell
   Start-Service ConfidentialVaultRuntime
   Start-Service ConfidentialVaultHost
   ```

---

## 5. Scripts de Instalación Automatizada y Empaquetado Global

Para simplificar la distribución a gran escala, la carpeta `daemons/` incluye scripts de instalación, desinstalación y empaquetado automatizado:

### Instalación Rápida por Plataforma:
- **Linux (systemd)**:
  ```bash
  sudo ./daemons/linux/install.sh
  # Desinstalación:
  sudo ./daemons/linux/uninstall.sh
  ```
- **Windows (PowerShell SCM)**:
  ```powershell
  Set-ExecutionPolicy Bypass -Scope Process
  .\daemons\windows\setup.ps1
  # Desinstalación:
  .\daemons\windows\uninstall.ps1
  ```
- **macOS (launchd)**:
  ```bash
  sudo ./daemons/macos/install.sh
  # Desinstalación:
  sudo ./daemons/macos/uninstall.sh
  ```

### Empaquetado de Release para Distribución:
Para generar los archivos comprimidos de instalación final (`.tar.gz` y `.zip`) para los clientes/servidores objetivo:
```bash
./daemons/package.sh
```
Esto genera en `daemons/dist/`:
- `confidential-vault-daemon-linux-x86_64.tar.gz`
- `confidential-vault-daemon-windows-x64.zip`
- `confidential-vault-daemon-macos-universal.tar.gz`

