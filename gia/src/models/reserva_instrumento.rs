use rusqlite::{Result as SqlResult, Row};

#[allow(dead_code)]
/// Representa la relacion entre una reserva y un ejemplar
pub struct ReservaInstrumento {
    pub reserva_id: i64,
    pub ejemplar_id: i64,
}

impl ReservaInstrumento {
    #[allow(dead_code)]
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ReservaInstrumento {
            reserva_id: row.get("reserva_id")?,
            ejemplar_id: row.get("ejemplar_id")?,
        })
    }
}
