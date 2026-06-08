use rusqlite::{Result as SqlResult, Row};

/// Representa una imagen asociada a un modelo de instrumento. Tiene un orden para mantener la secuencia de imagenes
pub struct ImagenModelo {
    pub modelo_id: i64,
    pub orden: i32,
    pub direccion_imagen: String,
}

#[allow(dead_code)]
impl ImagenModelo {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ImagenModelo {
            modelo_id: row.get("modelo_id")?,
            orden: row.get("orden")?,
            direccion_imagen: row.get("direccion_imagen")?,
        })
    }
}
