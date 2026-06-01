pub mod auth_routes;
pub mod modelo_instrumento_routes;
pub mod ejemplar_routes;
pub mod reserva_routes;

use rouille::{Request, Response};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

pub fn dispatch(
    request: &Request,
    conn: Arc<Mutex<Connection>>,
) -> Response {

    let response =
        auth_routes::router(request, Arc::clone(&conn));

    if response.status_code != 404 {
        return response;
    }

    let response =
        modelo_instrumento_routes::router(
            request,
            Arc::clone(&conn),
        );

    if response.status_code != 404 {
        return response;
    }

    let response =
        ejemplar_routes::router(
            request,
            Arc::clone(&conn),
        );

    if response.status_code != 404 {
        return response;
    }

    let response =
        reserva_routes::router(
            request,
            Arc::clone(&conn),
        );

    if response.status_code != 404 {
        return response;
    }

    Response::empty_404()
}