use crate::handlers::reserva_handler::ReservaHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,

        (GET) (/reservas) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::mostrar_formulario_reserva(
                request,
                &conn_guard
            )
        },

        (GET) (/reservas/modelo/{modelo_id: i64}) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::mostrar_ejemplares_modelo(
                &conn_guard,
                modelo_id,
            )
        },

        (POST) (/reservas/nueva) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::procesar_reserva(
                request,
                &conn_guard,
            )
        },

        _ => Response::empty_404()
    )
}
