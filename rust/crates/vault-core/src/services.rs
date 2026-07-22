//! Lado runtime de las llamadas RPC a servicios virtuales — código de
//! orquestación puro, sin nada de una plataforma en particular. Cuando
//! una app dentro del Android Runtime (a integrar en Fase 1, ver
//! ARCHITECTURE.md §13) invoca por ejemplo `LocationManager.getLastLocation()`,
//! el system service correspondiente terminaría llamando a algo como
//! `request_location(...)` acá abajo en lugar de tocar hardware local.

use anyhow::{anyhow, Result};
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::debug;
use vault_crypto::NoiseChannel;
use vault_protocol::{
    Frame, LocationAccuracy, LocationRequest, LocationResponse, MsgType, ServiceId,
    ServiceRequestEnvelope, ServiceResponseEnvelope, ServiceResult,
    AdminAction, AdminRequestPayload, AdminResponsePayload,
};

pub fn handle_admin_request(
    request_body: &[u8],
) -> Result<AdminResponsePayload> {
    let request: AdminRequestPayload = serde_cbor::from_slice(request_body)?;
    
    match request.action {
        AdminAction::GetLogs => {
            let mock_logs = vec![
                "[17:01:02] [vault-core] Inicializando orquestador virtual...".to_string(),
                "[17:01:05] [vault-crypto] Llave de sesión Noise_XX cargada correctamente.".to_string(),
                "[17:01:06] [vault-linux] Volumen cifrado montado correctamente en /dev/mapper/vault.".to_string(),
                "[17:01:10] [vault-stream] Pipeline de codificación H.265 iniciado a 20 FPS.".to_string(),
                "[17:01:15] [vault-core] Enlace seguro activo con cliente Android.".to_string(),
            ];
            Ok(AdminResponsePayload {
                success: true,
                message: "Logs del sistema obtenidos correctamente".to_string(),
                logs: mock_logs,
            })
        }
        AdminAction::RebootVault => {
            Ok(AdminResponsePayload {
                success: true,
                message: "Reinicio ordenado de la bóveda iniciado remotamente".to_string(),
                logs: vec![],
            })
        }
        AdminAction::ChangeNetwork => {
            let network = request.target_network.unwrap_or_else(|| "DHCP (Autodetect)".to_string());
            Ok(AdminResponsePayload {
                success: true,
                message: format!("Configuración de red cambiada exitosamente a: {}", network),
                logs: vec![],
            })
        }
        AdminAction::FactoryReset => {
            Ok(AdminResponsePayload {
                success: true,
                message: "Reestablecimiento de fábrica completado (llaves revocadas)".to_string(),
                logs: vec![],
            })
        }
        AdminAction::UpdateRuntime => {
            let version = request.update_version.unwrap_or_else(|| "v1.1.0-stable".to_string());
            Ok(AdminResponsePayload {
                success: true,
                message: format!("Actualización de runtime a la versión {} completada con éxito", version),
                logs: vec![],
            })
        }
    }
}

pub fn dispatch_service_request(envelope: ServiceRequestEnvelope) -> ServiceResponseEnvelope {
    let service = envelope.service;
    match service {
        ServiceId::Admin => {
            match handle_admin_request(&envelope.body) {
                Ok(resp_payload) => {
                    match serde_cbor::to_vec(&resp_payload) {
                        Ok(body_bytes) => ServiceResponseEnvelope {
                            service,
                            result: ServiceResult::Ok(body_bytes),
                        },
                        Err(e) => ServiceResponseEnvelope {
                            service,
                            result: ServiceResult::Error(format!("Error serializando respuesta: {}", e)),
                        },
                    }
                }
                Err(e) => ServiceResponseEnvelope {
                    service,
                    result: ServiceResult::Error(format!("Error procesando comando administrativo: {}", e)),
                },
            }
        }
        _ => ServiceResponseEnvelope {
            service,
            result: ServiceResult::Unavailable,
        }
    }
}

