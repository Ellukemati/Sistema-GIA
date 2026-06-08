use crate::handlers::auth_handler::AuthHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/registro) => {
            AuthHandler::mostrar_formulario_registro()
        },

        (POST) (/registro) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_registro(request, &conn_guard)
        },

        (GET) (/login) => {
            AuthHandler::mostrar_formulario_login()
        },

        (POST) (/login) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_login(request, &conn_guard)
        },

        (GET) (/) => {
            Response::text("Bienvenido a GIA. Ve a /registro para crear una cuenta.")
        },

        _ => Response::empty_404()
    )
}