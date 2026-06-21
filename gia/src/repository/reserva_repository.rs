use crate::models::reserva::Reserva;
use rusqlite::{Connection, Result as SqlResult, params};

pub struct ReservaRepository;

impl ReservaRepository {
    pub fn crear(conn: &Connection, reserva: &Reserva) -> SqlResult<i64> {
        conn.execute(
            "INSERT INTO reservas (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                reserva.id_usuario,
                reserva.fecha_inicio,
                reserva.fecha_fin,
                reserva.estado,
                reserva.motivo,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn buscar_por_id(conn: &Connection, id: i64) -> SqlResult<Option<Reserva>> {
        let mut stmt = conn.prepare(
            "SELECT id, id_usuario, fecha_inicio, fecha_fin, estado, motivo, momento_creacion 
             FROM reservas WHERE id = ?1",
        )?;

        let resultado = stmt.query_row([id], Reserva::from_row);

        match resultado {
            Ok(reserva) => Ok(Some(reserva)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn listar_por_usuario(conn: &Connection, usuario_id: i64) -> SqlResult<Vec<Reserva>> {
        let mut stmt = conn.prepare(
            "SELECT id, id_usuario, fecha_inicio, fecha_fin, estado, motivo, momento_creacion
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

    pub fn cancelar_por_usuario(
        conn: &Connection,
        reserva_id: i64,
        usuario_id: i64,
    ) -> SqlResult<usize> {
        conn.execute(
            "
            UPDATE reservas
            SET estado = 'cancelada'
            WHERE id = ?1
            AND id_usuario = ?2
            AND estado != 'cancelada'
            ",
            rusqlite::params![reserva_id, usuario_id,],
        )
    }

    pub fn listar_todas(conn: &Connection) -> SqlResult<Vec<Reserva>> {
        let mut stmt = conn.prepare(
            "SELECT id, id_usuario, fecha_inicio, fecha_fin, estado, motivo, momento_creacion
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

    pub fn ejemplar_disponible(
        conn: &Connection,
        ejemplar_id: i64,
        fecha_inicio: &str,
        fecha_fin: &str,
    ) -> SqlResult<bool> {
        let cantidad: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM reservas r
             INNER JOIN reserva_ejemplar re ON r.id = re.reserva_id
             WHERE re.ejemplar_id = ?1
               AND r.estado != 'cancelada'
               AND (r.fecha_inicio <= ?2 AND r.fecha_fin >= ?3)",
            params![ejemplar_id, fecha_fin, fecha_inicio],
            |row| row.get(0),
        )?;

        Ok(cantidad == 0)
    }

    pub fn listar_por_estado(conn: &Connection, estado: &str) -> SqlResult<Vec<Reserva>> {
        let mut stmt = conn.prepare(
            "SELECT id, id_usuario, fecha_inicio, fecha_fin, estado, motivo, momento_creacion
             FROM reservas WHERE estado = ?1 ORDER BY momento_creacion ASC",
        )?;
        let filas = stmt.query_map([estado], Reserva::from_row)?;
        let mut reservas = Vec::new();
        for r in filas {
            reservas.push(r?);
        }
        Ok(reservas)
    }

    pub fn cambiar_estado(
        conn: &Connection,
        reserva_id: i64,
        nuevo_estado: &str,
    ) -> SqlResult<usize> {
        conn.execute(
            "UPDATE reservas SET estado = ?1 WHERE id = ?2",
            rusqlite::params![nuevo_estado, reserva_id],
        )
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::reserva::Reserva;
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE reserva_ejemplar (
            reserva_id INTEGER NOT NULL,
            ejemplar_id INTEGER NOT NULL,
            PRIMARY KEY(reserva_id, ejemplar_id)
        )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE reservas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                id_usuario INTEGER NOT NULL,
                fecha_inicio TEXT NOT NULL,
                fecha_fin TEXT NOT NULL,
                estado TEXT NOT NULL,
                motivo TEXT,
                momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .unwrap();

        conn
    }

    fn reserva_test() -> Reserva {
        Reserva {
            id: 0,
            id_usuario: 1,
            fecha_inicio: "2026-07-01".to_string(),
            fecha_fin: "2026-07-05".to_string(),
            estado: "pendiente".to_string(),
            motivo: Some("Test".to_string()),
            momento_creacion: "2026-06-25T12:00:00".to_string(),
        }
    }

    #[test]
    fn crear_reserva_guarda_una_fila() {
        let conn = crear_db_test();
        let reserva = reserva_test();

        let id_generado = ReservaRepository::crear(&conn, &reserva).unwrap();

        assert_eq!(id_generado, 1);
    }

    #[test]
    fn ejemplar_disponible_si_no_tiene_reservas() {
        let conn = crear_db_test();

        let disponible =
            ReservaRepository::ejemplar_disponible(&conn, 1, "2026-07-01", "2026-07-05").unwrap();

        assert!(disponible);
    }

    #[test]
    fn ejemplar_no_disponible_si_ya_esta_reservado() {
        let conn = crear_db_test();

        ReservaRepository::crear(&conn, &reserva_test()).unwrap();

        conn.execute(
            "INSERT INTO reserva_ejemplar
        (reserva_id, ejemplar_id)
        VALUES (1,1)",
            [],
        )
        .unwrap();

        let disponible =
            ReservaRepository::ejemplar_disponible(&conn, 1, "2026-07-03", "2026-07-08").unwrap();

        assert!(!disponible);
    }

    #[test]
    fn ejemplar_disponible_si_fechas_no_se_superponen() {
        let conn = crear_db_test();

        ReservaRepository::crear(&conn, &reserva_test()).unwrap();

        conn.execute(
            "INSERT INTO reserva_ejemplar
        (reserva_id, ejemplar_id)
        VALUES (1,1)",
            [],
        )
        .unwrap();

        let disponible =
            ReservaRepository::ejemplar_disponible(&conn, 1, "2026-08-01", "2026-08-05").unwrap();

        assert!(disponible);
    }

    #[test]
    fn buscar_por_id_devuelve_reserva() {
        let conn = crear_db_test();

        let reserva = reserva_test();

        ReservaRepository::crear(&conn, &reserva).unwrap();

        let resultado = ReservaRepository::buscar_por_id(&conn, 1).unwrap();

        assert!(resultado.is_some());
    }

    #[test]
    fn buscar_por_id_inexistente_devuelve_none() {
        let conn = crear_db_test();

        let resultado = ReservaRepository::buscar_por_id(&conn, 999).unwrap();

        assert!(resultado.is_none());
    }

    #[test]
    fn listar_por_usuario_devuelve_sus_reservas() {
        let conn = crear_db_test();

        let reserva = reserva_test();

        ReservaRepository::crear(&conn, &reserva).unwrap();

        let reservas = ReservaRepository::listar_por_usuario(&conn, 1).unwrap();

        assert_eq!(reservas.len(), 1);
    }

    #[test]
    fn cancelar_reserva_actualiza_estado() {
        let conn = crear_db_test();

        let reserva = reserva_test();

        ReservaRepository::crear(&conn, &reserva).unwrap();

        ReservaRepository::cancelar(&conn, 1).unwrap();

        let estado: String = conn
            .query_row(
                "SELECT estado
                 FROM reservas
                 WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(estado, "cancelada");
    }

    #[test]
    fn listar_todas_devuelve_todas_las_reservas() {
        let conn = crear_db_test();

        ReservaRepository::crear(&conn, &reserva_test()).unwrap();

        ReservaRepository::crear(&conn, &reserva_test()).unwrap();

        let reservas = ReservaRepository::listar_todas(&conn).unwrap();

        assert_eq!(reservas.len(), 2);
    }
    pub fn mostrar_mis_reservas(request: &Request, conn: &Connection) -> Response {
        let usuario_id = match Self::obtener_usuario_sesion(request, conn) {
            Ok(id) => id,

            Err(response) => {
                return response;
            }
        };

        let reservas = match ReservaRepository::listar_por_usuario(conn, usuario_id) {
            Ok(r) => r,

            Err(e) => {
                return Response::text(format!("Error cargando reservas: {}", e))
                    .with_status_code(500);
            }
        };

        let mut filas = String::new();

        for reserva in reservas {
            let boton = if reserva.estado != "cancelada" {
                format!(
                    r#"
                    <form
                        method="POST"
                        action="/mis-reservas/cancelar/{}">

                        <button class="btn-danger">
                            Cancelar
                        </button>

                    </form>
                    "#,
                    reserva.id
                )
            } else {
                "<span style='color:red'>
                    Cancelada
                </span>"
                    .to_string()
            };

            filas.push_str(&format!(
                r#"
                <tr>

                    <td>{}</td>

                    <td>
                        {} al {}
                    </td>

                    <td>{}</td>

                    <td>{}</td>

                </tr>
                "#,
                reserva.id, reserva.fecha_inicio, reserva.fecha_fin, reserva.estado, boton
            ));
        }

        let html = include_str!("../../templates/mis_reservas.html");

        let html = html.replace("{{reservas}}", &filas);

        Response::html(html)
    }
    pub fn cancelar_reserva(request: &Request, conn: &Connection, reserva_id: i64) -> Response {
        let usuario_id = match Self::obtener_usuario_sesion(request, conn) {
            Ok(id) => id,

            Err(response) => {
                return response;
            }
        };

        match ReservaService::cancelar_reserva(conn, reserva_id, usuario_id) {
            Ok(_) => Response::redirect_303("/mis-reservas"),

            Err(e) => templates::response_mensaje_error("Error cancelando reserva", &e),
        }
    }
}
