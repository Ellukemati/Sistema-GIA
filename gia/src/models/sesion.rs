use rusqlite::{Result as SqlResult, Row};

/// Representa una sesion activa en la tabla `sesiones`
pub struct Sesion {
    pub token: String,
    pub id_usuario: i64,
    pub momento_creacion: String,
}

impl Sesion {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Sesion {
            token: row.get("token")?,
            id_usuario: row.get("id_usuario")?,
            momento_creacion: row.get("momento_creacion")?,
        })
    }
}
