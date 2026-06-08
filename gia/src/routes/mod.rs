// Agregamos las rutas estáticas e imágenes en los imports
pub mod auth_routes;
pub mod ejemplar_routes;
pub mod image_routes;
pub mod modelo_routes;
pub mod reserva_routes;
pub mod static_routes;

use rouille::{Request, Response};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn dispatch(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    // 1. Prioridad alta: Archivos estáticos
    if let Some(response) = static_routes::serve(request) {
        return response;
    }

    // 2. Rutas especiales: Subida de imágenes
    if request.method() == "POST" && request.url().starts_with("/imagenes/") {
        return image_routes::router(request, Arc::clone(&conn));
    }

    // 3. Cascada de rutas de negocio (APIs y Formularios HTMX)
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

    // 4. Si nadie supo qué hacer con la ruta, se devuelve el 404 global
    Response::empty_404()
}
