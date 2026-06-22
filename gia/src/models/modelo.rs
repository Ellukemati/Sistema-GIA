use rusqlite::{Result as SqlResult, Row};
use serde::Serialize;

/// Representa un modelo de instrumento en la tabla `modelos`
#[derive(Serialize)]
pub struct Modelo {
    pub id: i64,
    pub marca: String,
    pub nombre_modelo: String,
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
}

impl Modelo {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Modelo {
            id: row.get("id")?,
            marca: row.get("marca")?,
            nombre_modelo: row.get("nombre_modelo")?,
            categoria: row.get("categoria")?,
            descripcion: row.get("descripcion")?,
        })
    }
}
