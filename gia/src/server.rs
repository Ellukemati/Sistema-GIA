use rusqlite::Connection;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use crate::routes;
use crate::service::pdf_worker_service::{PdfRequest, iniciar_pdf_worker};

pub struct Server {
    pub address: String,
    pub conn: Arc<Mutex<Connection>>,
    pub pdf_tx: SyncSender<PdfRequest>,
}

impl Server {
    pub fn new(address: &str, conn: Connection) -> Self {
        Server {
            address: address.to_string(),
            conn: Arc::new(Mutex::new(conn)),
            pdf_tx: iniciar_pdf_worker(),
        }
    }

    pub fn run(&self) {
        println!("Servidor escuchando en {}", self.address);
        let conn = Arc::clone(&self.conn);
        let pdf_tx = self.pdf_tx.clone();

        rouille::start_server(&self.address, move |request| {
            routes::dispatch(request, Arc::clone(&conn), pdf_tx.clone())
        });
    }
}
