use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use crate::handlers::reserva_handler::ReservaHandler;
use crate::service::pdf_worker_service::PdfRequest;

pub fn router(
    request: &Request,
    conn: Arc<Mutex<Connection>>,
    pdf_tx: SyncSender<PdfRequest>,
) -> Response {
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

        (GET) (/reservas/modelos/busqueda) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::buscar_modelos(
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

        (POST) (/reservas/carrito/motivo) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::actualizar_motivo_carrito(
                request,
                &conn_guard,
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

        (GET) (/mis-reservas) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::mostrar_mis_reservas(
                request,
                &conn_guard,
            )
        },

        (GET) (/mis-reservas/comprobante/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            ReservaHandler::descargar_comprobante_pdf(
                request,
                &conn_guard,
                id,
                pdf_tx.clone(),
            )
        },

        (POST) (/mis-reservas/cancelar/{id: i64}) => {

            let conn_guard =
                conn.lock().unwrap();

            ReservaHandler::cancelar_reserva(
                request,
                &conn_guard,
                id,
            )
        },

        _ => Response::empty_404()
    )
}
