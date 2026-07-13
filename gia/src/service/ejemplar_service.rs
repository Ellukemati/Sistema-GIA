use crate::models::ejemplar::Ejemplar;
use crate::repository::{
    ejemplar_repository::EjemplarRepository, image_repository::ImageRepository,
    modelo_repository::ModeloRepository, reserva_repository::ReservaRepository,
};
use rusqlite::Connection;
use serde::Serialize;

pub struct EjemplarService;

/// Datos de un ejemplar listos para mostrar en la pantalla de seleccion,
/// con su disponibilidad para las fechas y si ya esta en el carrito.
#[derive(Serialize)]
pub struct EjemplarDTO {
    pub id: i64,
    pub numero_serie: String,
    pub patrimonio: String,
    pub ubicacion: String,
    pub disponible: bool,
    pub esta_disponible: bool, // hace referencia al estado en el inventario, no a la fecha de reserva
    pub en_carrito: bool,
    pub codigo_qr: String,
    pub observaciones: Option<String>,
    pub accesorios: Option<String>,
    pub tiene_reserva_bloqueante: bool,
    pub imagen: Option<String>,
}

pub struct CrearEjemplarData {
    pub modelo_id: i64,
    pub numero_serie: Option<String>,
    pub codigo_qr: Option<String>,
    pub patrimonio: Option<String>,
    pub observaciones: Option<String>,
    pub accesorios: Option<String>,
    pub esta_disponible: bool,
    pub ubicacion: Option<String>,
}

impl EjemplarService {
    pub fn crear_ejemplar(conn: &Connection, data: CrearEjemplarData) -> Result<Ejemplar, String> {
        let ejemplar_temporal = Ejemplar {
            id: 0, // Temporal para mandarle al repositorio
            modelo_id: data.modelo_id,
            numero_serie: data.numero_serie,
            codigo_qr: data.codigo_qr,
            patrimonio: data.patrimonio,
            observaciones: data.observaciones,
            accesorios: data.accesorios,
            esta_disponible: data.esta_disponible,
            ubicacion: data.ubicacion,
            eliminado: false,
        };

        match EjemplarRepository::crear(conn, &ejemplar_temporal) {
            Ok(id_real) => Ok(Ejemplar {
                id: id_real,
                ..ejemplar_temporal
            }),
            Err(e) => Err(format!(
                "Error en la base de datos al crear ejemplar: {}",
                e
            )),
        }
    }

