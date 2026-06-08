use rusqlite::{Result as SqlResult, Row};

/// Representa un ejemplar en la tabla `ejemplares`
pub struct Ejemplar {
    //pub id: i64,
    pub modelo_id: i64,
    pub numero_serie: Option<String>,
    pub codigo_qr: Option<String>,
    pub patrimonio: Option<String>,
    pub observaciones: Option<String>,
    //pub accesorios: Option<String>,
    pub esta_disponible: bool,
    pub ubicacion: Option<String>,
    //pub direccion_imagen_principal: Option<String>,
}

impl Ejemplar {
    #[allow(dead_code)]
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        // La columna `esta_disponible` es 0 o 1 al ser boolean en la BDD, por eso se convierte a bool
        let disponible: i32 = row.get("esta_disponible")?;
        Ok(Ejemplar {
            //id: row.get("id")?,
            modelo_id: row.get("modelo_id")?,
            numero_serie: row.get("numero_serie")?,
            codigo_qr: row.get("codigo_qr")?,
            patrimonio: row.get("patrimonio")?,
            observaciones: row.get("observaciones")?,
            //accesorios: row.get("accesorios")?,
            esta_disponible: disponible != 0,
            ubicacion: row.get("ubicacion")?,
            //direccion_imagen_principal: row.get("direccion_imagen_principal")?,
        })
    }
}
