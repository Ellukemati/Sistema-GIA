use rusqlite::{Result as SqlResult, Row};

/// Representa una imagen asociada a un ejemplar. Tiene un orden para mantener la secuencia de imagenes.
pub struct ImagenEjemplar {
    pub ejemplar_id: i64,
    pub orden: i32,
    pub imagen_direccion: String,
}

#[allow(dead_code)]
impl ImagenEjemplar {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ImagenEjemplar {
            ejemplar_id: row.get("ejemplar_id")?,
            orden: row.get("orden")?,
            imagen_direccion: row.get("imagen_direccion")?,
        })
    }
}
