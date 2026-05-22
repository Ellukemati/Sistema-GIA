use crate::network::routes;
use rouille::Response;
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
            routes::handle(request, Arc::clone(&conn))
        });
    }
}