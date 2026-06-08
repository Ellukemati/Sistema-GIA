use rusqlite::{Result as SqlResult, Row};

#[allow(dead_code)]
/// Representa una imagen asociada a un modelo de instrumento. Tiene un orden para mantener la secuencia de imagenes
pub struct ModeloImagen {
    pub modelo_id: i64,
    pub orden: i32,
    pub direccion_imagen: String,
}

#[allow(dead_code)]
impl ModeloImagen {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ModeloImagen {
            modelo_id: row.get("modelo_id")?,
            orden: row.get("orden")?,
            direccion_imagen: row.get("direccion_imagen")?,
        })
    }
}
