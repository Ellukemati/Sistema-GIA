use crate::handlers::modelo_handler::ModeloHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/modelo) => {
            let conn_guard = conn.lock().unwrap();
            ModeloHandler::listar_modelos(&conn_guard)
        },

        (GET) (/modelo/registro) => {
            let conn_guard = conn.lock().unwrap();
            ModeloHandler::mostrar_formulario_registro(request, &conn_guard)
        },

        (POST) (/modelo/registro) => {
            let conn_guard = conn.lock().unwrap();
            ModeloHandler::procesar_registro(request, &conn_guard)
        },

        (GET) (/modelo/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            ModeloHandler::mostrar_detalle(&conn_guard, id)
        },

        _ => Response::empty_404()
    )
}
