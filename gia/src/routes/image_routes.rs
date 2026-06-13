use crate::errors::ImageStorageError;
use crate::service::image_storage::{procesar_avatar, procesar_ejemplar, procesar_modelo};
use rouille::{Request, Response};
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::params;
use std::io::Read;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    let raw = request.url();
    let path = raw.split('?').next().unwrap_or("");
    let tail = match path.strip_prefix("/imagenes/") {
        Some(t) => t,
        None => return Response::empty_404(),
    };

    let parts: Vec<&str> = tail.split('/').collect();
    if parts.is_empty() {
        return Response::empty_404();
    }

    match (request.method(), parts[0]) {
        ("POST", "avatares") => {
            if parts.len() != 2 {
                return Response::empty_404();
            }
            subir_avatar_route(request, parts[1], conn)
        }
        ("POST", "modelos") => {
            if parts.len() != 3 {
                return Response::empty_404();
            }
            subir_modelo_route(request, parts[1], parts[2], conn)
        }
        ("POST", "ejemplares") => {
            if parts.len() != 3 {
                return Response::empty_404();
            }
            subir_ejemplar_route(request, parts[1], parts[2], conn)
        }
        ("GET", "avatares") => {
            if parts.len() != 2 {
                return Response::empty_404();
            }
            servir_avatar_route(parts[1], conn)
        }
        ("GET", "modelos") => {
            if parts.len() != 3 {
                return Response::empty_404();
            }
            servir_modelo_route(parts[1], parts[2], conn)
        }
        ("GET", "ejemplares") => {
            if parts.len() != 3 {
                return Response::empty_404();
            }
            servir_ejemplar_route(parts[1], parts[2], conn)
        }
        _ => Response::empty_404(),
    }
}

fn read_body_bytes(request: &Request) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let Some(mut reader) = request.data() else {
        return Err("No se recibio ninguna imagen".to_string());
    };
    reader.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Err("Vacia".to_string());
    }
    Ok(bytes)
}

fn subir_modelo_route(
    request: &Request,
    modelo_id: &str,
    orden: &str,
    conn: Arc<Mutex<Connection>>,
) -> Response {
    let m_id = match modelo_id.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };
    let ord = match orden.parse::<i32>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };

    let bytes = match read_body_bytes(request) {
        Ok(b) => b,
        Err(m) => return Response::text(m).with_status_code(400),
    };

    match procesar_modelo(&bytes) {
        Ok((blob_final, mime)) => {
            match conn.lock() {
                Ok(guard) => {
                    let conn_ref: &Connection = &guard;
                    if let Err(e) = conn_ref.execute(
                        "INSERT OR REPLACE INTO modelo_imagen (modelo_id, orden, imagen_blob, imagen_mime) VALUES (?1, ?2, ?3, ?4)",
                        params![m_id, ord, blob_final, mime],
                    ) {
                        return Response::text(e.to_string()).with_status_code(500);
                    }
                }
                Err(_) => return Response::text("Mutex envenenado").with_status_code(500),
            }
            Response::text(format!("/imagenes/modelos/{}/{}", m_id, ord))
        }
        Err(ImageStorageError::InvalidImage(msg)) => Response::text(msg).with_status_code(400),
        Err(err) => Response::text(err.to_string()).with_status_code(500),
    }
}

fn servir_modelo_route(modelo_id: &str, orden: &str, conn: Arc<Mutex<Connection>>) -> Response {
    let m_id = match modelo_id.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };
    let ord = match orden.parse::<i32>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };

    match conn.lock() {
        Ok(guard) => {
            let conn_ref: &Connection = &guard;
            let datos: Option<(Vec<u8>, String)> = conn_ref
                .query_row(
                    "SELECT imagen_blob, imagen_mime FROM modelo_imagen WHERE modelo_id = ?1 AND orden = ?2",
                    params![m_id, ord],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .unwrap_or(None);

            if let Some((blob, mime)) = datos {
                Response::from_data(mime, blob)
            } else {
                Response::empty_404()
            }
        }
        Err(_) => Response::text("Mutex envenenado").with_status_code(500),
    }
}

