use rusqlite::{Result as SqlResult, Row};

/// Representa una imagen asociada a un instrumento. Tiene un orden para mantener la secuencia de imagenes
pub struct InstrumentoImagen {
    pub instrumento_id: i64,
    pub orden: i32,
    pub imagen_url: String,
}

impl InstrumentoImagen {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(InstrumentoImagen {
            instrumento_id: row.get("instrumento_id")?,
            orden: row.get("orden")?,
            imagen_url: row.get("imagen_url")?,
        })
    }
}