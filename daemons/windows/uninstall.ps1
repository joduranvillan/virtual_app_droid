# ==============================================================================
# Script de Desinstalación de Servicios de Windows
# Confidential Vault - Multiplatform Backend Services
# Debe ejecutarse en PowerShell como Administrador
# ==============================================================================

$ErrorActionPreference = "Continue"

Write-Host "====================================================" -ForegroundColor Cyan
Write-Host " Desinstalador de Confidential Vault (Windows)       " -ForegroundColor Cyan
Write-Host "====================================================" -ForegroundColor Cyan

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "Este script debe ejecutarse como Administrador."
    Exit
}

Write-Host "[1/3] Deteniendo y eliminando servicios..." -ForegroundColor Cyan
Stop-Service -Name "ConfidentialVaultHost" -Force -ErrorAction SilentlyContinue
Stop-Service -Name "ConfidentialVaultRuntime" -Force -ErrorAction SilentlyContinue

sc.exe delete "ConfidentialVaultHost" | Out-Null
sc.exe delete "ConfidentialVaultRuntime" | Out-Null

Write-Host "[2/3] Eliminando regla de Firewall..." -ForegroundColor Cyan
Remove-NetFirewallRule -DisplayName "Confidential Vault Host TCP 7443" -ErrorAction SilentlyContinue

Write-Host "[3/3] Eliminando archivos de programa..." -ForegroundColor Cyan
$BinDir = "C:\Program Files\ConfidentialVault"
if (Test-Path $BinDir) {
    Remove-Item -Path $BinDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Desinstalación completada con éxito." -ForegroundColor Green
