use crate::models::modelo::Modelo;
use rusqlite::{Connection, Result as SqlResult};

pub struct ModeloRepository;

impl ModeloRepository {
    pub fn crear(conn: &Connection, modelo: &Modelo) -> SqlResult<usize> {
        conn.execute(
           "INSERT INTO modelos (marca, nombre_modelo, categoria, descripcion, manual_url, direccion_imagen_principal)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                modelo.marca,
                modelo.modelo,
                modelo.categoria,
                modelo.descripcion,
                modelo.manual_url,
                modelo.direccion_imagen_principal,
            ],
        )
    }

    pub fn listar_todos(conn: &Connection) -> SqlResult<Vec<Modelo>> {
        let mut stmt = conn.prepare("SELECT * FROM modelos ORDER BY nombre_modelo")?;
        let filas = stmt.query_map([], Modelo::from_row)?;

        let mut modelos = Vec::new();
        for modelo in filas {
            modelos.push(modelo?);
        }
        Ok(modelos)
    }
    pub fn buscar_por_id(conn: &Connection, id: i64) -> SqlResult<Option<Modelo>> {
        let mut stmt = conn.prepare(
            "SELECT *
                FROM modelos
                WHERE id = ?1",
        )?;

        let resultado = stmt.query_row([id], Modelo::from_row);

        match resultado {
            Ok(modelo) => Ok(Some(modelo)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(e) => Err(e),
        }
    }
}
