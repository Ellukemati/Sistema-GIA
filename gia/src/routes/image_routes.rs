use crate::errors::ImageStorageError;
use crate::service::image_storage::{
    eliminar_imagen_por_direccion, guardar_avatar_con_legajo, guardar_imagen_ejemplar,
    guardar_imagen_modelo,
};
use rouille::{Request, Response};
use rusqlite::params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::io::Read;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    // Se esperan rutas del tipo:
    // POST /imagenes/avatares/{legajo}
    // POST /imagenes/modelos/{modelo_id}/{orden}
    // POST /imagenes/ejemplares/{ejemplar_id}/{orden}
    if request.method() != "POST" {
        return Response::empty_404();
    }

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

    match parts[0] {
        "avatares" => {
            if parts.len() != 2 {
                return Response::text("Ruta de avatar invalida. Se requiere /avatares/{legajo}")
                    .with_status_code(400);
            }
            subir_imagen_avatar_route(request, parts[1], conn)
        }
        "modelos" => {
            if parts.len() != 3 {
                return Response::text(
                    "Ruta de modelo invalida. Se requiere /modelos/{id}/{orden}",
                )
                .with_status_code(400);
            }
            subir_imagen_modelo_route(request, parts[1], parts[2], conn)
        }
        "ejemplares" => {
            if parts.len() != 3 {
                return Response::text(
                    "Ruta de ejemplar invalida. Se requiere /ejemplares/{id}/{orden}",
                )
                .with_status_code(400);
            }
            subir_imagen_ejemplar_route(request, parts[1], parts[2], conn)
        }
        _ => Response::empty_404(),
    }
}

fn read_body_bytes(request: &Request) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let Some(mut reader) = request.data() else {
        return Err("No se recibio ninguna imagen".to_string());
    };

    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("No se pudo leer la imagen: {}", e))?;

    if bytes.is_empty() {
        return Err("La imagen esta vacia".to_string());
    }

    Ok(bytes)
}

/// Maneja la subida de una imagen para un modelo_id y orden especificos, guardando la direccion en la DB
fn subir_imagen_modelo_route(
    request: &Request,
    modelo_id: &str,
    orden: &str,
    conn: Arc<Mutex<Connection>>,
) -> Response {
    let modelo = match modelo_id.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::text("modelo_id invalido").with_status_code(400),
    };

    let orden_i = match orden.parse::<i32>() {
        Ok(v) => v,
        Err(_) => return Response::text("orden invalido").with_status_code(400),
    };

    let bytes = match read_body_bytes(request) {
        Ok(b) => b,
        Err(mensaje) => return Response::text(mensaje).with_status_code(400),
    };

    match guardar_imagen_modelo(modelo, &bytes) {
        Ok(url) => {
            let mut imagen_a_borrar: Option<String> = None;

            match conn.lock() {
                Ok(guard) => {
                    let conn_ref: &Connection = &guard;
                    let imagen_anterior: Option<String> = conn_ref
                        .query_row(
                            "SELECT imagen_direccion FROM modelo_imagen WHERE modelo_id = ?1 AND orden = ?2",
                            params![modelo, orden_i],
                            |row| row.get(0),
                        )
                        .optional()
                        .unwrap_or(None);

                    if let Err(e) = conn_ref.execute(
                        "INSERT OR REPLACE INTO modelo_imagen (modelo_id, orden, imagen_direccion) VALUES (?1, ?2, ?3)",
                        params![modelo, orden_i, url.clone()],
                    ) {
                        return Response::from_data("text/plain", format!("Error guardando en DB: {}", e))
                            .with_status_code(500);
                    }

                    if orden_i == 0 {
                        if let Err(e) = conn_ref.execute(
                            "UPDATE modelos SET imagen_principal_direccion = ?1 WHERE id = ?2",
                            params![url.clone(), modelo],
                        ) {
                            return Response::from_data(
                                "text/plain",
                                format!("Error actualizando imagen principal del modelo: {}", e),
                            )
                            .with_status_code(500);
                        }
                    }

                    if let Some(direccion_anterior) = imagen_anterior {
                        if direccion_anterior != url {
                            imagen_a_borrar = Some(direccion_anterior);
                        }
                    }
                }
                Err(_) => {
                    return Response::from_data("text/plain", "Mutex envenenado")
                        .with_status_code(500);
                }
            }

            if let Some(direccion_vieja) = imagen_a_borrar {
                let _ = eliminar_imagen_por_direccion(&direccion_vieja);
            }

            Response::text(url)
        }
        Err(ImageStorageError::InvalidImage(msg)) => Response::text(msg).with_status_code(400),
        Err(err) => Response::from_data("text/plain", err.to_string()).with_status_code(500),
    }
}

