use rusqlite::{Connection, OptionalExtension, Result as SqlResult, params};

use crate::models::reserva::Reserva;

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

    pub fn buscar_por_id(conn: &Connection, id: i64) -> Result<Option<Reserva>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, id_usuario, fecha_inicio, fecha_fin, estado, motivo, 
                    momento_creacion, momento_confirmacion, id_admin_aprobador 
             FROM reservas 
             WHERE id = ?",
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Reserva::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn obtener_id_admin_aprobador(
        conn: &Connection,
        id_reserva: i64,
    ) -> Result<Option<i64>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT id_admin_aprobador FROM reservas WHERE id = ?")?;

        match stmt.query_row([id_reserva], |row| row.get::<_, Option<i64>>(0)) {
            Ok(id_admin) => Ok(id_admin),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn listar_por_usuario(
        conn: &Connection,
        id_usuario: i64,
    ) -> Result<Vec<Reserva>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, id_usuario, fecha_inicio, fecha_fin, estado, motivo,
                momento_creacion, momento_confirmacion, id_admin_aprobador
         FROM reservas
         WHERE id_usuario = ?
         ORDER BY id DESC",
        )?;

        let mapped_rows = stmt.query_map([id_usuario], Reserva::from_row)?;

        let mut lista = Vec::new();
        for r in mapped_rows {
            lista.push(r?);
        }

        Ok(lista)
    }

    pub fn listar_todas_detalladas(conn: &Connection) -> SqlResult<Vec<Reserva>> {
        let mut stmt = conn.prepare(
            "SELECT
                id,
                id_usuario,
                fecha_inicio,
                fecha_fin,
                estado,
                motivo,
                momento_creacion
            FROM reservas
            ORDER BY momento_creacion DESC",
        )?;

        let filas = stmt.query_map([], Reserva::from_row)?;

        let mut reservas = Vec::new();

        for fila in filas {
            reservas.push(fila?);
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

    pub fn listar_todas(conn: &Connection) -> Result<Vec<Reserva>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, id_usuario, fecha_inicio, fecha_fin, estado, motivo, 
                    momento_creacion, momento_confirmacion, id_admin_aprobador 
             FROM reservas",
        )?;

        let mapped_rows = stmt.query_map([], Reserva::from_row)?;

        let mut lista = Vec::new();
        for r in mapped_rows {
            lista.push(r?);
        }
        Ok(lista)
    }

    pub fn tiene_reserva_activa_o_pendiente(
        conn: &Connection,
        ejemplar_id: i64,
    ) -> SqlResult<bool> {
        let cantidad: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM reservas r
             INNER JOIN reserva_ejemplar re ON r.id = re.reserva_id
             WHERE re.ejemplar_id = ?1
               AND r.estado IN ('pendiente', 'activa')",
            params![ejemplar_id],
            |row| row.get(0),
        )?;

        Ok(cantidad > 0)
    }

    pub fn ejemplar_disponible(
        conn: &Connection,
        ejemplar_id: i64,
        fecha_inicio: &str,
        fecha_fin: &str,
    ) -> SqlResult<bool> {
        let disponible: Option<bool> = conn
            .query_row(
                "SELECT e.esta_disponible != 0
                    AND NOT EXISTS (
                        SELECT 1
                        FROM reservas r
                        INNER JOIN reserva_ejemplar re ON r.id = re.reserva_id
                        WHERE re.ejemplar_id = e.id
                          AND r.estado != 'cancelada'
                          AND (r.fecha_inicio <= ?2 AND r.fecha_fin >= ?3)
                    )
                 FROM ejemplares e
                 WHERE e.id = ?1",
                params![ejemplar_id, fecha_fin, fecha_inicio],
                |row| row.get(0),
            )
            .optional()?;

        Ok(disponible.unwrap_or(false))
    }

    pub fn listar_por_estado(
        conn: &Connection,
        estado: &str,
    ) -> Result<Vec<Reserva>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, id_usuario, fecha_inicio, fecha_fin, estado, motivo, 
                    momento_creacion, momento_confirmacion, id_admin_aprobador 
            FROM reservas 
            WHERE estado = ?",
        )?;

        let mapped_rows = stmt.query_map([estado], Reserva::from_row)?;

        let mut lista = Vec::new();
        for r in mapped_rows {
            lista.push(r?);
        }
        Ok(lista)
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
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::reserva::Reserva;
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE modelos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                marca TEXT NOT NULL,
                nombre_modelo TEXT NOT NULL,
                categoria TEXT,
                descripcion TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE ejemplares (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                modelo_id INTEGER NOT NULL,
                numero_serie TEXT,
                codigo_qr TEXT,
                patrimonio TEXT,
                observaciones TEXT,
                accesorios TEXT,
                esta_disponible BOOLEAN DEFAULT TRUE,
                ubicacion TEXT,
                eliminado BOOLEAN NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
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

    fn insertar_ejemplar(conn: &Connection, id: i64, esta_disponible: bool) {
        conn.execute(
            "INSERT INTO ejemplares (id, modelo_id, esta_disponible)
             VALUES (?1, 1, ?2)",
            params![id, esta_disponible],
        )
        .unwrap();
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
            momento_confirmacion: Some("2026-06-26T12:00:00".to_string()),
            id_admin_aprobador: Some(1),
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
        insertar_ejemplar(&conn, 1, true);

        let disponible =
            ReservaRepository::ejemplar_disponible(&conn, 1, "2026-07-01", "2026-07-05").unwrap();

        assert!(disponible);
    }

    #[test]
    fn ejemplar_no_disponible_si_esta_disponible_es_false() {
        let conn = crear_db_test();
        insertar_ejemplar(&conn, 1, false);

        let disponible =
            ReservaRepository::ejemplar_disponible(&conn, 1, "2026-07-01", "2026-07-05").unwrap();

        assert!(!disponible);
    }

    #[test]
    fn ejemplar_no_disponible_si_ya_esta_reservado() {
        let conn = crear_db_test();
        insertar_ejemplar(&conn, 1, true);

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
        insertar_ejemplar(&conn, 1, true);

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

    // fn utilizada para testing
    fn vincular_ejemplar(conn: &Connection, reserva_id: i64, ejemplar_id: i64) {
        conn.execute(
            "INSERT INTO reserva_ejemplar (reserva_id, ejemplar_id) VALUES (?1, ?2)",
            params![reserva_id, ejemplar_id],
        )
        .unwrap();
    }

    #[test]
    fn tiene_reserva_activa_o_pendiente_sin_reservas() {
        let conn = crear_db_test();

        let bloqueado = ReservaRepository::tiene_reserva_activa_o_pendiente(&conn, 1).unwrap();

        assert!(!bloqueado);
    }

    #[test]
    fn tiene_reserva_activa_o_pendiente_con_pendiente() {
        let conn = crear_db_test();
        ReservaRepository::crear(&conn, &reserva_test()).unwrap();
        vincular_ejemplar(&conn, 1, 1);

        let bloqueado = ReservaRepository::tiene_reserva_activa_o_pendiente(&conn, 1).unwrap();

        assert!(bloqueado);
    }

    #[test]
    fn tiene_reserva_activa_o_pendiente_con_activa() {
        let conn = crear_db_test();
        let mut reserva = reserva_test();
        reserva.estado = "activa".to_string();
        ReservaRepository::crear(&conn, &reserva).unwrap();
        vincular_ejemplar(&conn, 1, 1);

        let bloqueado = ReservaRepository::tiene_reserva_activa_o_pendiente(&conn, 1).unwrap();

        assert!(bloqueado);
    }

    #[test]
    fn tiene_reserva_activa_o_pendiente_ignora_concluida() {
        let conn = crear_db_test();
        let mut reserva = reserva_test();
        reserva.estado = "concluida".to_string();
        ReservaRepository::crear(&conn, &reserva).unwrap();
        vincular_ejemplar(&conn, 1, 1);

        let bloqueado = ReservaRepository::tiene_reserva_activa_o_pendiente(&conn, 1).unwrap();

        assert!(!bloqueado);
    }

    #[test]
    fn tiene_reserva_activa_o_pendiente_ignora_cancelada() {
        let conn = crear_db_test();
        let mut reserva = reserva_test();
        reserva.estado = "cancelada".to_string();
        ReservaRepository::crear(&conn, &reserva).unwrap();
        vincular_ejemplar(&conn, 1, 1);

        let bloqueado = ReservaRepository::tiene_reserva_activa_o_pendiente(&conn, 1).unwrap();

        assert!(!bloqueado);
    }

    #[test]
    fn obtener_equipos_por_reserva_incluye_ejemplar_eliminado() {
        let conn = crear_db_test();

        conn.execute(
            "INSERT INTO modelos (id, marca, nombre_modelo, categoria)
             VALUES (1, 'Marca', 'Modelo', 'Categoria')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ejemplares (id, modelo_id, numero_serie, esta_disponible, eliminado)
             VALUES (1, 1, 'SN-1', TRUE, 1)",
            [],
        )
        .unwrap();
        ReservaRepository::crear(&conn, &reserva_test()).unwrap();
        vincular_ejemplar(&conn, 1, 1);

        let equipos = ReservaRepository::obtener_equipos_por_reserva(&conn, 1).unwrap();

        assert_eq!(equipos.len(), 1);
        assert_eq!(equipos[0].ejemplar_id, 1);
        assert_eq!(equipos[0].numero_serie.as_deref(), Some("SN-1"));
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
