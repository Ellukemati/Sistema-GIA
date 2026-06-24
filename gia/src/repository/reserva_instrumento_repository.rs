use crate::models::reserva_instrumento::ReservaInstrumento;
use rusqlite::{Connection, Result as SqlResult};

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

    pub fn obtener_nombres_equipos_reserva(
        conn: &Connection,
        reserva_id: i64,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "
        SELECT
            modelos.nombre_modelo,
            ejemplares.numero_serie

        FROM reserva_ejemplar

        JOIN ejemplares
            ON ejemplares.id =
               reserva_ejemplar.ejemplar_id

        JOIN modelos
            ON modelos.id =
               ejemplares.modelo_id

        WHERE reserva_ejemplar.reserva_id = ?1
        ",
        )?;

        let equipos = stmt
            .query_map([reserva_id], |row| {
                let modelo: String = row.get(0)?;

                let serie: Option<String> = row.get(1)?;

                Ok(format!(
                    "{} ({})",
                    modelo,
                    serie.unwrap_or("Sin serie".to_string())
                ))
            })?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(equipos)
    }
}