/// Maneja la subida de una imagen para un ejemplar y un orden especificos, guardando la direccion en la DB
fn subir_imagen_ejemplar_route(
    request: &Request,
    ejemplar_id: &str,
    orden: &str,
    conn: Arc<Mutex<Connection>>,
) -> Response {
    let ejemplar = match ejemplar_id.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::text("ejemplar_id invalido").with_status_code(400),
    };

    let orden_i = match orden.parse::<i32>() {
        Ok(v) => v,
        Err(_) => return Response::text("orden invalido").with_status_code(400),
    };

    let bytes = match read_body_bytes(request) {
        Ok(b) => b,
        Err(mensaje) => return Response::text(mensaje).with_status_code(400),
    };

    match guardar_imagen_ejemplar(ejemplar, &bytes) {
        Ok(url) => {
            let mut imagen_a_borrar: Option<String> = None;

            match conn.lock() {
                Ok(guard) => {
                    let conn_ref: &Connection = &guard;
                    let imagen_anterior: Option<String> = conn_ref
                        .query_row(
                            "SELECT imagen_direccion FROM imagen_ejemplar WHERE ejemplar_id = ?1 AND orden = ?2",
                            params![ejemplar, orden_i],
                            |row| row.get(0),
                        )
                        .optional()
                        .unwrap_or(None);

                    if let Err(e) = conn_ref.execute(
                        "INSERT OR REPLACE INTO imagen_ejemplar (ejemplar_id, orden, imagen_direccion) VALUES (?1, ?2, ?3)",
                        params![ejemplar, orden_i, url.clone()],
                    ) {
                        return Response::from_data(
                            "text/plain",
                            format!("Error guardando en DB: {}", e),
                        )
                        .with_status_code(500);
                    }

                    if orden_i == 0 {
                        if let Err(e) = conn_ref.execute(
                            "UPDATE ejemplares SET direccion_imagen_principal = ?1 WHERE id = ?2",
                            params![url.clone(), ejemplar],
                        ) {
                            return Response::from_data(
                                "text/plain",
                                format!("Error actualizando imagen principal del ejemplar: {}", e),
                            )
                            .with_status_code(500);
                        }
                    }

                    if let Some(direccion_anterior) = imagen_anterior {
                        if direccion_anterior != url {
                            imagen_a_borrar = Some(direccion_anterior);
                        }
                    }
                }
                Err(_) => {
                    return Response::from_data("text/plain", "Mutex envenenado")
                        .with_status_code(500);
                }
            }

            if let Some(direccion_vieja) = imagen_a_borrar {
                let _ = eliminar_imagen_por_direccion(&direccion_vieja);
            }

            Response::text(url)
        }
        Err(ImageStorageError::InvalidImage(msg)) => Response::text(msg).with_status_code(400),
        Err(err) => Response::from_data("text/plain", err.to_string()).with_status_code(500),
    }
}

/// Maneja la subida de una imagen para un avatar, guardando la URL en la DB
fn subir_imagen_avatar_route(
    request: &Request,
    legajo: &str,
    conn: Arc<Mutex<Connection>>,
) -> Response {
    let leg = match legajo.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return Response::text("Legajo invalido").with_status_code(400),
    };

    let bytes = match read_body_bytes(request) {
        Ok(b) => b,
        Err(mensaje) => return Response::text(mensaje).with_status_code(400),
    };

    match guardar_avatar_con_legajo(leg, &bytes) {
        Ok(url) => {
            let mut avatar_a_borrar: Option<String> = None;

            match conn.lock() {
                Ok(guard) => {
                    let conn_ref: &Connection = &guard;
                    let avatar_anterior: Option<String> = conn_ref
                        .query_row(
                            "SELECT avatar_direccion FROM usuarios WHERE legajo = ?1",
                            params![leg],
                            |row| row.get(0),
                        )
                        .optional()
                        .unwrap_or(None);

                    if let Err(e) = conn_ref.execute(
                        "UPDATE usuarios SET avatar_direccion = ?1 WHERE legajo = ?2",
                        params![url.clone(), leg],
                    ) {
                        return Response::from_data(
                            "text/plain",
                            format!("Error guardando avatar en BD: {}", e),
                        )
                        .with_status_code(500);
                    }

                    if let Some(direccion_anterior) = avatar_anterior {
                        if direccion_anterior != url {
                            avatar_a_borrar = Some(direccion_anterior);
                        }
                    }
                }
                Err(_) => {
                    return Response::from_data("text/plain", "Mutex poisoned")
                        .with_status_code(500);
                }
            }

            if let Some(direccion_vieja) = avatar_a_borrar {
                let _ = eliminar_imagen_por_direccion(&direccion_vieja);
            }

            Response::text(url)
        }
        Err(ImageStorageError::InvalidImage(mensaje)) => {
            Response::text(mensaje).with_status_code(400)
        }
        Err(err) => Response::from_data("text/plain", err.to_string()).with_status_code(500),
    }
}
