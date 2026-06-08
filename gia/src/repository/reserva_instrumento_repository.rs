use crate::models::reserva_instrumento::ReservaInstrumento;
use rusqlite::{Connection, Result as SqlResult};

pub struct ReservaInstrumentoRepository;

impl ReservaInstrumentoRepository {
    pub fn crear(conn: &Connection, reserva_id: i64, ejemplar_id: i64) -> SqlResult<usize> {
        conn.execute(
            "INSERT INTO reserva_instrumentos
            (reserva_id, ejemplar_id)
            VALUES (?1, ?2)",
            rusqlite::params![reserva_id, ejemplar_id],
        )
    }

    #[allow(dead_code)]
    pub fn obtener_por_reserva(
        conn: &Connection,
        reserva_id: i64,
    ) -> SqlResult<Vec<ReservaInstrumento>> {
        let mut stmt = conn.prepare(
            "SELECT *
             FROM reserva_instrumentos
             WHERE reserva_id = ?1",
        )?;

        let filas = stmt.query_map([reserva_id], ReservaInstrumento::from_row)?;

        let mut relaciones = Vec::new();

        for relacion in filas {
            relaciones.push(relacion?);
        }

        Ok(relaciones)
    }
}
