use rusqlite::{Connection, Result as SqlResult};
use crate::models::modelo_instrumento::ModeloInstrumento;

pub struct ModeloInstrumentoRepository;

impl ModeloInstrumentoRepository {
    pub fn crear(
        conn: &Connection,
        modelo: &ModeloInstrumento
    ) -> SqlResult<usize> {
        conn.execute(
           "INSERT INTO modelos_instrumentos
            (marca, nombre_modelo, categoria, descripcion, manual_url, imagen_principal_url)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                modelo.marca,
                modelo.nombre_modelo,
                modelo.categoria,
                modelo.descripcion,
                modelo.manual_url,
                modelo.imagen_principal_url,
            ],
        )
    }
}