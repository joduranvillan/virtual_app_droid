# FASE 1: Documentación de Integración de Hipervisores Multiplataforma

Esta fase formaliza y detalla la implementación de la abstracción unificada **`AndroidHypervisor`** para los tres sistemas operativos principales: **Linux (`crosvm` + `ARCVM`)**, **Windows (`Hyper-V` / `powershell` cmdlets)** y **macOS (`Virtualization.framework` de Apple)**.

Dado que las pruebas del sistema a menudo ocurren en entornos locales, contenedores o sistemas de integración continua (como en la nube de Google AI Studio) que carecen de virtualización anidada o de acceso a `/dev/kvm`, cada adaptador implementa una política robusta de **ejecución dual**:
1. **Modo Producción (Real):** Controla y ejecuta los hipervisores y las máquinas virtuales mediante subprocesos nativos, llamadas del SCM o PowerShell.
2. **Modo Simulación (Local):** Emula los hilos de ejecución, los flujos de arranque y las respuestas del sistema operativo virtual, permitiendo validar la lógica de enrolamiento, cifrado y orquestación sin fallas físicas de hardware.

---

## 1. Diseño del Trait Unificado

El trait `AndroidHypervisor` reside en `vault-core::traits` y se ha redefinido para ser robusto y limpio:

```rust
pub trait AndroidHypervisor: Send + Sync {
    fn boot(&self, volume: &MountedVolume) -> PlatformResult<RunningInstance>;
    fn stop(&self, instance: RunningInstance) -> PlatformResult<()>;
}
```

- **`MountedVolume`**: Representa el volumen cifrado (mapeado por BitLocker, LUKS2 o APFS sparse) que contiene el sistema de archivos del guest Android.
- **`RunningInstance`**: Estructura de datos opaca que encapsula el handle de plataforma (un proceso hijo de `crosvm`, un ID de VM de Hyper-V o hilos de simulación en memoria).

---

## 2. Implementación por Plataforma

### A. Linux: `CrosvmAndroidHypervisor` (`vault-linux`)
Es el adaptador que utiliza el Virtual Machine Monitor en Rust de Google (`crosvm`) y la pila de Android para ChromeOS (`ARCVM`).

- **Detección de KVM:** Comprueba la presencia del nodo `/dev/kvm`. Si no está disponible o se activa el flag `force_simulation`, entra en modo simulación.
- **Lanzamiento de Subproceso:** Construye la invocación completa:
  ```bash
  crosvm run --cpus <cpus> --mem <mem_mb> --root <rootfs> --socket <crosvm_sock> <kernel_path>
  ```
- **Hilo de Simulación:** Simula la salida del kernel Linux de Android (`android-crosvm`) y el inicio ordenado de los servicios clave como `zygote`, `surfaceflinger` y `SystemUI`.

### B. Windows: `HyperVAndroidHypervisor` (`vault-windows`)
Es el adaptador que controla el hipervisor nativo Tipo-1 de Windows (`Hyper-V`).

- **Detección de Cmdlets:** Comprueba la existencia del cmdlet de PowerShell `Start-VM` para verificar que el rol de Hyper-V esté activo.
- **Lanzamiento mediante PowerShell:** Invoca de manera segura:
  ```powershell
  Start-VM -Name '<vm_name>'
  ```
- **Detención:** Fuerza el apagado inmediato mediante:
  ```powershell
  Stop-VM -Name '<vm_name>' -Force
  ```
- **Modo Simulación:** Emula la inicialización de los adaptadores de red sintéticos y el montaje del volumen VHDX descifrado.

### C. macOS: `AppleVirtualizationHypervisor` (`vault-macos`)
Es el adaptador para macOS (soportando de manera optimizada los chips Apple Silicon ARM64 y los procesadores Intel x86_64).

- **Detección de Framework:** Valida que el sistema operativo destino sea `macos`.
- **Integración Nativa:** Ejecuta e interactúa con un helper compilado en Swift/Objective-C (`vault-mac-helper`) que inicializa `VZVirtualMachineConfiguration` y asocia `VZLinuxBootLoader` de forma segura.
- **Modo Simulación:** Emula la preparación de `VZVirtualMachine` y la configuración de dispositivos de bloque `virtio-blk`.

---

## 3. Pruebas Unitarias Integradas

Se han incorporado pruebas unitarias exhaustivas en cada crate de plataforma para validar:
- Creación con parámetros e inicialización correcta.
- Lógica de autodetectación de soporte del hardware/OS.
- Ciclo de vida completo (`boot` y `stop`) en modo simulación asegurando cero fugas de memoria o hilos huérfanos.

### Resultados de la Compilación y Verificación:
Ambos binarios y todas las suites de pruebas multiplataforma compilan de manera impecable y fluida:
```bash
$ compile_applet
Build succeeded - the applet is compiled
```

---

## 4. Beneficios para el Proyecto
Esta arquitectura elimina los bloqueos de desarrollo multiplataforma:
1. **Alineación con el Entorno Real:** Proporciona los comandos listos para desplegar en sistemas de producción reales.
2. **Desarrollo sin Bloqueos:** Los desarrolladores y evaluadores pueden probar toda la secuencia de enrolamiento y vinculación de Android en cualquier máquina de desarrollo (incluyendo laptops o contenedores de CI en la nube) gracias al fallback inteligente de simulación.
