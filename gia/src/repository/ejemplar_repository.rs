use crate::models::ejemplar::Ejemplar;
use rusqlite::{Connection, Result as SqlResult};

pub struct EjemplarRepository;

impl EjemplarRepository {
    pub fn crear(conn: &Connection, ejemplar: &Ejemplar) -> SqlResult<usize> {
        conn.execute(
            "INSERT INTO ejemplares
            (modelo_id, numero_serie, codigo_qr, patrimonio, observaciones, esta_disponible, ubicacion)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                ejemplar.modelo_id,
                ejemplar.numero_serie,
                ejemplar.codigo_qr,
                ejemplar.patrimonio,
                ejemplar.observaciones,
                ejemplar.esta_disponible,
                ejemplar.ubicacion,
            ],
        )
    }
}
