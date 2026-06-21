use gia::{constants, db, server};

fn main() {
    let _conn = match db::init_db(constants::DB_PATH) {
        Ok(c) => {
            println!(
                "✓ Base de datos inicializada correctamente en: {}",
                constants::DB_PATH
            );
            c
        }
        Err(e) => {
            eprintln!("✗ Error al inicializar la base de datos: {}", e);
            return;
        }
    };

    let server = server::Server::new(constants::ADDRESS, _conn);
    server.run();
}
