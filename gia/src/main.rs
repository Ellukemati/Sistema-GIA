pub mod logger;
use dotenvy::dotenv;
use gia::constants::{DB_PATH_DEFAULT, SERVER_ADDRESS_DEFAULT};
use gia::{db, server};
use std::env;

fn main() {
    dotenv().ok();

    let db_path = env::var("DB_PATH").unwrap_or_else(|_| DB_PATH_DEFAULT.to_string());
    let address = env::var("ADDRESS").unwrap_or_else(|_| SERVER_ADDRESS_DEFAULT.to_string());

    let _conn = match db::init_db(&db_path) {
        Ok(c) => {
            crate::logger::info(&format!(
                "Base de datos inicializada correctamente en: {}",
                db_path
            ));
            c
        }
        Err(e) => {
            crate::logger::error(&format!("Error al inicializar la base de datos: {}", e));
            return;
        }
    };

    crate::logger::info(&format!("Servidor GIA escuchando en: {}", address));
    let server = server::Server::new(&address, _conn);
    server.run();
}
