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

        (GET) (/admin/dashboard) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::mostrar_dashboard(request, &conn_guard)
        },

        (GET) (/admin/tablas/recargar) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::recargar_tablas_htmx(request, &conn_guard)
        },

        (GET) (/admin/reservas/previsualizar/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::previsualizar_comprobante(request, &conn_guard, id, pdf_tx)
        },

        (POST) (/admin/reservas/aprobar/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::aprobar_reserva(request, &conn_guard, id, pdf_tx)
        },

        (POST) (/admin/reservas/rechazar/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::rechazar_reserva(request, &conn_guard, id)
        },

        (GET) (/admin/historial-reservas/comprobante/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::descargar_comprobante_admin(request, &conn_guard, id, &pdf_tx.clone())
        },

        (POST) (/admin/profesores/aprobar/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::aprobar_profesor(request, &conn_guard, id)
        },

        (POST) (/admin/profesores/rechazar/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::rechazar_profesor(request, &conn_guard, id)
        },
        (POST) (/admin/usuarios/hacer-admin/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::hacer_admin(request, &conn_guard, id)
        },
        (GET) (/admin/historial-reservas) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::mostrar_historial_reservas(request, &conn_guard)
        },

        (POST) (/admin/usuarios/quitar-admin/{id: i64}) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::quitar_admin(request, &conn_guard, id)
        },
        (GET) (/admin/historial-reservas/csv) => {
            let conn_guard = conn.lock().unwrap();
            AdminHandler::exportar_historial_csv(request, &conn_guard)
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
