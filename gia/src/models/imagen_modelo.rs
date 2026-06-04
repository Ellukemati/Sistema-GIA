use rusqlite::{Result as SqlResult, Row};

/// Representa una imagen asociada a un modelo de instrumento. Tiene un orden para mantener la secuencia de imagenes
pub struct ModeloImagen {
    pub modelo_id: i64,
    pub orden: i32,
    pub imagen_direccion: String,
}

impl ModeloImagen {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ModeloImagen {
            modelo_id: row.get("modelo_id")?,
            orden: row.get("orden")?,
            imagen_direccion: row.get("imagen_direccion")?,
        })
    }
}
