use rustsqlite::{Result as SqlResult, Row};

/// Representa una sesion activa en la tabla `sesiones`
pub struct Sesion {
    pub token: String,
    pub usuario_id: i64,
    pub momento_creacion: String,
}

impl Sesion {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Sesion {
            token: row.get("token")?,
            usuario_id: row.get("usuario_id")?,
            momento_creacion: row.get("momento_creacion")?,
        })
    }
}