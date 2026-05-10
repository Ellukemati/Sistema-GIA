use rusqlite::{Row, Result as SqlResult};

/// Representa un instrumento segun la tabla `instrumentos`
pub struct Instrumento {
    pub id: i64,
    pub nombre: String,
    pub categoria: String,
    pub descripcion: Option<String>,
    pub stock: i32,
    pub estado: String,
    pub manual_url: Option<String>,
    pub imagen_principal_url: Option<String>,
}

impl Instrumento {
    /// Crea un `Instrumento` a partir de una fila retornada por rusqlite.
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let estado: String = row.get("estado")?;
        Ok(Instrumento {
            id: row.get("id")?,
            nombre: row.get("nombre")?,
            categoria: row.get("categoria")?,
            descripcion: row.get("descripcion")?,
            stock: row.get("stock")?,
            estado: estado,
            manual_url: row.get("manual_url")?,
            imagen_principal_url: row.get("imagen_principal_url")?,
        })
    }
}
