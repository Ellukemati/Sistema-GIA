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

        (GET) (/restablecer) => {
            AuthHandler::mostrar_formulario_solicitud()
        },

        (POST) (/restablecer) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_solicitud_restablecimiento_password(request, &conn_guard)
        },

        (GET) (/restablecer-contrasena) => {
            AuthHandler::mostrar_formulario_cambio(request)
        },

        (POST) (/restablecer-contrasena) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_cambio_password(request, &conn_guard)
        },

        (GET) (/) => {
            AuthHandler::mostrar_bienvenida()
        },

        (GET) (/inicio) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::mostrar_home(request, &conn_guard)
        },
        (GET) (/perfil) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::mostrar_perfil(request, &conn_guard)
        },

        (POST) (/perfil) => {
            let conn_guard = conn.lock().unwrap();

            AuthHandler::actualizar_perfil(
                request,
                &conn_guard,
            )
        },

        (GET) (/cerrar-sesion) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_logout(request, &conn_guard)
        },

        /*
        (GET) (/registro-invitacion) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::mostrar_formulario_registro_invitacion(request, &conn_guard)
        },

        (POST) (/registro-invitacion) => {
            let conn_guard = conn.lock().unwrap();
            AuthHandler::procesar_registro_invitacion(request, &conn_guard)
        },
        */

        _ => Response::empty_404()
    )
}
