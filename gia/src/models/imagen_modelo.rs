use rusqlite::{Result as SqlResult, Row};

/// Representa una imagen asociada a un modelo de instrumento. Tiene un orden para mantener la secuencia de imagenes
pub struct ImagenModelo {
    pub modelo_id: i64,
    pub orden: i32,
    pub imagen_direccion: String,
}

impl ImagenModelo {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ImagenModelo {
            modelo_id: row.get("modelo_id")?,
            orden: row.get("orden")?,
            imagen_direccion: row.get("imagen_direccion")?,
        })
    }
}
