use crate::models::sesion::Sesion;
use rustsqlite::{Connection, Result as SqlResult, OptionalExtension};

pub struct SesionRepository;

impl SesionRepository {
    /// Guarda un nuevo token asociado a un usuario
    pub fn crear(
        conn: &Connection,
        token: &str,
        usuario_id: i64,
    ) -> SqlResult<()> {
        conn.execute(
            "INSERT INTO sesiones (token, usuario_id) VALUES (?1, ?2)",
            rusqlite::params![token, usuario_id],
        )
    }

    /// Busca una sesión valida por su token
    pub fn buscar_por_token(
        conn: &Connection,
        token: &str,
    ) -> SqlResult<Option<Sesion>> {
        let mut stmt = conn.prepare("SELECT * FROM sesiones WHERE token = ?1")?;
        
        // OptionalExtension para convertir el error QueryReturnedNoRows directamente en un None limpio
        stmt.query_row([token], Sesion::from_row).optional()
    }

    /// Elimina la sesion de la base de datos (logout)
    pub fn eliminar_por_token(
        conn: &Connection,
        token: &str,
    ) -> SqlResult<usize> {
        conn.execute("DELETE FROM sesiones WHERE token = ?1", [token])
    } 
}