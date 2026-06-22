use crate::handlers::admin_handler::AdminHandler;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/admin/solicitudes) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::mostrar_solicitudes(request, &conn_guard)
        },

        (GET) (/admin/tablas/recargar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::recargar_tablas_htmx(request, &conn_guard)
        },

        (POST) (/admin/reservas/{id: i64}/aprobar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::aprobar_reserva(request, &conn_guard, id)
        },

        (POST) (/admin/reservas/{id: i64}/rechazar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::rechazar_reserva(request, &conn_guard, id)
        },

        (POST) (/admin/profesores/{id: i64}/aprobar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::aprobar_profesor(request, &conn_guard, id)
        },

        (POST) (/admin/profesores/{id: i64}/rechazar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::rechazar_profesor(request, &conn_guard, id)
        },

        // IDEA: Endpoint para enviar un comunicado general por mail a todos los usuarios, a un grupo específico o uno solo
        (POST) (/admin/comunicados/enviar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::enviar_notificacion_admin(request, &conn_guard)
        },

        _ => Response::empty_404()
    )
}
