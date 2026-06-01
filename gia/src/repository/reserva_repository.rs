use crate::models::reserva::Reserva;
use rusqlite::{Connection, Result as SqlResult};

pub struct ReservaRepository;

impl ReservaRepository {
    pub fn crear(
        conn: &Connection,
        reserva: &Reserva,
    ) -> SqlResult<usize> {
        conn.execute(
            "INSERT INTO reservas
            (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                reserva.id_usuario,
                reserva.fecha_inicio,
                reserva.fecha_fin,
                reserva.estado,
                reserva.motivo,
            ],
        )
    }

    pub fn buscar_por_id(
        conn: &Connection,
        id: i64,
    ) -> SqlResult<Option<Reserva>> {
        let mut stmt =
            conn.prepare("SELECT * FROM reservas WHERE id = ?1")?;

        let resultado = stmt.query_row(
            [id],
            Reserva::from_row,
        );

        match resultado {
            Ok(reserva) => Ok(Some(reserva)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn listar_por_usuario(
        conn: &Connection,
        usuario_id: i64,
    ) -> SqlResult<Vec<Reserva>> {
        let mut stmt = conn.prepare(
            "SELECT *
             FROM reservas
             WHERE id_usuario = ?1
             ORDER BY fecha_inicio"
        )?;

        let filas =
            stmt.query_map([usuario_id], Reserva::from_row)?;

        let mut reservas = Vec::new();

        for reserva in filas {
            reservas.push(reserva?);
        }

        Ok(reservas)
    }

    pub fn cancelar(
        conn: &Connection,
        reserva_id: i64,
    ) -> SqlResult<usize> {
        conn.execute(
            "UPDATE reservas
             SET estado = 'cancelada'
             WHERE id = ?1",
            [reserva_id],
        )
    }

    pub fn listar_todas(
        conn: &Connection,
    ) -> SqlResult<Vec<Reserva>> {
        let mut stmt =
            conn.prepare(
                "SELECT *
                 FROM reservas
                 ORDER BY fecha_inicio"
            )?;

        let filas =
            stmt.query_map([], Reserva::from_row)?;

        let mut reservas = Vec::new();

        for reserva in filas {
            reservas.push(reserva?);
        }

        Ok(reservas)
    }
}