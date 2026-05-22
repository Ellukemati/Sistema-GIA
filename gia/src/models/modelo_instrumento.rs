use rusqlite::{Result as SqlResult, Row};

/// Representa un modelo de instrumento en la tabla `modelos_instrumentos`
pub struct ModeloInstrumento {
    pub id: i64,
    pub marca: Option<String>,
    pub nombre_modelo: String,
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
    pub manual_url: Option<String>,
    pub imagen_principal_url: Option<String>,
}

impl ModeloInstrumento {
    /// Crea un `ModeloInstrumento` a partir de una fila retornada por rusqlite.
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ModeloInstrumento {
            id: row.get("id")?,
            marca: row.get::<_, Option<String>>("marca")?,
            nombre_modelo: row.get("nombre_modelo")?,
            categoria: row.get::<_, Option<String>>("categoria")?,
            descripcion: row.get("descripcion")?,
            manual_url: row.get("manual_url")?,
            imagen_principal_url: row.get("imagen_principal_url")?,
        })
    }

    pub fn cambiar_imagen(&mut self, nueva_url: String) {
        self.imagen_principal_url = Some(nueva_url);
    }
    pub fn cambiar_manual(&mut self, nueva_url: String) {
        self.manual_url = Some(nueva_url);
    }
}
