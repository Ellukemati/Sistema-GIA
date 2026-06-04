use crate::handlers::reserva_handler::ReservaHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,

        (GET) (/reservas/nueva) => {

            ReservaHandler::mostrar_formulario_reserva(
            )
        },

        (POST) (/reservas/nueva) => {

            let conn_guard = conn.lock().unwrap();

            ReservaHandler::procesar_reserva(
                request,
                &conn_guard
            )
        },

        _ => Response::empty_404()
    )
}
