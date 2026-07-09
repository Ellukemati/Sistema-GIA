use dotenvy::dotenv;
use gia::{db, server};
use std::env;

fn main() {
    dotenv().ok();

    // Leer las variables (con un valor por defecto por si el .env falla
    let db_path = env::var("DB_PATH").unwrap_or_else(|_| "data/gia.db".to_string());
    let address = env::var("ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    let _conn = match db::init_db(&db_path) {
        Ok(c) => {
            println!("✓ Base de datos inicializada correctamente en: {}", db_path);
            c
        }
        Err(e) => {
            eprintln!("✗ Error al inicializar la base de datos: {}", e);
            return;
        }
    };

    let server = server::Server::new(&address, _conn);
    server.run();
}
