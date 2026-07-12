use crate::handlers::image_handler::ImageHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        // MODELOS
        (GET) (/imagenes/modelos/{modelo_id: i64}) => {
            ImageHandler::listar_modelo(modelo_id, Arc::clone(&conn))
        },
        (GET) (/imagenes/modelos/{modelo_id: i64}/{orden: i32}) => {
            ImageHandler::servir_modelo(modelo_id, orden, Arc::clone(&conn))
        },
        (POST) (/imagenes/modelos/{modelo_id: i64}/{orden: i32}) => {
            ImageHandler::subir_modelo(request, modelo_id, orden, Arc::clone(&conn))
        },

        // EJEMPLARES
        (GET) (/imagenes/ejemplares/{ejemplar_id: i64}/{orden: i32}) => {
            ImageHandler::servir_ejemplar(ejemplar_id, orden, Arc::clone(&conn))
        },
        (POST) (/imagenes/ejemplares/{ejemplar_id: i64}/{orden: i32}) => {
            ImageHandler::subir_ejemplar(request, ejemplar_id, orden, Arc::clone(&conn))
        },

        // USUARIOS
        (GET) (/imagenes/avatares/{legajo: i64}) => {
            ImageHandler::servir_avatar(legajo, Arc::clone(&conn))
        },
        (POST) (/imagenes/avatares/{legajo: i64}) => {
            ImageHandler::subir_avatar(request, legajo, Arc::clone(&conn))
        },

        _ => Response::empty_404()
    )
}
