use rusqlite::{Result as SqlResult, Row};

/// Representa una cuenta segun la tabla `cuentas`
pub struct Cuenta {
    pub id: i64,
    pub nombre: String,
    pub segundo_nombre: Option<String>,
    pub apellido: String,
    pub segundo_apellido: Option<String>,
    pub email: String,
    pub legajo: i32,
    pub tipo: String,
    pub password_hash: String,
    pub momento_creacion: String,
}

impl Cuenta {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Cuenta {
            id: row.get("id")?,
            nombre: row.get("nombre")?,
            segundo_nombre: row.get("segundo_nombre")?,
            apellido: row.get("apellido")?,
            segundo_apellido: row.get("segundo_apellido")?,
            email: row.get("email")?,
            legajo: row.get("legajo")?,
            tipo: row.get("tipo")?,
            password_hash: row.get("password_hash")?,
            momento_creacion: row.get("momento_creacion")?,
        })
    }
}