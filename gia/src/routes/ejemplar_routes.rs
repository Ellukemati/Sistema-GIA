use crate::handlers::ejemplar_handler::EjemplarHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/ejemplar/registro) => {
            let conn_guard = conn.lock().unwrap();
            EjemplarHandler::mostrar_formulario_registro(&conn_guard)
        },

        (POST) (/ejemplar/registro) => {
            let conn_guard = conn.lock().unwrap();
            EjemplarHandler::procesar_registro(request, &conn_guard)
        },

        _ => Response::empty_404()
    )
}
