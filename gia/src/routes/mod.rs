pub mod auth_routes;
pub mod ejemplar_routes;
pub mod modelo_routes;
pub mod reserva_routes;
pub mod image_routes;
pub mod static_routes;

use rouille::{Request, Response};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn dispatch(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    let response = auth_routes::router(request, Arc::clone(&conn));

    if response.status_code != 404 {
        return response;
    }

    let response = modelo_routes::router(request, Arc::clone(&conn));

    if response.status_code != 404 {
        return response;
    }

    let response = ejemplar_routes::router(request, Arc::clone(&conn));

    if response.status_code != 404 {
        return response;
    }

    let response = reserva_routes::router(request, Arc::clone(&conn));

    if response.status_code != 404 {
        return response;
    }

    Response::empty_404()
}