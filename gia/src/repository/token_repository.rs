use rusqlite::{Connection, Result as SqliteResult, params};

pub struct TokenRepository;

impl TokenRepository {
    /// Guarda un token para un usuario. Si ya existía un token activo para ese id_usuario,
    /// se pisa automáticamente en la misma PRIMARY KEY.
    pub fn guardar(
        conn: &Connection,
        id_usuario: i64,
        token: &str,
        expira_en: i64,
    ) -> SqliteResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO tokens_recuperacion (id_usuario, token, expira_en) 
             VALUES (?, ?, ?)",
            params![id_usuario, token, expira_en],
        )?;
        Ok(())
    }

    /// Busca un registro que coincida con el token enviado y que su tiempo de expiración sea mayor que el tiempo actual
    pub fn buscar_valido(
        conn: &Connection,
        token: &str,
        ahora_segundos: i64,
    ) -> SqliteResult<Option<i64>> {
        let mut stmt = conn.prepare(
            "SELECT id_usuario FROM tokens_recuperacion 
             WHERE token = ? AND expira_en > ?",
        )?;

        let mut rows = stmt.query(params![token, ahora_segundos])?;

        if let Some(row) = rows.next()? {
            let id_usuario: i64 = row.get(0)?;
            Ok(Some(id_usuario))
        } else {
            Ok(None)
        }
    }

    /// Elimina el token de la base de datos una vez que ya cumplió su función
    pub fn eliminar(conn: &Connection, id_usuario: i64) -> SqliteResult<()> {
        conn.execute(
            "DELETE FROM tokens_recuperacion WHERE id_usuario = ?",
            params![id_usuario],
        )?;
        Ok(())
    }
}
