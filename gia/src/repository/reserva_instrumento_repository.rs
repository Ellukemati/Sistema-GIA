use rusqlite::{Connection, Result as SqlResult};

use crate::models::reserva_instrumento::ReservaInstrumento;
use crate::models::reserva_view::EquipoReservaView;

pub struct ReservaInstrumentoRepository;

impl ReservaInstrumentoRepository {
    pub fn crear(conn: &Connection, reserva_id: i64, ejemplar_id: i64) -> SqlResult<usize> {
        conn.execute(
            "INSERT INTO reserva_ejemplar
            (reserva_id, ejemplar_id)
            VALUES (?1, ?2)",
            rusqlite::params![reserva_id, ejemplar_id],
        )
    }

    pub fn obtener_por_reserva(
        conn: &Connection,
        reserva_id: i64,
    ) -> SqlResult<Vec<ReservaInstrumento>> {
        let mut stmt = conn.prepare(
            "SELECT *
             FROM reserva_ejemplar
             WHERE reserva_id = ?1",
        )?;

        let filas = stmt.query_map([reserva_id], ReservaInstrumento::from_row)?;

        let mut relaciones = Vec::new();

        for relacion in filas {
            relaciones.push(relacion?);
        }

        Ok(relaciones)
    }

    pub fn obtener_detalle_equipos_reserva(
        conn: &Connection,
        reserva_id: i64,
    ) -> Result<Vec<EquipoReservaView>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "
        SELECT
            modelos.nombre_modelo,
            ejemplares.codigo_qr,
            ejemplares.numero_serie,
            ejemplares.patrimonio,
            ejemplares.id

        FROM reserva_ejemplar

        JOIN ejemplares
            ON ejemplares.id = reserva_ejemplar.ejemplar_id

        JOIN modelos
            ON modelos.id = ejemplares.modelo_id

        WHERE reserva_ejemplar.reserva_id = ?1
        ",
        )?;

        let equipos = stmt
            .query_map([reserva_id], |row| {
                Ok(EquipoReservaView {
                    nombre: row.get(0)?,
                    codigo_qr: row.get(1)?,
                    numero_serie: row.get(2)?,
                    patrimonio: row.get(3)?,
                    id_interno: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<EquipoReservaView>, _>>()?;

        Ok(equipos)
    }
}
