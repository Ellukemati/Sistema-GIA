use crate::handlers::manual_handler::ManualHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/manuales/{modelo_id}) => {
            ManualHandler::descargar_manual(modelo_id, Arc::clone(&conn))
        },

        (POST) (/manuales/{modelo_id}) => {
            ManualHandler::subir_manual(request, modelo_id, Arc::clone(&conn))
        },

        _ => Response::empty_404()
    )
}
