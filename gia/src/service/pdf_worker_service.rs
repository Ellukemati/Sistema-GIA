use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread;
use wkhtmltopdf;

use crate::service::comprobante_service::{ComprobanteData, ComprobanteService};

pub struct PdfRequest {
    pub data: ComprobanteData,
    pub responder: oneshot::Sender<Result<Vec<u8>, String>>,
}

pub fn iniciar_pdf_worker() -> SyncSender<PdfRequest> {
    // Buffer de 8 de capacidad para encolar peticiones
    let (tx, rx) = sync_channel::<PdfRequest>(8);

    thread::spawn(move || {
        // La aplicacion nativa de C++ de wkhtmltopdf se inicializa una sola vez en la vida de la app
        let app = wkhtmltopdf::PdfApplication::new().expect("No se pudo inicializar wkhtmltopdf");

        while let Ok(req) = rx.recv() {
            let resultado = ComprobanteService::generar_pdf_con_app(&app, req.data);

            let _ = req.responder.send(resultado);
        }
    });

    tx
}