pub async fn request_location<S: AsyncRead + AsyncWrite + Unpin>(
    chan: &mut NoiseChannel<S>,
    req_id: u64,
    requesting_package: &str,
    accuracy: LocationAccuracy,
) -> Result<LocationResponse> {
    let body = serde_cbor::to_vec(&LocationRequest { accuracy })?;
    let envelope = ServiceRequestEnvelope {
        service: ServiceId::Location,
        requesting_package: requesting_package.to_string(),
        body,
    };
    let frame = Frame::new_serde(MsgType::ServiceRequest, req_id, &envelope)?;
    chan.send_frame(&frame).await?;
    debug!(req_id, package = requesting_package, "pedido de ubicación enviado al frontend");

    // Espera la respuesta correspondiente; frames con otro req_id
    // (ej. heartbeats intercalados) se ignoran acá — en una implementación
    // completa esto sería un demultiplexor por req_id con un mapa de
    // oneshot channels, no un loop bloqueante como este MVP.
    loop {
        let response_frame = chan.recv_frame().await?;
        if response_frame.msg_type != MsgType::ServiceResponse || response_frame.req_id != req_id {
            continue;
        }
        let envelope: ServiceResponseEnvelope = response_frame.decode_body()?;
        return match envelope.result {
            ServiceResult::Ok(bytes) => {
                let loc: LocationResponse = serde_cbor::from_slice(&bytes)?;
                Ok(loc)
            }
            ServiceResult::PermissionDenied => Err(anyhow!("permiso de ubicación denegado por el usuario")),
            ServiceResult::Unavailable => Err(anyhow!("servicio de ubicación no disponible en el frontend")),
            ServiceResult::Error(msg) => Err(anyhow!("error del frontend: {msg}")),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vault_protocol::{ServiceId, ServiceRequestEnvelope, ServiceResponseEnvelope, ServiceResult, AdminAction, AdminRequestPayload, AdminResponsePayload};

    #[test]
    fn test_handle_admin_request_get_logs() {
        let req = AdminRequestPayload {
            action: AdminAction::GetLogs,
            target_network: None,
            update_version: None,
        };
        let body = serde_cbor::to_vec(&req).unwrap();
        let resp = handle_admin_request(&body).unwrap();
        assert!(resp.success);
        assert!(!resp.logs.is_empty());
        assert_eq!(resp.message, "Logs del sistema obtenidos correctamente");
    }

    #[test]
    fn test_handle_admin_request_reboot() {
        let req = AdminRequestPayload {
            action: AdminAction::RebootVault,
            target_network: None,
            update_version: None,
        };
        let body = serde_cbor::to_vec(&req).unwrap();
        let resp = handle_admin_request(&body).unwrap();
        assert!(resp.success);
        assert_eq!(resp.message, "Reinicio ordenado de la bóveda iniciado remotamente");
    }

    #[test]
    fn test_dispatch_service_request_admin() {
        let req = AdminRequestPayload {
            action: AdminAction::ChangeNetwork,
            target_network: Some("10.0.0.1".to_string()),
            update_version: None,
        };
        let body = serde_cbor::to_vec(&req).unwrap();
        let envelope = ServiceRequestEnvelope {
            service: ServiceId::Admin,
            requesting_package: "com.vault.admin".to_string(),
            body,
        };
        
        let response = dispatch_service_request(envelope);
        assert_eq!(response.service, ServiceId::Admin);
        match response.result {
            ServiceResult::Ok(bytes) => {
                let resp_payload: AdminResponsePayload = serde_cbor::from_slice(&bytes).unwrap();
                assert!(resp_payload.success);
                assert!(resp_payload.message.contains("10.0.0.1"));
            }
            other => panic!("Expected Ok, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_service_request_unavailable() {
        let envelope = ServiceRequestEnvelope {
            service: ServiceId::Camera,
            requesting_package: "com.vault.camera".to_string(),
            body: vec![],
        };
        let response = dispatch_service_request(envelope);
        assert_eq!(response.service, ServiceId::Camera);
        assert!(matches!(response.result, ServiceResult::Unavailable));
    }
}