fn subir_ejemplar_route(
    request: &Request,
    ejemplar_id: &str,
    orden: &str,
    conn: Arc<Mutex<Connection>>,
) -> Response {
    let e_id = match ejemplar_id.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };
    let ord = match orden.parse::<i32>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };

    let bytes = match read_body_bytes(request) {
        Ok(b) => b,
        Err(m) => return Response::text(m).with_status_code(400),
    };

    match procesar_ejemplar(&bytes) {
        Ok((blob_final, mime)) => {
            match conn.lock() {
                Ok(guard) => {
                    let conn_ref: &Connection = &guard;
                    if let Err(e) = conn_ref.execute(
                        "INSERT OR REPLACE INTO ejemplar_imagen (ejemplar_id, orden, imagen_blob, imagen_mime) VALUES (?1, ?2, ?3, ?4)",
                        params![e_id, ord, blob_final, mime],
                    ) {
                        return Response::text(e.to_string()).with_status_code(500);
                    }
                }
                Err(_) => return Response::text("Mutex envenenado").with_status_code(500),
            }
            Response::text(format!("/imagenes/ejemplares/{}/{}", e_id, ord))
        }
        Err(ImageStorageError::InvalidImage(msg)) => Response::text(msg).with_status_code(400),
        Err(err) => Response::text(err.to_string()).with_status_code(500),
    }
}

fn servir_ejemplar_route(ejemplar_id: &str, orden: &str, conn: Arc<Mutex<Connection>>) -> Response {
    let e_id = match ejemplar_id.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };
    let ord = match orden.parse::<i32>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };

    match conn.lock() {
        Ok(guard) => {
            let conn_ref: &Connection = &guard;
            let datos: Option<(Vec<u8>, String)> = conn_ref
                .query_row(
                    "SELECT imagen_blob, imagen_mime FROM ejemplar_imagen WHERE ejemplar_id = ?1 AND orden = ?2",
                    params![e_id, ord],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .unwrap_or(None);

            if let Some((blob, mime)) = datos {
                Response::from_data(mime, blob)
            } else {
                Response::empty_404()
            }
        }
        Err(_) => Response::text("Mutex envenenado").with_status_code(500),
    }
}

fn subir_avatar_route(request: &Request, legajo: &str, conn: Arc<Mutex<Connection>>) -> Response {
    let leg = match legajo.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };

    let bytes = match read_body_bytes(request) {
        Ok(b) => b,
        Err(m) => return Response::text(m).with_status_code(400),
    };

    match procesar_avatar(&bytes) {
        Ok((blob_final, mime)) => {
            match conn.lock() {
                Ok(guard) => {
                    let conn_ref: &Connection = &guard;
                    if let Err(e) = conn_ref.execute(
                        "UPDATE usuarios SET avatar_blob = ?1, avatar_mime = ?2 WHERE legajo = ?3",
                        params![blob_final, mime, leg],
                    ) {
                        return Response::text(e.to_string()).with_status_code(500);
                    }
                }
                Err(_) => return Response::text("Mutex envenenado").with_status_code(500),
            }
            Response::text(format!("/imagenes/avatares/{}", leg))
        }
        Err(ImageStorageError::InvalidImage(msg)) => Response::text(msg).with_status_code(400),
        Err(err) => Response::text(err.to_string()).with_status_code(500),
    }
}

fn servir_avatar_route(legajo: &str, conn: Arc<Mutex<Connection>>) -> Response {
    let leg = match legajo.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::empty_404(),
    };

    match conn.lock() {
        Ok(guard) => {
            let conn_ref: &Connection = &guard;
            let datos: Option<(Vec<u8>, String)> = conn_ref
                .query_row(
                    "SELECT avatar_blob, avatar_mime FROM usuarios WHERE legajo = ?1",
                    params![leg],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .unwrap_or(None);

            if let Some((blob, mime)) = datos {
                Response::from_data(mime, blob)
            } else {
                Response::empty_404()
            }
        }
        Err(_) => Response::text("Mutex envenenado").with_status_code(500),
    }
}
