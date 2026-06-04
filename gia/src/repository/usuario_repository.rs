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
            (nombre, apellido, email, legajo, tipo, password_hash, imagen)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                usuario.nombre,
                usuario.apellido,
                usuario.email,
                usuario.legajo,
                usuario.tipo,
                usuario.password_hash,
                usuario.imagen,
            ],
        )
    }

    pub fn actualizar_imagen(conn: &Connection, usuario_id: i64, imagen: &str) -> SqlResult<usize> {
        conn.execute(
            "UPDATE usuarios
             SET imagen = ?1
             WHERE id = ?2",
            [imagen, &usuario_id.to_string()],
        )
    }

    pub fn eliminar(conn: &Connection, usuario_id: i64) -> SqlResult<usize> {
        conn.execute("DELETE FROM usuarios WHERE id = ?1", [usuario_id])
    }
}
