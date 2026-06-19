use crate::models::ejemplar::Ejemplar;
use rusqlite::{Connection, Result as SqlResult};

pub struct EjemplarRepository;

impl EjemplarRepository {
    pub fn crear(conn: &Connection, ejemplar: &Ejemplar) -> SqlResult<i64> {
        conn.execute(
            "INSERT INTO ejemplares
            (modelo_id, numero_serie, codigo_qr, patrimonio, observaciones, accesorios, esta_disponible, ubicacion)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                ejemplar.modelo_id,
                ejemplar.numero_serie,
                ejemplar.codigo_qr,
                ejemplar.patrimonio,
                ejemplar.observaciones,
                ejemplar.accesorios,
                ejemplar.esta_disponible,
                ejemplar.ubicacion,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn listar_todos(conn: &Connection) -> SqlResult<Vec<Ejemplar>> {
        let mut stmt = conn.prepare(
            "SELECT id, modelo_id, numero_serie, codigo_qr, patrimonio, observaciones, accesorios, esta_disponible, ubicacion
             FROM ejemplares",
        )?;

        let filas = stmt.query_map([], Ejemplar::from_row)?;
        let mut ejemplares = Vec::new();

        for ejemplar in filas {
            ejemplares.push(ejemplar?);
        }

        Ok(ejemplares)
    }

    pub fn listar_por_modelo(conn: &Connection, modelo_id: i64) -> SqlResult<Vec<Ejemplar>> {
        let mut stmt = conn.prepare(
            "SELECT id, modelo_id, numero_serie, codigo_qr, patrimonio, observaciones, accesorios, esta_disponible, ubicacion
             FROM ejemplares
             WHERE modelo_id = ?1",
        )?;

        let filas = stmt.query_map([modelo_id], Ejemplar::from_row)?;
        let mut ejemplares = Vec::new();

        for ejemplar in filas {
            ejemplares.push(ejemplar?);
        }

        Ok(ejemplares)
    }
}
