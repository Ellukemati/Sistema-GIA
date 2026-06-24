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

        (GET) (/ingreso) => {
            AuthHandler::mostrar_formulario_login()
        },

        (POST) (/ingreso) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_login(request, &conn_guard)
        },

        (GET) (/recuperar-contrasena) => {
            AuthHandler::mostrar_formulario_solicitud()
        },

        (POST) (/recuperar-contrasena) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_solicitud_recuperacion_password(request, &conn_guard)
        },

        (GET) (/restablecer-contrasena) => {
            AuthHandler::mostrar_formulario_cambio(request)
        },

        (POST) (/restablecer-contrasena) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_cambio_password(request, &conn_guard)
        },

        (GET) (/registro-invitacion) => {
            AuthHandler::mostrar_formulario_registro_invitacion(request)
        },

        (POST) (/registro-invitacion) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_alta_registro_invitacion(request, &conn_guard)
        },

        (GET) (/) => {
            AuthHandler::mostrar_bienvenida()
        },

        _ => Response::empty_404()
    )
}
