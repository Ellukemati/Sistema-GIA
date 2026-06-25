use crate::models::reserva::Reserva;
use crate::models::reserva_view::ReservaView;
use crate::repository::reserva_instrumento_repository::ReservaInstrumentoRepository;
use crate::service::reserva_service::ReservaService;
use crate::templates;
use chrono::NaiveDate;
use rouille::{Request, Response};
use rusqlite::{Connection, Result as SqlResult, params};

pub struct EquipoRaw {
    pub modelo_id: i64,
    pub nombre_modelo: String,
    pub marca: String,
    pub categoria: Option<String>,
    pub ejemplar_id: i64,
    pub codigo_qr: Option<String>,
    pub numero_serie: Option<String>,
    pub patrimonio: Option<String>,
    pub observaciones: Option<String>,
    pub accesorios: Option<String>,
}
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

    pub fn obtener_equipos_por_reserva(
        conn: &Connection,
        reserva_id: i64,
    ) -> Result<Vec<EquipoRaw>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT m.id, m.nombre_modelo, m.marca, m.categoria, e.id, e.codigo_qr, e.numero_serie, e.patrimonio, e.observaciones, e.accesorios 
             FROM reserva_ejemplar re
             JOIN ejemplares e ON re.ejemplar_id = e.id
             JOIN modelos m ON e.modelo_id = m.id
             WHERE re.reserva_id = ?1"
        )?;

        let equipos_iter = stmt.query_map([reserva_id], |row| {
            Ok(EquipoRaw {
                modelo_id: row.get(0)?,
                nombre_modelo: row.get(1)?,
                marca: row.get(2)?,
                categoria: row.get(3)?,
                ejemplar_id: row.get(4)?,
                codigo_qr: row.get(5)?,
                numero_serie: row.get(6)?,
                patrimonio: row.get(7)?,
                observaciones: row.get(8)?,
                accesorios: row.get(9)?,
            })
        })?;

        let mut equipos = Vec::new();
        for equipo in equipos_iter {
            equipos.push(equipo?);
        }
        Ok(equipos)
    }

    pub fn obtener_imagenes_ejemplar(
        conn: &Connection,
        ejemplar_id: i64,
    ) -> Result<Vec<Vec<u8>>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT imagen_blob FROM ejemplar_imagen WHERE ejemplar_id = ?1 ORDER BY orden ASC",
        )?;

        let filas = stmt.query_map([ejemplar_id], |row| row.get::<_, Vec<u8>>(0))?;
        let mut imagenes = Vec::new();
        for img in filas {
            imagenes.push(img?);
        }
        Ok(imagenes)
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

    /// Guarda la auditoría completa de la confirmación
    pub fn confirmar_aprobacion(
        conn: &Connection,
        reserva_id: i64,
        nuevo_estado: &str,
        admin_id: i64,
        momento_confirmacion: &str,
    ) -> SqlResult<usize> {
        conn.execute(
            "UPDATE reservas 
             SET estado = ?1, id_admin_aprobador = ?2, momento_confirmacion = ?3 
             WHERE id = ?4",
            params![nuevo_estado, admin_id, momento_confirmacion, reserva_id],
        )
    }

    /// Modifica el estado de forma atómica
    pub fn cambiar_estado(
        conn: &Connection,
        reserva_id: i64,
        nuevo_estado: &str,
    ) -> SqlResult<usize> {
        conn.execute(
            "UPDATE reservas SET estado = ?1 WHERE id = ?2",
            params![nuevo_estado, reserva_id],
        )
    }

    /// Único método que junta toda la información cruzada usando JOINs
    pub fn obtener_datos_notificacion(
        conn: &Connection,
        reserva_id: i64,
    ) -> SqlResult<(String, String, String, String, String, String)> {
        conn.query_row(
            "SELECT 
                u_docente.email AS docente_email, 
                u_docente.nombre || ' ' || u_docente.apellido AS docente_nombre, 
                COALESCE(r.motivo, 'Uso de instrumental') AS motivo, 
                r.fecha_inicio,
                COALESCE(r.momento_confirmacion, 'Sin fecha') AS momento_confirmacion,
                COALESCE(u_admin.nombre || ' ' || u_admin.apellido, 'Administración GIA') AS admin_nombre
             FROM reservas r 
             JOIN usuarios u_docente ON r.id_usuario = u_docente.id 
             LEFT JOIN usuarios u_admin ON r.id_admin_aprobador = u_admin.id
             WHERE r.id = ?",
            [reserva_id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        )
    }

    pub fn mostrar_mis_reservas(request: &Request, conn: &Connection) -> Response {
        let usuario_id = match Self::obtener_usuario_sesion(request, conn) {
            Ok(id) => id,
            Err(response) => return response,
        };

        let reservas = match Self::listar_por_usuario(conn, usuario_id) {
            Ok(r) => r,
            Err(e) => return Response::text(format!("Error: {}", e)).with_status_code(500),
        };

        let mut reservas_vista: Vec<ReservaView> = Vec::new();

        for reserva in reservas {
            let clase_estado = match reserva.estado.as_str() {
                "activa" => "estado-aprobada",
                "concluida" => "estado-concluida",
                "pendiente" => "estado-pendiente",
                "cancelada" => "estado-cancelada",
                _ => "",
            };

            let texto_estado = match reserva.estado.as_str() {
                "activa" => "Aceptada",
                "concluida" => "Finalizada",
                "pendiente" => "Pendiente",
                "cancelada" => "Cancelada",
                _ => &reserva.estado,
            };

            let equipos =
                ReservaInstrumentoRepository::obtener_nombres_equipos_reserva(conn, reserva.id)
                    .unwrap_or(vec![]);
            let inicio = NaiveDate::parse_from_str(&reserva.fecha_inicio, "%Y-%m-%d").unwrap();
            let fin = NaiveDate::parse_from_str(&reserva.fecha_fin, "%Y-%m-%d").unwrap();
            let dias = (fin - inicio).num_days();

            // Formatear creación
            let creada_txt = "Hoy".to_string();

            reservas_vista.push(ReservaView {
                id: reserva.id,
                fecha_inicio: reserva.fecha_inicio,
                fecha_fin: reserva.fecha_fin,
                estado: reserva.estado.clone(),
                texto_estado: texto_estado.to_string(),
                clase_estado: clase_estado.to_string(),
                motivo: reserva.motivo.unwrap_or("Sin motivo".to_string()),
                equipos,
                dias,
                creada: creada_txt,
            });
        }

        let mut ctx = tera::Context::new();
        ctx.insert("reservas", &reservas_vista);

        match templates::render("mis_reservas.html", &ctx) {
            Ok(html) => templates::response_html(Ok(html)),
            Err(e) => Response::text(format!("Error Tera: {:?}", e)).with_status_code(500),
        }
    }

    pub fn cancelar_reserva(request: &Request, conn: &Connection, reserva_id: i64) -> Response {
        let usuario_id = match Self::obtener_usuario_sesion(request, conn) {
            Ok(id) => id,
            Err(response) => return response,
        };

        match ReservaService::cancelar_reserva(conn, reserva_id, usuario_id) {
            Ok(_) => Response::empty_204().with_additional_header("HX-Redirect", "/mis-reservas"),
            Err(e) => templates::response_mensaje_error("Error cancelando reserva", &e),
        }
    }

    pub fn obtener_usuario_sesion(request: &Request, conn: &Connection) -> Result<i64, Response> {
        let token = match crate::utils::extraer_token_sesion(request) {
            Some(t) => t,
            None => return Err(Response::redirect_303("/login")),
        };

        match crate::repository::sesion_repository::SesionRepository::buscar_por_token(conn, &token)
        {
            Ok(Some(sesion)) => Ok(sesion.id_usuario),
            Ok(None) => Err(Response::redirect_303("/login")),
            Err(e) => Err(Response::text(format!("Error: {}", e)).with_status_code(500)),
        }
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
                estado TEXT NOT NULL CHECK (estado IN ('pendiente', 'activa', 'concluida', 'cancelada')),
                motivo TEXT,
                momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
                id_admin_aprobador INTEGER,
                momento_confirmacion TEXT
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
        let reserva = reserva_test(); // id_usuario es 1 por defecto en reserva_test()

        // Creamos la reserva en la base de datos (obtiene ID 1)
        ReservaRepository::crear(&conn, &reserva).unwrap();

        // Llamamos a la función nueva que implementamos para la lógica de negocio
        // Le pasamos: conn, reserva_id (1), usuario_id (1)
        ReservaRepository::cancelar_por_usuario(&conn, 1, 1).unwrap();

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

    #[test]
    fn test_obtener_imagenes_ejemplar_respeta_orden() {
        use crate::repository::reserva_repository::ReservaRepository;
        use rusqlite::Connection;

        let conn = Connection::open_in_memory().unwrap();

        conn.execute(
            "CREATE TABLE ejemplar_imagen (
            ejemplar_id INTEGER NOT NULL,
            orden INTEGER NOT NULL,
            imagen_blob BLOB NOT NULL,
            imagen_mime TEXT NOT NULL,
            PRIMARY KEY (ejemplar_id, orden)
        )",
            [],
        )
        .unwrap();

        // Inserta desordenado a propósito en la BDD
        conn.execute(
            "INSERT INTO ejemplar_imagen VALUES (1, 1, ?, 'image/png')",
            [vec![22, 22]],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ejemplar_imagen VALUES (1, 0, ?, 'image/png')",
            [vec![11, 11]],
        )
        .unwrap();

        let imgs = ReservaRepository::obtener_imagenes_ejemplar(&conn, 1).unwrap();

        assert_eq!(imgs.len(), 2);
        assert_eq!(imgs[0], vec![11, 11]); // Orden 0 primero
        assert_eq!(imgs[1], vec![22, 22]); // Orden 1 segundo
    }
}
