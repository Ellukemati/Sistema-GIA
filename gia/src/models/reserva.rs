use rusqlite::{Result as SqlResult, Row};

/// Representa una reserva segun la tabla `reservas`
pub struct Reserva {
    //pub id: i64,
    pub id_usuario: i64,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub estado: String,
    pub motivo: Option<String>,
    //pub momento_creacion: String,
}

impl Reserva {
    #[allow(dead_code)]
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Reserva {
            //id: row.get("id")?,
            id_usuario: row.get("id_usuario")?,
            fecha_inicio: row.get("fecha_inicio")?,
            fecha_fin: row.get("fecha_fin")?,
            estado: row.get("estado")?,
            motivo: row.get("motivo")?,
            //momento_creacion: row.get("momento_creacion")?,
        })
    }

    #[allow(dead_code)]
    /// Edita las fechas de la reserva
    pub fn editar_fechas(&mut self, nueva_fecha_inicio: String, nueva_fecha_fin: String) {
        self.fecha_inicio = nueva_fecha_inicio;
        self.fecha_fin = nueva_fecha_fin;
    }
}
