use rusqlite::{Result as SqlResult, Row};

/// Representa la relacion entre una reserva y un instrumento
pub struct ReservaInstrumento {
    pub reserva_id: i64,
    pub instrumento_id: i64,
    pub cantidad: i32,
}

impl ReservaInstrumento {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ReservaInstrumento {
            reserva_id: row.get("reserva_id")?,
            instrumento_id: row.get("instrumento_id")?,
            cantidad: row.get("cantidad")?,
        })
    }
}