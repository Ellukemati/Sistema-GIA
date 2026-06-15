use crate::models::ejemplar::Ejemplar;
use crate::repository::ejemplar_repository::EjemplarRepository;
use rusqlite::Connection;

pub struct EjemplarService;

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
}
