mod db;
mod errors;
mod models;

const DB_PATH: &str = "gia.db";

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
}
