use crate::handlers::auth_handler::AuthHandler;
use crate::handlers::ejemplar_handler::EjemplarHandler;
use crate::handlers::modelo_handler::ModeloHandler;
use crate::handlers::reserva_handler::ReservaHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/registro) => {
            AuthHandler::mostrar_formulario_registro()
        },

        (GET) (/modelo/registro) => {
            ModeloHandler::mostrar_formulario_registro()
        },

        (GET) (/ejemplar/registro) => {
            let conn_guard = conn.lock().unwrap();
            EjemplarHandler::mostrar_formulario_registro(&conn_guard)
        },

        (GET) (/reservas/nueva) => {
            ReservaHandler::mostrar_formulario_reserva()
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