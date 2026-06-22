use crate::models::ejemplar::Ejemplar;
use crate::repository::{ejemplar_repository::EjemplarRepository, reserva_repository::ReservaRepository};
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
    pub en_carrito: bool,
    pub codigo_qr: String,
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
            let disponible =
                ReservaRepository::ejemplar_disponible(conn, ejemplar.id, inicio, fin)
                    .map_err(|e| e.to_string())?;

            dtos.push(EjemplarDTO {
                id: ejemplar.id,
                numero_serie: ejemplar.numero_serie.unwrap_or_else(|| "Sin serie".to_string()),
                patrimonio: ejemplar
                    .patrimonio
                    .unwrap_or_else(|| "Sin patrimonio".to_string()),
                ubicacion: ejemplar
                    .ubicacion
                    .unwrap_or_else(|| "Sin ubicación".to_string()),
                en_carrito: ids_carrito.contains(&ejemplar.id),
                disponible,
                codigo_qr: ejemplar.codigo_qr.unwrap_or_else(|| "Sin QR".to_string()),
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
        let ejemplares =
            EjemplarRepository::listar_por_modelo(conn, modelo_id).map_err(|e| e.to_string())?;

        let dtos = ejemplares
            .into_iter()
            .map(|ejemplar| EjemplarDTO {
                id: ejemplar.id,
                numero_serie: ejemplar.numero_serie.unwrap_or_else(|| "Sin serie".to_string()),
                patrimonio: ejemplar
                    .patrimonio
                    .unwrap_or_else(|| "Sin patrimonio".to_string()),
                ubicacion: ejemplar
                    .ubicacion
                    .unwrap_or_else(|| "Sin ubicación".to_string()),
                disponible: true,
                en_carrito: false,
                codigo_qr: ejemplar.codigo_qr.unwrap_or_else(|| "Sin QR".to_string()),
            })
            .collect();

        Ok(dtos)
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
                manual_mime TEXT
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
                ubicacion TEXT
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
}