    pub fn actualizar_ejemplar(
        conn: &Connection,
        id: i64,
        data: CrearEjemplarData,
    ) -> Result<Ejemplar, String> {
        let _existente = EjemplarRepository::buscar_por_id(conn, id)
            .map_err(|e| format!("Error al buscar ejemplar: {}", e))?
            .ok_or_else(|| "El ejemplar solicitado no existe.".to_string())?;

        if ReservaRepository::tiene_reserva_activa_o_pendiente(conn, id)
            .map_err(|e| format!("Error al verificar reservas: {}", e))?
        {
            return Err(
                "No se puede modificar el ejemplar porque tiene una reserva pendiente o activa."
                    .to_string(),
            );
        }

        ModeloRepository::buscar_por_id(conn, data.modelo_id)
            .map_err(|e| format!("Error al verificar modelo: {}", e))?
            .ok_or_else(|| "El modelo seleccionado no existe.".to_string())?;

        let ejemplar = Ejemplar {
            id,
            modelo_id: data.modelo_id,
            numero_serie: data.numero_serie,
            codigo_qr: data.codigo_qr,
            patrimonio: data.patrimonio,
            observaciones: data.observaciones,
            accesorios: data.accesorios,
            esta_disponible: data.esta_disponible,
            ubicacion: data.ubicacion,
            eliminado: false,
        };

        EjemplarRepository::actualizar(conn, &ejemplar).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("UNIQUE constraint failed: ejemplares.numero_serie") {
                "Ya existe otro ejemplar con ese número de serie.".to_string()
            } else if msg.contains("UNIQUE constraint failed: ejemplares.codigo_qr") {
                "Ya existe otro ejemplar con ese código QR.".to_string()
            } else if msg.contains("UNIQUE constraint failed: ejemplares.patrimonio") {
                "Ya existe otro ejemplar con ese patrimonio.".to_string()
            } else {
                format!("Error en la base de datos al actualizar ejemplar: {}", msg)
            }
        })?;

        Ok(ejemplar)
    }

    /// Elimina un ejemplar marcandolo como `eliminado`.
    /// Falla si el ejemplar no existe, si ya fue eliminado, o si tiene una
    /// reserva activa o pendiente.
    pub fn eliminar_ejemplar(conn: &Connection, id: i64) -> Result<(), String> {
        let ejemplar = EjemplarRepository::buscar_por_id(conn, id)
            .map_err(|e| format!("Error al buscar ejemplar: {}", e))?
            .ok_or_else(|| "El ejemplar no existe.".to_string())?;

        if ejemplar.eliminado {
            return Err("El ejemplar ya fue eliminado.".to_string());
        }

        if ReservaRepository::tiene_reserva_activa_o_pendiente(conn, id)
            .map_err(|e| format!("Error al verificar reservas: {}", e))?
        {
            return Err(
                "No se puede eliminar: el ejemplar tiene una reserva activa o pendiente."
                    .to_string(),
            );
        }

        EjemplarRepository::marcar_eliminado(conn, id)
            .map_err(|e| format!("Error en la base de datos al eliminar ejemplar: {}", e))?;

        Ok(())
    }

    /// Lista los ejemplares de un modelo con su disponibilidad para el rango de
    /// fechas y marcando los que ya estan en el carrito.
    pub fn listar_ejemplares_para_modelo(
        conn: &Connection,
        modelo_id: i64,
        inicio: &str,
        fin: &str,
        ids_carrito: &[i64],
    ) -> Result<Vec<EjemplarDTO>, String> {
        let ejemplares =
            EjemplarRepository::listar_por_modelo(conn, modelo_id).map_err(|e| e.to_string())?;

        let mut dtos = Vec::with_capacity(ejemplares.len());
        for ejemplar in ejemplares {
            let disponible = ReservaRepository::ejemplar_disponible(conn, ejemplar.id, inicio, fin)
                .map_err(|e| e.to_string())?;

            dtos.push(EjemplarDTO {
                id: ejemplar.id,
                numero_serie: ejemplar
                    .numero_serie
                    .unwrap_or_else(|| "Sin serie".to_string()),
                patrimonio: ejemplar
                    .patrimonio
                    .unwrap_or_else(|| "Sin patrimonio".to_string()),
                ubicacion: ejemplar
                    .ubicacion
                    .unwrap_or_else(|| "Sin ubicación".to_string()),
                en_carrito: ids_carrito.contains(&ejemplar.id),
                disponible,
                esta_disponible: ejemplar.esta_disponible,
                codigo_qr: ejemplar.codigo_qr.unwrap_or_else(|| "Sin QR".to_string()),
                observaciones: Self::texto_opcional(ejemplar.observaciones),
                accesorios: Self::texto_opcional(ejemplar.accesorios),
                tiene_reserva_bloqueante: false,
                imagen: Self::url_imagen_principal(conn, ejemplar.id),
            });
        }

        Ok(dtos)
    }

    /// Lista todos los ejemplares de un modelo sin evaluar disponibilidad (se usa
    /// cuando todavia no hay fechas elegidas).
    pub fn listar_ejemplares_basico(
        conn: &Connection,
        modelo_id: i64,
    ) -> Result<Vec<EjemplarDTO>, String> {
        Self::listar_ejemplares_basico_interno(conn, modelo_id, false)
    }

    /// Igual que listar_ejemplares_basico pero incluye si cada ejemplar tiene
    /// una reserva pendiente o activa que impide su edición.
    pub fn listar_ejemplares_para_detalle(
        conn: &Connection,
        modelo_id: i64,
    ) -> Result<Vec<EjemplarDTO>, String> {
        Self::listar_ejemplares_basico_interno(conn, modelo_id, true)
    }

    fn listar_ejemplares_basico_interno(
        conn: &Connection,
        modelo_id: i64,
        incluir_bloqueo_reserva: bool,
    ) -> Result<Vec<EjemplarDTO>, String> {
        let ejemplares =
            EjemplarRepository::listar_por_modelo(conn, modelo_id).map_err(|e| e.to_string())?;

        let mut dtos = Vec::with_capacity(ejemplares.len());
        for ejemplar in ejemplares {
            let tiene_reserva_bloqueante = if incluir_bloqueo_reserva {
                ReservaRepository::tiene_reserva_activa_o_pendiente(conn, ejemplar.id)
                    .map_err(|e| e.to_string())?
            } else {
                false
            };

            dtos.push(EjemplarDTO {
                id: ejemplar.id,
                numero_serie: ejemplar
                    .numero_serie
                    .unwrap_or_else(|| "Sin serie".to_string()),
                patrimonio: ejemplar
                    .patrimonio
                    .unwrap_or_else(|| "Sin patrimonio".to_string()),
                ubicacion: ejemplar
                    .ubicacion
                    .unwrap_or_else(|| "Sin ubicación".to_string()),
                disponible: true,
                esta_disponible: ejemplar.esta_disponible,
                en_carrito: false,
                codigo_qr: ejemplar.codigo_qr.unwrap_or_else(|| "Sin QR".to_string()),
                observaciones: Self::texto_opcional(ejemplar.observaciones),
                accesorios: Self::texto_opcional(ejemplar.accesorios),
                tiene_reserva_bloqueante,
                imagen: Self::url_imagen_principal(conn, ejemplar.id),
            });
        }

        Ok(dtos)
    }

    fn url_imagen_principal(conn: &Connection, ejemplar_id: i64) -> Option<String> {
        match ImageRepository::existe_imagen_principal_ejemplar(conn, ejemplar_id) {
            Ok(true) => Some(format!("/imagenes/ejemplares/{}/0", ejemplar_id)),
            _ => None,
        }
    }

    fn texto_opcional(valor: Option<String>) -> Option<String> {
        valor.filter(|s| !s.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::ejemplar::Ejemplar;
    use crate::models::modelo::Modelo;
    use crate::repository::ejemplar_repository::EjemplarRepository;
    use crate::repository::modelo_repository::ModeloRepository;
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE modelos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                marca TEXT NOT NULL,
                nombre_modelo TEXT NOT NULL,
                categoria TEXT,
                descripcion TEXT,
                manual_blob BLOB,
                manual_mime TEXT,
                eliminado BOOLEAN NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE ejemplares (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                modelo_id INTEGER NOT NULL,
                numero_serie TEXT UNIQUE,
                codigo_qr TEXT UNIQUE,
                patrimonio TEXT UNIQUE,
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
        conn.execute(
            "CREATE TABLE reserva_ejemplar (
                reserva_id INTEGER NOT NULL,
                ejemplar_id INTEGER NOT NULL,
                PRIMARY KEY(reserva_id, ejemplar_id)
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn insertar_modelo(conn: &Connection, nombre: &str) -> i64 {
        let modelo = Modelo {
            id: 0,
            marca: "Marca".into(),
            nombre_modelo: nombre.into(),
            categoria: None,
            descripcion: None,
            eliminado: false,
        };
        ModeloRepository::crear(conn, &modelo).unwrap()
    }

    fn insertar_ejemplar(
        conn: &Connection,
        modelo_id: i64,
        numero_serie: Option<&str>,
        patrimonio: Option<&str>,
        ubicacion: Option<&str>,
        codigo_qr: Option<&str>,
    ) -> i64 {
        let ejemplar = Ejemplar {
            id: 0,
            modelo_id,
            numero_serie: numero_serie.map(String::from),
            codigo_qr: codigo_qr.map(String::from),
            patrimonio: patrimonio.map(String::from),
            observaciones: None,
            accesorios: None,
            esta_disponible: true,
            ubicacion: ubicacion.map(String::from),
            eliminado: false,
        };
        EjemplarRepository::crear(conn, &ejemplar).unwrap()
    }

    #[test]
    fn listar_ejemplares_basico_sin_ejemplares_retorna_vacio() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");

        let dtos = EjemplarService::listar_ejemplares_basico(&conn, modelo_id).unwrap();

        assert!(dtos.is_empty());
    }

    #[test]
    fn listar_ejemplares_basico_modelo_inexistente_retorna_vacio() {
        let conn = crear_db_test();

        let dtos = EjemplarService::listar_ejemplares_basico(&conn, 999).unwrap();

        assert!(dtos.is_empty());
    }

    #[test]
    fn listar_ejemplares_basico_retorna_datos_completos() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let id = insertar_ejemplar(
            &conn,
            modelo_id,
            Some("SN-001"),
            Some("PAT-001"),
            Some("Depósito"),
            Some("QR-001"),
        );

        let dtos = EjemplarService::listar_ejemplares_basico(&conn, modelo_id).unwrap();

        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].id, id);
        assert_eq!(dtos[0].numero_serie, "SN-001");
        assert_eq!(dtos[0].patrimonio, "PAT-001");
        assert_eq!(dtos[0].ubicacion, "Depósito");
        assert_eq!(dtos[0].codigo_qr, "QR-001");
        assert!(dtos[0].disponible);
        assert!(dtos[0].esta_disponible);
        assert!(!dtos[0].en_carrito);
    }

    #[test]
    fn listar_ejemplares_basico_campos_opcionales_usan_defaults() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        insertar_ejemplar(&conn, modelo_id, None, None, None, None);

        let dtos = EjemplarService::listar_ejemplares_basico(&conn, modelo_id).unwrap();

        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].numero_serie, "Sin serie");
        assert_eq!(dtos[0].patrimonio, "Sin patrimonio");
        assert_eq!(dtos[0].ubicacion, "Sin ubicación");
        assert_eq!(dtos[0].codigo_qr, "Sin QR");
    }

    #[test]
    fn listar_ejemplares_basico_solo_del_modelo_solicitado() {
        let conn = crear_db_test();
        let modelo_a = insertar_modelo(&conn, "Violín");
        let modelo_b = insertar_modelo(&conn, "Viola");
        insertar_ejemplar(&conn, modelo_a, Some("A-1"), Some("PA-1"), None, None);
        insertar_ejemplar(&conn, modelo_b, Some("B-1"), Some("PB-1"), None, None);

        let dtos = EjemplarService::listar_ejemplares_basico(&conn, modelo_a).unwrap();

        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].numero_serie, "A-1");
    }

    #[test]
    fn listar_ejemplares_basico_varios_ejemplares() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);
        insertar_ejemplar(&conn, modelo_id, Some("SN-2"), None, None, None);

        let dtos = EjemplarService::listar_ejemplares_basico(&conn, modelo_id).unwrap();

        assert_eq!(dtos.len(), 2);
    }

    fn datos_ejemplar(modelo_id: i64) -> CrearEjemplarData {
        CrearEjemplarData {
            modelo_id,
            numero_serie: Some("SN-NUEVO".into()),
            codigo_qr: Some("QR-NUEVO".into()),
            patrimonio: Some("PAT-NUEVO".into()),
            observaciones: Some("Obs".into()),
            accesorios: None,
            esta_disponible: false,
            ubicacion: Some("Lab 2".into()),
        }
    }

    #[test]
    fn actualizar_ejemplar_sin_reservas() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(
            &conn,
            modelo_id,
            Some("SN-OLD"),
            Some("PAT-OLD"),
            Some("Depósito"),
            Some("QR-OLD"),
        );

        let actualizado =
            EjemplarService::actualizar_ejemplar(&conn, ejemplar_id, datos_ejemplar(modelo_id))
                .unwrap();

        assert_eq!(actualizado.numero_serie.as_deref(), Some("SN-NUEVO"));
        assert_eq!(actualizado.ubicacion.as_deref(), Some("Lab 2"));
        assert!(!actualizado.esta_disponible);
    }

    #[test]
    fn actualizar_ejemplar_falla_con_reserva_pendiente() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);

        conn.execute(
            "INSERT INTO reservas (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
             VALUES (1, '2026-07-01', '2026-07-05', 'pendiente', 'Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reserva_ejemplar (reserva_id, ejemplar_id) VALUES (1, ?1)",
            [ejemplar_id],
        )
        .unwrap();

        match EjemplarService::actualizar_ejemplar(&conn, ejemplar_id, datos_ejemplar(modelo_id)) {
            Err(msg) => assert!(msg.contains("reserva pendiente o activa")),
            Ok(_) => panic!("Se esperaba un error por reserva pendiente"),
        }
    }

    #[test]
    fn actualizar_ejemplar_falla_con_reserva_activa() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);

        conn.execute(
            "INSERT INTO reservas (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
             VALUES (1, '2026-07-01', '2026-07-05', 'activa', 'Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reserva_ejemplar (reserva_id, ejemplar_id) VALUES (1, ?1)",
            [ejemplar_id],
        )
        .unwrap();

        let resultado =
            EjemplarService::actualizar_ejemplar(&conn, ejemplar_id, datos_ejemplar(modelo_id));

        assert!(resultado.is_err());
    }

    #[test]
    fn actualizar_ejemplar_permite_si_reserva_concluida() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);

        conn.execute(
            "INSERT INTO reservas (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
             VALUES (1, '2026-07-01', '2026-07-05', 'concluida', 'Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reserva_ejemplar (reserva_id, ejemplar_id) VALUES (1, ?1)",
            [ejemplar_id],
        )
        .unwrap();

        let actualizado =
            EjemplarService::actualizar_ejemplar(&conn, ejemplar_id, datos_ejemplar(modelo_id))
                .unwrap();

        assert_eq!(actualizado.numero_serie.as_deref(), Some("SN-NUEVO"));
    }

    #[test]
    fn eliminar_ejemplar_sin_reservas_lo_marca_eliminado() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);

        EjemplarService::eliminar_ejemplar(&conn, ejemplar_id).unwrap();

        let ejemplar = EjemplarRepository::buscar_por_id(&conn, ejemplar_id)
            .unwrap()
            .unwrap();
        assert!(ejemplar.eliminado);
    }

    #[test]
    fn eliminar_ejemplar_ya_eliminado_falla() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);

        EjemplarService::eliminar_ejemplar(&conn, ejemplar_id).unwrap();

        match EjemplarService::eliminar_ejemplar(&conn, ejemplar_id) {
            Err(msg) => assert!(msg.contains("ya fue eliminado")),
            Ok(_) => panic!("Se esperaba un error por ejemplar ya eliminado"),
        }
    }

    #[test]
    fn eliminar_ejemplar_falla_con_reserva_pendiente() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);

        conn.execute(
            "INSERT INTO reservas (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
             VALUES (1, '2026-07-01', '2026-07-05', 'pendiente', 'Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reserva_ejemplar (reserva_id, ejemplar_id) VALUES (1, ?1)",
            [ejemplar_id],
        )
        .unwrap();

        match EjemplarService::eliminar_ejemplar(&conn, ejemplar_id) {
            Err(msg) => assert!(msg.contains("reserva activa o pendiente")),
            Ok(_) => panic!("Se esperaba un error por reserva pendiente"),
        }
    }

    #[test]
    fn eliminar_ejemplar_falla_con_reserva_activa() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);

        conn.execute(
            "INSERT INTO reservas (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
             VALUES (1, '2026-07-01', '2026-07-05', 'activa', 'Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reserva_ejemplar (reserva_id, ejemplar_id) VALUES (1, ?1)",
            [ejemplar_id],
        )
        .unwrap();

        assert!(EjemplarService::eliminar_ejemplar(&conn, ejemplar_id).is_err());
    }

    #[test]
    fn eliminar_ejemplar_permite_si_reserva_concluida() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(&conn, modelo_id, Some("SN-1"), None, None, None);

        conn.execute(
            "INSERT INTO reservas (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
             VALUES (1, '2026-07-01', '2026-07-05', 'concluida', 'Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reserva_ejemplar (reserva_id, ejemplar_id) VALUES (1, ?1)",
            [ejemplar_id],
        )
        .unwrap();

        EjemplarService::eliminar_ejemplar(&conn, ejemplar_id).unwrap();

        let ejemplar = EjemplarRepository::buscar_por_id(&conn, ejemplar_id)
            .unwrap()
            .unwrap();
        assert!(ejemplar.eliminado);
    }
}
