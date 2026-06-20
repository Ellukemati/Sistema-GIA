use crate::models::sesion::Sesion;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult};

pub struct SesionRepository;

impl SesionRepository {
    /// Guarda un nuevo token asociado a un usuario
    pub fn crear(conn: &Connection, token: &str, id_usuario: i64) -> SqlResult<usize> {
        conn.execute(
            "INSERT INTO sesiones (token, id_usuario) VALUES (?1, ?2)",
            rusqlite::params![token, id_usuario],
        )
    }

    /// Busca una sesión valida por su token
    pub fn buscar_por_token(conn: &Connection, token: &str) -> SqlResult<Option<Sesion>> {
        let mut stmt = conn.prepare("SELECT * FROM sesiones WHERE token = ?1")?;

        // OptionalExtension para convertir el error QueryReturnedNoRows directamente en un None limpio
        stmt.query_row([token], Sesion::from_row).optional()
    }

    /// Elimina la sesion de la base de datos (logout)
    pub fn eliminar_por_token(conn: &Connection, token: &str) -> SqlResult<usize> {
        conn.execute("DELETE FROM sesiones WHERE token = ?1", [token])
    }

    /// Elimina todas las sesiones que tengan más de 24 horas de antigüedad
    pub fn limpiar_expiradas(conn: &Connection) -> SqlResult<usize> {
        conn.execute(
            "DELETE FROM sesiones WHERE momento_creacion <= datetime('now', '-1 day')",
            [],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute(
            "CREATE TABLE usuarios (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                nombre TEXT, apellido TEXT, email TEXT, legajo INTEGER,
                tipo TEXT, password_hash TEXT, momento_creacion TEXT,
                avatar_blob BLOB, avatar_mime TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE sesiones (
                token TEXT PRIMARY KEY,
                id_usuario INTEGER NOT NULL,
                momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (id_usuario) REFERENCES usuarios(id) ON DELETE CASCADE
            )",
            [],
        )
        .unwrap();

        // Insertamos a Elon Musk
        conn.execute(
            "INSERT INTO usuarios (nombre, apellido, email, legajo, tipo, password_hash) 
             VALUES ('Elon', 'Musk', 'emusk@fi.uba.ar', 1, 'S', 'hash_spacex')",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_crear_y_buscar_sesion() {
        let conn = crear_db_test();
        let token = "token_tesla_secreto_123";
        let id_usuario = 1;

        SesionRepository::crear(&conn, token, id_usuario).unwrap();

        let sesion = SesionRepository::buscar_por_token(&conn, token)
            .unwrap()
            .expect("La sesión de Elon debería existir");

        assert_eq!(sesion.token, token);
        assert_eq!(sesion.id_usuario, id_usuario);
    }

    #[test]
    fn test_limpiar_expiradas_borra_solo_viejas() {
        let conn = crear_db_test();

        // Sesión expirada
        conn.execute(
            "INSERT INTO sesiones (token, id_usuario, momento_creacion) 
             VALUES ('token_viejo_twitter', 1, datetime('now', '-2 days'))",
            [],
        )
        .unwrap();

        // Sesión actual
        conn.execute(
            "INSERT INTO sesiones (token, id_usuario, momento_creacion) 
             VALUES ('token_nuevo_x', 1, datetime('now'))",
            [],
        )
        .unwrap();

        let borradas = SesionRepository::limpiar_expiradas(&conn).unwrap();

        assert_eq!(borradas, 1, "Debería borrar exactamente 1 sesión");

        assert!(
            SesionRepository::buscar_por_token(&conn, "token_viejo_twitter")
                .unwrap()
                .is_none()
        );
        assert!(
            SesionRepository::buscar_por_token(&conn, "token_nuevo_x")
                .unwrap()
                .is_some()
        );
    }
}
