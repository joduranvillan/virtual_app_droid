# ==============================================================================
# Script de Instalación Automatizada de Servicios de Windows
# Confidential Vault - Multiplatform Backend Services
# Debe ejecutarse en PowerShell como Administrador
# ==============================================================================

$ErrorActionPreference = "Stop"

Write-Host "====================================================" -ForegroundColor Cyan
Write-Host "  Instalador de Confidential Vault Daemons (Windows) " -ForegroundColor Cyan
Write-Host "====================================================" -ForegroundColor Cyan

# 1. Verificar privilegios de Administrador
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "Este script debe ejecutarse como Administrador. Ejecuta PowerShell como Administrador e inténtalo de nuevo."
    Exit
}

# 2. Rutas del sistema
$BinDir = "C:\Program Files\ConfidentialVault"
$HostBin = "$BinDir\vault-host.exe"
$RuntimeBin = "$BinDir\vault-runtime.exe"
$ScriptDir = $PSScriptRoot

# 3. Detener servicios existentes si ya estuvieran corriendo
Write-Host "[1/6] Comprobando servicios existentes..." -ForegroundColor Cyan
$svcHost = Get-Service -Name "ConfidentialVaultHost" -ErrorAction SilentlyContinue
if ($svcHost) {
    Write-Host "  - Deteniendo ConfidentialVaultHost existente..." -ForegroundColor Yellow
    Stop-Service -Name "ConfidentialVaultHost" -Force -ErrorAction SilentlyContinue
}

$svcRuntime = Get-Service -Name "ConfidentialVaultRuntime" -ErrorAction SilentlyContinue
if ($svcRuntime) {
    Write-Host "  - Deteniendo ConfidentialVaultRuntime existente..." -ForegroundColor Yellow
    Stop-Service -Name "ConfidentialVaultRuntime" -Force -ErrorAction SilentlyContinue
}

# 4. Crear directorio de destino
Write-Host "[2/6] Preparando directorio de instalación: $BinDir" -ForegroundColor Cyan
if (-not (Test-Path $BinDir)) {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
}

# Copiar ejecutables si existen en el directorio actual
if (Test-Path "$ScriptDir\vault-host.exe") {
    Copy-Item "$ScriptDir\vault-host.exe" -Destination $HostBin -Force
}
if (Test-Path "$ScriptDir\vault-runtime.exe") {
    Copy-Item "$ScriptDir\vault-runtime.exe" -Destination $RuntimeBin -Force
}

# 5. Configurar regla de Firewall de Windows para TCP 7443
Write-Host "[3/6] Configurando Firewall de Windows (TCP 7443)..." -ForegroundColor Cyan
$fwRule = Get-NetFirewallRule -DisplayName "Confidential Vault Host TCP 7443" -ErrorAction SilentlyContinue
if (-not $fwRule) {
    New-NetFirewallRule -DisplayName "Confidential Vault Host TCP 7443" `
                        -Direction Inbound `
                        -LocalPort 7443 `
                        -Protocol TCP `
                        -Action Allow `
                        -Profile Any | Out-Null
    Write-Host "  + Regla de Firewall agregada correctamente." -ForegroundColor Green
}

# 6. Registrar ConfidentialVaultRuntime como Servicio de Windows
Write-Host "[4/6] Registrando servicio ConfidentialVaultRuntime..." -ForegroundColor Cyan
if ($svcRuntime) {
    sc.exe delete "ConfidentialVaultRuntime" | Out-Null
    Start-Sleep -Seconds 1
}

New-Service -Name "ConfidentialVaultRuntime" `
            -BinaryPathName "`"$RuntimeBin`" --service" `
            -DisplayName "Confidential Vault Runtime" `
            -Description "Servicio seguro central que procesa la criptografia Noise_XX y despacha llamadas RPC." `
            -StartupType Automatic | Out-Null

sc.exe failure ConfidentialVaultRuntime reset= 86400 actions= restart/5000/restart/10000/restart/20000 | Out-Null

# 7. Registrar ConfidentialVaultHost como Servicio de Windows
Write-Host "[5/6] Registrando servicio ConfidentialVaultHost..." -ForegroundColor Cyan
if ($svcHost) {
    sc.exe delete "ConfidentialVaultHost" | Out-Null
    Start-Sleep -Seconds 1
}

New-Service -Name "ConfidentialVaultHost" `
            -BinaryPathName "`"$HostBin`" --service" `
            -DisplayName "Confidential Vault Host" `
            -Description "Blind forwarder TCP que reenvia el canal cifrado hacia la consola o hipervisor Hyper-V." `
            -StartupType Automatic `
            -DependsOn "ConfidentialVaultRuntime" | Out-Null

sc.exe failure ConfidentialVaultHost reset= 86400 actions= restart/5000/restart/10000/restart/20000 | Out-Null

# 8. Iniciar Servicios
Write-Host "[6/6] Iniciando servicios de Windows..." -ForegroundColor Cyan
Start-Service -Name "ConfidentialVaultRuntime"
Start-Service -Name "ConfidentialVaultHost"

Write-Host "====================================================" -ForegroundColor Green
Write-Host "  ¡Instalación de servicios completada exitosamente! " -ForegroundColor Green
Write-Host "====================================================" -ForegroundColor Green
Get-Service -Name "ConfidentialVault*"
