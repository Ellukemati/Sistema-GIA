use crate::errors::ManualStorageError;
use crate::repository::modelo_repository::ModeloRepository;
use crate::service::manual_service::validar_y_procesar_manual;
use rouille::{Request, Response};
use rusqlite::Connection;
use std::io::Read;
use std::sync::{Arc, Mutex};

pub struct ManualHandler;

impl ManualHandler {
    pub fn subir_manual(
        request: &Request,
        modelo_id: i64,
        conn: Arc<Mutex<Connection>>,
    ) -> Response {
        let mut bytes = Vec::new();
        let Some(mut reader) = request.data() else {
            return Response::text("No se recibio ningun archivo").with_status_code(400);
        };

        if let Err(e) = reader.read_to_end(&mut bytes) {
            return Response::from_data(
                "text/plain",
                format!("Error de Entrada/Salida al leer el documento: {}", e),
            )
            .with_status_code(500);
        }

        let (pdf_data, mime_type) = match validar_y_procesar_manual(&bytes) {
            Ok(data) => data,
            Err(ManualStorageError::InvalidManual(msg)) => {
                return Response::text(msg).with_status_code(400);
            }
            Err(e) => {
                return Response::from_data("text/plain", e.to_string()).with_status_code(500);
            }
        };

        match conn.lock() {
            Ok(guard) => {
                match ModeloRepository::actualizar_manual(&guard, modelo_id, &pdf_data, &mime_type)
                {
                    Ok(_) => Response::text("Manual subido exitosamente"),
                    Err(e) => Response::from_data(
                        "text/plain",
                        format!("Error en la base de datos: {}", e),
                    )
                    .with_status_code(500),
                }
            }
            Err(_) => Response::from_data("text/plain", "Mutex envenenado").with_status_code(500),
        }
    }

    pub fn descargar_manual(modelo_id: i64, conn: Arc<Mutex<Connection>>) -> Response {
        match conn.lock() {
            Ok(guard) => match ModeloRepository::buscar_manual(&guard, modelo_id) {
                Ok(Some((blob, mime))) => Response::from_data(mime, blob),
                Ok(None) => {
                    Response::text("El modelo no tiene un manual cargado").with_status_code(404)
                }
                Err(e) => {
                    Response::from_data("text/plain", format!("Error en la base de datos: {}", e))
                        .with_status_code(500)
                }
            },
            Err(_) => Response::from_data("text/plain", "Mutex envenenado").with_status_code(500),
        }
    }
}
