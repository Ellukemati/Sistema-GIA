use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use crate::handlers::admin_handler::AdminHandler;
use crate::service::pdf_worker_service::PdfRequest;

pub fn router(
    request: &Request,
    conn: Arc<Mutex<Connection>>,
    pdf_tx: &SyncSender<PdfRequest>,
) -> Response {
    router!(request,
        (POST) (/admin/usuarios/cambiar-rol) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::procesar_cambio_rol(request, &conn_guard)
        },

        (GET) (/admin/solicitudes) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::mostrar_solicitudes(request, &conn_guard)
        },

        (GET) (/admin/tablas/recargar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::recargar_tablas_htmx(request, &conn_guard)
        },

        (POST) (/admin/reservas/aprobar/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::aprobar_reserva(request, &conn_guard, id, pdf_tx)
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
        (POST) (/admin/usuarios/{id: i64}/hacer-admin) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::hacer_admin(request, &conn_guard, id)
        },

        (POST) (/admin/usuarios/{id: i64}/quitar-admin) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::quitar_admin(request, &conn_guard, id)
        },

        // Endpoint para invitar usuarios no registrados a través de un correo electrónico institucional, como Administradores o Docentes
        /*
        (POST) (/admin/invitar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::procesar_envio_invitacion(request, &conn_guard)
        },
        */

        // Endpoint para enviar un comunicado general por mail a todos los usuarios, a un grupo específico o uno solo
        /*
        (POST) (/admin/comunicados/enviar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::enviar_notificacion_admin(request, &conn_guard)
        },
        */

        _ => Response::empty_404()
    )
}
