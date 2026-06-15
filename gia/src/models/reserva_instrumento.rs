use rusqlite::{Result as SqlResult, Row};

/// Representa la relacion entre una reserva y un ejemplar
pub struct ReservaInstrumento {
    pub reserva_id: i64,
    pub ejemplar_id: i64,
}

impl ReservaInstrumento {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ReservaInstrumento {
            reserva_id: row.get("reserva_id")?,
            ejemplar_id: row.get("ejemplar_id")?,
        })
    }
}
