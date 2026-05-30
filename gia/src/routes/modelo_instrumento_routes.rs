use crate::handlers::modelo_instrumento_handler::ModeloInstrumentoHandler;
use rouille::{router, Request, Response};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/modelo/registro) => {
            ModeloInstrumentoHandler::mostrar_formulario_registro()
        },
        
        (POST) (/modelo/registro) => {
            let conn_guard = conn.lock().unwrap();
            ModeloInstrumentoHandler::procesar_registro(request, &conn_guard)
        },
        
        _ => Response::empty_404()
    )
}