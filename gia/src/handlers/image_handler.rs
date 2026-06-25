use crate::errors::ImageStorageError;
use crate::repository::image_repository::ImageRepository;
use crate::service::image_service::{procesar_avatar, procesar_ejemplar, procesar_modelo};

use rouille::{Request, Response};
use rusqlite::Connection;
use std::io::Read;
use std::sync::{Arc, Mutex};

pub struct ImageHandler;

impl ImageHandler {
    fn read_body_bytes(request: &Request) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        let Some(mut reader) = request.data() else {
            return Err("No se recibió ninguna imagen".to_string());
        };
        reader.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if bytes.is_empty() {
            return Err("Archivo vacío".to_string());
        }
        Ok(bytes)
    }

    pub fn subir_modelo(
        request: &Request,
        modelo_id: i64,
        orden: i32,
        conn: Arc<Mutex<Connection>>,
    ) -> Response {
        let bytes = match Self::read_body_bytes(request) {
            Ok(b) => b,
            Err(m) => return Response::text(m).with_status_code(400),
        };

        match procesar_modelo(&bytes) {
            Ok((blob_final, mime)) => match conn.lock() {
                Ok(guard) => match ImageRepository::guardar_modelo(
                    &guard,
                    modelo_id,
                    orden,
                    &blob_final,
                    &mime,
                ) {
                    Ok(_) => Response::text(format!("/imagenes/modelos/{}/{}", modelo_id, orden)),
                    Err(e) => Response::text(e.to_string()).with_status_code(500),
                },
                Err(_) => Response::text("Mutex envenenado").with_status_code(500),
            },
            Err(ImageStorageError::InvalidImage(msg)) => Response::text(msg).with_status_code(400),
            Err(err) => Response::text(err.to_string()).with_status_code(500),
        }
    }

    pub fn servir_modelo(modelo_id: i64, orden: i32, conn: Arc<Mutex<Connection>>) -> Response {
        match conn.lock() {
            Ok(guard) => match ImageRepository::buscar_modelo(&guard, modelo_id, orden) {
                Ok(Some((blob, mime))) => Response::from_data(mime, blob),
                Ok(None) => Response::empty_404(),
                Err(e) => Response::text(e.to_string()).with_status_code(500),
            },
            Err(_) => Response::text("Mutex envenenado").with_status_code(500),
        }
    }

    pub fn guardar_imagen_ejemplar_bytes(
        conn: &Connection,
        ejemplar_id: i64,
        orden: i32,
        bytes: &[u8],
    ) -> Result<(), String> {
        let (blob_final, mime) = procesar_ejemplar(bytes).map_err(|err| match err {
            ImageStorageError::InvalidImage(msg) => msg,
            other => other.to_string(),
        })?;

        ImageRepository::guardar_ejemplar(conn, ejemplar_id, orden, &blob_final, &mime)
            .map_err(|e| e.to_string())
    }

    pub fn subir_ejemplar(
        request: &Request,
        ejemplar_id: i64,
        orden: i32,
        conn: Arc<Mutex<Connection>>,
    ) -> Response {
        let bytes = match Self::read_body_bytes(request) {
            Ok(b) => b,
            Err(m) => return Response::text(m).with_status_code(400),
        };

        match conn.lock() {
            Ok(guard) => match Self::guardar_imagen_ejemplar_bytes(&guard, ejemplar_id, orden, &bytes)
            {
                Ok(_) => {
                    Response::text(format!("/imagenes/ejemplares/{}/{}", ejemplar_id, orden))
                }
                Err(msg) => Response::text(msg).with_status_code(400),
            },
            Err(_) => Response::text("Mutex envenenado").with_status_code(500),
        }
    }

    pub fn servir_ejemplar(ejemplar_id: i64, orden: i32, conn: Arc<Mutex<Connection>>) -> Response {
        match conn.lock() {
            Ok(guard) => match ImageRepository::buscar_ejemplar(&guard, ejemplar_id, orden) {
                Ok(Some((blob, mime))) => Response::from_data(mime, blob),
                Ok(None) => Response::empty_404(),
                Err(e) => Response::text(e.to_string()).with_status_code(500),
            },
            Err(_) => Response::text("Mutex envenenado").with_status_code(500),
        }
    }

    pub fn subir_avatar(request: &Request, legajo: i64, conn: Arc<Mutex<Connection>>) -> Response {
        let bytes = match Self::read_body_bytes(request) {
            Ok(b) => b,
            Err(m) => return Response::text(m).with_status_code(400),
        };

        match procesar_avatar(&bytes) {
            Ok((blob_final, mime)) => match conn.lock() {
                Ok(guard) => {
                    match ImageRepository::guardar_avatar(&guard, legajo, &blob_final, &mime) {
                        Ok(_) => Response::text(format!("/imagenes/avatares/{}", legajo)),
                        Err(e) => Response::text(e.to_string()).with_status_code(500),
                    }
                }
                Err(_) => Response::text("Mutex envenenado").with_status_code(500),
            },
            Err(ImageStorageError::InvalidImage(msg)) => Response::text(msg).with_status_code(400),
            Err(err) => Response::text(err.to_string()).with_status_code(500),
        }
    }

    pub fn servir_avatar(legajo: i64, conn: Arc<Mutex<Connection>>) -> Response {
        match conn.lock() {
            Ok(guard) => match ImageRepository::buscar_avatar(&guard, legajo) {
                Ok(Some((blob, mime))) => Response::from_data(mime, blob),
                Ok(None) => Response::empty_404(),
                Err(e) => Response::text(e.to_string()).with_status_code(500),
            },
            Err(_) => Response::text("Mutex envenenado").with_status_code(500),
        }
    }
}
