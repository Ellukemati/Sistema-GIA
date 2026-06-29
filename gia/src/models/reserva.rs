use rusqlite::{Result as SqlResult, Row};
use serde::Serialize;

/// Representa una reserva según la tabla `reservas` de la DB
#[derive(Serialize, Clone)]
pub struct Reserva {
    pub id: i64,
    pub id_usuario: i64,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub estado: String,
    pub motivo: Option<String>,
    pub momento_creacion: String,
    pub momento_confirmacion: Option<String>,
    pub id_admin_aprobador: Option<i64>,
}

impl Reserva {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Reserva {
            id: row.get("id")?,
            id_usuario: row.get("id_usuario")?,
            fecha_inicio: row.get("fecha_inicio")?,
            fecha_fin: row.get("fecha_fin")?,
            estado: row.get("estado")?,
            motivo: row.get("motivo")?,
            momento_creacion: row.get("momento_creacion")?,
            momento_confirmacion: row.get("momento_confirmacion")?,
            id_admin_aprobador: row.get("id_admin_aprobador")?,
        })
    }
}
