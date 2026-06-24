use crate::models::invitacion::Invitacion;
use rusqlite::{Connection, Result as SqliteResult, params};

pub struct InvitacionRepository;

impl InvitacionRepository {
    /// Guarda o actualiza una invitación activa indexada por el email institucional
    pub fn guardar(conn: &Connection, invitacion: &Invitacion) -> SqliteResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO tokens_invitacion (email, token, tipo, expira_en) 
             VALUES (?, ?, ?, ?)",
            params![
                invitacion.email,
                invitacion.token,
                invitacion.tipo,
                invitacion.expira_en
            ],
        )?;
        Ok(())
    }

    /// Busca una invitación asociada a un token que no haya expirado todavía
    pub fn buscar_valido(
        conn: &Connection,
        token: &str,
        ahora_segundos: i64,
    ) -> SqliteResult<Option<Invitacion>> {
        let mut stmt = conn.prepare(
            "SELECT email, token, tipo, expira_en FROM tokens_invitacion 
             WHERE token = ? AND expira_en > ?",
        )?;

        let mut rows = stmt.query(params![token, ahora_segundos])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Invitacion {
                email: row.get(0)?,
                token: row.get(1)?,
                tipo: row.get(2)?,
                expira_en: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Elimina la invitación asociada a un token, para limpiar después de usarla
    pub fn eliminar(conn: &Connection, token: &str) -> SqliteResult<()> {
        conn.execute(
            "DELETE FROM tokens_invitacion WHERE token = ?",
            params![token],
        )?;
        Ok(())
    }
}
