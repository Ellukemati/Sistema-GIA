use crate::models::reserva::Reserva;
use rusqlite::{Connection, Result as SqlResult};

pub struct ReservaRepository;

impl ReservaRepository {
    pub fn crear(conn: &Connection, reserva: &Reserva) -> SqlResult<usize> {
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

    pub fn buscar_por_id(conn: &Connection, id: i64) -> SqlResult<Option<Reserva>> {
        let mut stmt = conn.prepare("SELECT * FROM reservas WHERE id = ?1")?;

        let resultado = stmt.query_row([id], Reserva::from_row);

        match resultado {
            Ok(reserva) => Ok(Some(reserva)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn listar_por_usuario(conn: &Connection, usuario_id: i64) -> SqlResult<Vec<Reserva>> {
        let mut stmt = conn.prepare(
            "SELECT *
             FROM reservas
             WHERE id_usuario = ?1
             ORDER BY fecha_inicio",
        )?;

        let filas = stmt.query_map([usuario_id], Reserva::from_row)?;

        let mut reservas = Vec::new();

        for reserva in filas {
            reservas.push(reserva?);
        }

        Ok(reservas)
    }

    pub fn cancelar(conn: &Connection, reserva_id: i64) -> SqlResult<usize> {
        conn.execute(
            "UPDATE reservas
             SET estado = 'cancelada'
             WHERE id = ?1",
            [reserva_id],
        )
    }

    pub fn listar_todas(conn: &Connection) -> SqlResult<Vec<Reserva>> {
        let mut stmt = conn.prepare(
            "SELECT *
                 FROM reservas
                 ORDER BY fecha_inicio",
        )?;

        let filas = stmt.query_map([], Reserva::from_row)?;

        let mut reservas = Vec::new();

        for reserva in filas {
            reservas.push(reserva?);
        }

        Ok(reservas)
    }
}


#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::reserva::Reserva;
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {

        let conn =
            Connection::open_in_memory()
                .unwrap();

        conn.execute(
            "CREATE TABLE reservas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                id_usuario INTEGER NOT NULL,
                fecha_inicio TEXT NOT NULL,
                fecha_fin TEXT NOT NULL,
                estado TEXT NOT NULL,
                motivo TEXT
            )",
            [],
        )
        .unwrap();

        conn
    }

    fn reserva_test() -> Reserva {

        Reserva {
            id_usuario: 1,
            fecha_inicio:
                "2026-07-01".to_string(),
            fecha_fin:
                "2026-07-05".to_string(),
            estado:
                "pendiente".to_string(),
            motivo:
                Some(
                    "Test".to_string()
                ),
        }
    }

    #[test]
    fn crear_reserva_guarda_una_fila() {

        let conn =
            crear_db_test();

        let reserva =
            reserva_test();

        let filas =
            ReservaRepository::crear(
                &conn,
                &reserva,
            )
            .unwrap();

        assert_eq!(
            filas,
            1
        );
    }

    #[test]
    fn buscar_por_id_devuelve_reserva() {

        let conn =
            crear_db_test();

        let reserva =
            reserva_test();

        ReservaRepository::crear(
            &conn,
            &reserva,
        )
        .unwrap();

        let resultado =
            ReservaRepository::buscar_por_id(
                &conn,
                1,
            )
            .unwrap();

        assert!(
            resultado.is_some()
        );
    }

    #[test]
    fn buscar_por_id_inexistente_devuelve_none() {

        let conn =
            crear_db_test();

        let resultado =
            ReservaRepository::buscar_por_id(
                &conn,
                999,
            )
            .unwrap();

        assert!(
            resultado.is_none()
        );
    }

    #[test]
    fn listar_por_usuario_devuelve_sus_reservas() {

        let conn =
            crear_db_test();

        let reserva =
            reserva_test();

        ReservaRepository::crear(
            &conn,
            &reserva,
        )
        .unwrap();

        let reservas =
            ReservaRepository::listar_por_usuario(
                &conn,
                1,
            )
            .unwrap();

        assert_eq!(
            reservas.len(),
            1
        );
    }

    #[test]
    fn cancelar_reserva_actualiza_estado() {

        let conn =
            crear_db_test();

        let reserva =
            reserva_test();

        ReservaRepository::crear(
            &conn,
            &reserva,
        )
        .unwrap();

        ReservaRepository::cancelar(
            &conn,
            1,
        )
        .unwrap();

        let estado: String =
            conn.query_row(
                "SELECT estado
                 FROM reservas
                 WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            estado,
            "cancelada"
        );
    }

    #[test]
    fn listar_todas_devuelve_todas_las_reservas() {

        let conn =
            crear_db_test();

        ReservaRepository::crear(
            &conn,
            &reserva_test(),
        )
        .unwrap();

        ReservaRepository::crear(
            &conn,
            &reserva_test(),
        )
        .unwrap();

        let reservas =
            ReservaRepository::listar_todas(
                &conn,
            )
            .unwrap();

        assert_eq!(
            reservas.len(),
            2
        );
    }
}