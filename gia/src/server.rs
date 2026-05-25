use crate::routes::{auth_routes, image_routes, static_routes};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct Server {
    pub address: String,
    pub conn: Arc<Mutex<Connection>>,
}

impl Server {
    pub fn new(address: &str, conn: Connection) -> Self {
        Server {
            address: address.to_string(),
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn run(&self) {
        println!("Servidor escuchando en {}", self.address);
        let conn = Arc::clone(&self.conn);
        rouille::start_server(&self.address, move |request| {
            // Primero intentamos servir archivos estaticos para mejor performance
            if let Some(response) = static_routes::serve(request) {
                response
            } else if request.method() == "POST" && request.url().starts_with("/imagenes/") {
                image_routes::router(request, Arc::clone(&conn))
            } else {
                auth_routes::router(request, Arc::clone(&conn))
            }
        });
    }
}
