pub mod auth_routes;
pub mod modelo_instrumento_routes;

use rouille::{Request, Response};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn dispatch(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    let response = auth_routes::router(request, Arc::clone(&conn));
    if response.status_code != 404 {
        return response;
    }

    let response = modelo_instrumento_routes::router(request, Arc::clone(&conn));
    if response.status_code != 404 {
        return response;
    }

    Response::empty_404()
}