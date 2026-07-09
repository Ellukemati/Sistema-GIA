use dotenvy::dotenv;
use gia::{db, server};
use std::env;
pub mod logger;

fn main() {
    dotenv().ok();

    // Leer las variables (con un valor por defecto por si el .env falla
    let db_path = env::var("DB_PATH").unwrap_or_else(|_| "data/gia.db".to_string());
    let address = env::var("ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

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
