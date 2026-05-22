mod db;
mod errors;
mod models;
mod server;
mod network;

const DB_PATH: &str = "gia.db";
const ADDRESS: &str = "0.0.0.0:8080";

fn main() {
    // Inicializar la base de datos
    let _conn = match db::init_db(DB_PATH) {
        Ok(c) => {
            println!("✓ Base de datos inicializada correctamente en: {}", DB_PATH);
            c
        }
        Err(e) => {
            eprintln!("✗ Error al inicializar la base de datos: {}", e);
            return;
        }
    };

    let server = server::Server::new(ADDRESS, _conn);
    server.run();
}
