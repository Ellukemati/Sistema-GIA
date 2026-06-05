use rusqlite::{Result as SqlResult, Row};

/// Representa un modelo de instrumento en la tabla `modelos` (antes `modelos_instrumentos`).
pub struct Modelo {
    pub id: i64,
    pub marca: Option<String>,
    pub modelo: String,
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
    pub manual_url: Option<String>,
    pub direccion_imagen_principal: Option<String>,
}

impl Modelo {
    /// Crea un `Modelo` a partir de una fila retornada por rusqlite.
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Modelo {
            id: row.get("id")?,
            marca: row.get::<_, Option<String>>("marca")?,
            modelo: row.get("modelo")?,
            categoria: row.get::<_, Option<String>>("categoria")?,
            descripcion: row.get("descripcion")?,
            manual_url: row.get("manual_url")?,
            direccion_imagen_principal: row.get("direccion_imagen_principal")?,
        })
    }

    pub fn cambiar_imagen(&mut self, nueva_direccion: String) {
        self.direccion_imagen_principal = Some(nueva_direccion);
    }
    pub fn cambiar_manual(&mut self, nueva_url: String) {
        self.manual_url = Some(nueva_url);
    }
}
