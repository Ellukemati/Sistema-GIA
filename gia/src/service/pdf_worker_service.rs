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

        // Lee los logos una sola vez y los codifica en base64 para que puedan ser usados en la generación de PDFs
        let logo_fiuba =
            std::fs::read(crate::constants::PATH_LOGO_FIUBA_TRANSPARENTE).unwrap_or_default();
        let logo_agri =
            std::fs::read(crate::constants::PATH_LOGO_AGRIMENSURA_TRANSPARENTE).unwrap_or_default();

        use base64::{Engine as _, engine::general_purpose};
        let logo_fiuba_b64 = general_purpose::STANDARD.encode(logo_fiuba);
        let logo_agri_b64 = general_purpose::STANDARD.encode(logo_agri);

        while let Ok(req) = rx.recv() {
            let resultado = ComprobanteService::generar_pdf_con_app(
                &app,
                req.data,
                &logo_fiuba_b64,
                &logo_agri_b64,
            );
            let _ = req.responder.send(resultado);
        }
    });

    tx
}
