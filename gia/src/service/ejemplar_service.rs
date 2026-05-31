use crate::models::ejemplar::Ejemplar;
use crate::repository::ejemplar_repository::EjemplarRepository;
use rusqlite::Connection;

pub struct EjemplarService;

impl EjemplarService {
    pub fn crear_ejemplar(
        conn: &Connection,
        modelo_id: i64,
        numero_serie: Option<String>,
        codigo_qr: Option<String>,
        patrimonio: Option<String>,
        observaciones: Option<String>,
        esta_disponible: bool,
        ubicacion: Option<String>,
    ) -> Result<Ejemplar, String> {
        let ejemplar = Ejemplar {
            id: 0, // se asigna en la db
            modelo_id,
            numero_serie,
            codigo_qr,
            patrimonio,
            observaciones,
            esta_disponible,
            ubicacion,
        };

        match EjemplarRepository::crear(conn, &ejemplar) {
            Ok(_) => Ok(ejemplar),
            Err(e) => Err(format!("Error en la base de datos al crear ejemplar: {}", e)),
        }
    }
}
