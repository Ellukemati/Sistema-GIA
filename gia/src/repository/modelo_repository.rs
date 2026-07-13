use crate::models::modelo::Modelo;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult, params};

pub struct ModeloRepository;

impl ModeloRepository {
    pub fn crear(conn: &Connection, modelo: &Modelo) -> SqlResult<i64> {
        conn.execute(
            "INSERT INTO modelos (marca, nombre_modelo, categoria, descripcion)
            VALUES (?1, ?2, ?3, ?4)",
            params![
                modelo.marca,
                modelo.nombre_modelo,
                modelo.categoria,
                modelo.descripcion,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn listar_todos(conn: &Connection) -> SqlResult<Vec<Modelo>> {
        // Se listan los modelos que no han sido eliminados.
        let mut stmt = conn.prepare(
            "SELECT id, marca, nombre_modelo, categoria, descripcion, eliminado
             FROM modelos 
             WHERE eliminado = 0
             ORDER BY nombre_modelo",
        )?;
        let filas = stmt.query_map([], Modelo::from_row)?;

        let mut modelos = Vec::new();
        for modelo in filas {
            modelos.push(modelo?);
        }
        Ok(modelos)
    }

    pub fn actualizar(
        conn: &Connection,
        id: i64,
        marca: &str,
        nombre_modelo: &str,
        categoria: Option<&str>,
        descripcion: Option<&str>,
    ) -> SqlResult<()> {
        conn.execute(
            "UPDATE modelos
             SET marca = ?1, nombre_modelo = ?2, categoria = ?3, descripcion = ?4
             WHERE id = ?5",
            params![marca, nombre_modelo, categoria, descripcion, id],
        )?;
        Ok(())
    }

    pub fn buscar_por_id(conn: &Connection, id: i64) -> SqlResult<Option<Modelo>> {
        // No filtra por `eliminado`: permite validar si ya fue eliminado.
        let mut stmt = conn.prepare(
            "SELECT id, marca, nombre_modelo, categoria, descripcion, eliminado
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

    pub fn actualizar_manual(
        conn: &Connection,
        id: i64,
        blob: &[u8],
        mime: &str,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "UPDATE modelos SET manual_blob = ?1, manual_mime = ?2 WHERE id = ?3",
            rusqlite::params![blob, mime, id],
        )?;
        Ok(())
    }

    pub fn tiene_manual(conn: &Connection, id: i64) -> SqlResult<bool> {
        conn.query_row(
            "SELECT 1 FROM modelos WHERE id = ?1 AND manual_blob IS NOT NULL",
            params![id],
            |_| Ok(()),
        )
        .optional()
        .map(|o| o.is_some())
    }

    pub fn buscar_manual(
        conn: &Connection,
        id: i64,
    ) -> rusqlite::Result<Option<(Vec<u8>, String)>> {
        conn.query_row(
            "SELECT manual_blob, manual_mime FROM modelos WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                let blob: Option<Vec<u8>> = row.get(0)?;
                let mime: Option<String> = row.get(1)?;

                // Si alguna de las dos columnas es NULL en la BDD, lo tratamos como si no hubiera manual
                match (blob, mime) {
                    (Some(b), Some(m)) => Ok((b, m)),
                    _ => Err(rusqlite::Error::QueryReturnedNoRows),
                }
            },
        )
        .optional()
    }
    
    pub fn buscar_por_nombre(conn: &Connection, texto: &str) -> SqlResult<Vec<Modelo>> {
        let patron = format!("%{}%", texto);

        let mut stmt = conn.prepare(
            "SELECT id, marca, nombre_modelo, categoria, descripcion
            FROM modelos
            WHERE LOWER(nombre_modelo) LIKE LOWER(?1)
            ORDER BY nombre_modelo",
        )?;

        let filas = stmt.query_map(params![patron], Modelo::from_row)?;

        let mut modelos = Vec::new();

        for modelo in filas {
            modelos.push(modelo?);
        }

        Ok(modelos)
    }

    pub fn marcar_eliminado(conn: &Connection, id: i64) -> SqlResult<usize> {
        conn.execute(
            "UPDATE modelos SET eliminado = 1 WHERE id = ?1",
            params![id],
        )
    }
}
