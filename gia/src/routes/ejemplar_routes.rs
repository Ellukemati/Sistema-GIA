use crate::handlers::ejemplar_handler::EjemplarHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/ejemplar/registro) => {
            let conn_guard = conn.lock().unwrap();
            EjemplarHandler::mostrar_formulario_registro(request, &conn_guard)
        },
        (GET) (/ejemplar/modelos/opciones) => {
            let conn_guard = conn.lock().unwrap();
            EjemplarHandler::listar_opciones_modelos(&conn_guard)
        },
        (POST) (/ejemplar/registro) => {
            let conn_guard = conn.lock().unwrap();
            EjemplarHandler::procesar_registro(request, &conn_guard)
        },
        (GET) (/ejemplar/{id: i64}/editar) => {
            let conn_guard = conn.lock().unwrap();
            EjemplarHandler::mostrar_formulario_edicion(request, &conn_guard, id)
        },
        (POST) (/ejemplar/{id: i64}/editar) => {
            let conn_guard = conn.lock().unwrap();
            EjemplarHandler::procesar_edicion(request, &conn_guard, id)
        },

        _ => Response::empty_404()
    )
}
