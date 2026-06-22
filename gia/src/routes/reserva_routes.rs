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

        (GET) (/reservas/modelos) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::listar_modelos_disponibles(
                request,
                &conn_guard
            )
        },

        (GET) (/reservas/carrito) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::mostrar_carrito(
                request,
                &conn_guard
            )
        },

        (POST) (/reservas/carrito/remover/{ejemplar_id: i64}) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::remover_del_carrito(
                request,
                &conn_guard,
                ejemplar_id,
            )
        },

        (GET) (/reservas/modelo/{modelo_id: i64}) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::mostrar_ejemplares_modelo(
                request,
                &conn_guard,
                modelo_id,
            )
        },

        (POST) (/reservas/modelo/{modelo_id: i64}/agregar) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::agregar_al_carrito(
                request,
                &conn_guard,
                modelo_id,
            )
        },

        (POST) (/reservas/finalizar) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::finalizar_reserva(
                request,
                &conn_guard,
            )
        },

        _ => Response::empty_404()
    )
}
