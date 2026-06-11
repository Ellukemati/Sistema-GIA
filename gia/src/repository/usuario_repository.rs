use rusqlite::{Connection, Result as SqlResult};

use crate::models::usuario::Usuario;

pub struct UsuarioRepository;

impl UsuarioRepository {
    pub fn buscar_por_email(conn: &Connection, email: &str) -> SqlResult<Option<Usuario>> {
        let mut stmt = conn.prepare("SELECT * FROM usuarios WHERE email = ?1")?;

        let mut rows = stmt.query([email])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Usuario::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn buscar_por_id(conn: &Connection, id: i64) -> SqlResult<Option<Usuario>> {
        let mut stmt = conn.prepare("SELECT * FROM usuarios WHERE id = ?1")?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Usuario::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn crear(conn: &Connection, usuario: &Usuario) -> SqlResult<usize> {
        conn.execute(
            "INSERT INTO usuarios
            (nombre, apellido, email, legajo, tipo, password_hash)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                usuario.nombre,
                usuario.apellido,
                usuario.email,
                usuario.legajo,
                usuario.tipo,
                usuario.password_hash,
            ],
        )
    }

    #[allow(dead_code)]
    pub fn actualizar_avatar(
        conn: &Connection,
        usuario_id: i64,
        avatar_blob: &[u8],
        avatar_mime: &str,
    ) -> SqlResult<usize> {
        conn.execute(
            "UPDATE usuarios
             SET avatar_blob = ?1, avatar_mime = ?2
             WHERE id = ?3",
            rusqlite::params![avatar_blob, avatar_mime, usuario_id],
        )
    }

    #[allow(dead_code)]
    pub fn eliminar(conn: &Connection, usuario_id: i64) -> SqlResult<usize> {
        conn.execute("DELETE FROM usuarios WHERE id = ?1", [usuario_id])
    }
}
