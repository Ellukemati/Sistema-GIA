use rusqlite::{Connection, OptionalExtension, Result as SqlResult, params};

pub struct ImageRepository;

impl ImageRepository {
    pub fn eliminar_por_modelo(conn: &Connection, modelo_id: i64) -> SqlResult<()> {
        conn.execute(
            "DELETE FROM modelo_imagen WHERE modelo_id = ?1",
            params![modelo_id],
        )?;
        Ok(())
    }

    pub fn guardar_modelo(
        conn: &Connection,
        modelo_id: i64,
        orden: i32,
        blob: &[u8],
        mime: &str,
    ) -> SqlResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO modelo_imagen (modelo_id, orden, imagen_blob, imagen_mime) 
             VALUES (?1, ?2, ?3, ?4)",
            params![modelo_id, orden, blob, mime],
        )?;
        Ok(())
    }

    pub fn buscar_modelo(
        conn: &Connection,
        modelo_id: i64,
        orden: i32,
    ) -> SqlResult<Option<(Vec<u8>, String)>> {
        conn.query_row(
            "SELECT imagen_blob, imagen_mime FROM modelo_imagen WHERE modelo_id = ?1 AND orden = ?2",
            params![modelo_id, orden],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
    }

    /// Devuelve los `orden` de las imagenes de un modelo, ascendente, sin traer
    /// los blobs. Util para armar una galeria sin cargar los bytes en memoria.
    pub fn listar_ordenes_modelo(conn: &Connection, modelo_id: i64) -> SqlResult<Vec<i32>> {
        let mut stmt = conn.prepare(
            "SELECT orden FROM modelo_imagen WHERE modelo_id = ?1 ORDER BY orden ASC",
        )?;
        let ordenes = stmt
            .query_map(params![modelo_id], |row| row.get::<_, i32>(0))?
            .collect::<SqlResult<Vec<i32>>>()?;
        Ok(ordenes)
    }

    pub fn eliminar_por_ejemplar(conn: &Connection, ejemplar_id: i64) -> SqlResult<()> {
        conn.execute(
            "DELETE FROM ejemplar_imagen WHERE ejemplar_id = ?1",
            params![ejemplar_id],
        )?;
        Ok(())
    }

    pub fn guardar_ejemplar(
        conn: &Connection,
        ejemplar_id: i64,
        orden: i32,
        blob: &[u8],
        mime: &str,
    ) -> SqlResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO ejemplar_imagen (ejemplar_id, orden, imagen_blob, imagen_mime) 
             VALUES (?1, ?2, ?3, ?4)",
            params![ejemplar_id, orden, blob, mime],
        )?;
        Ok(())
    }

    pub fn buscar_ejemplar(
        conn: &Connection,
        ejemplar_id: i64,
        orden: i32,
    ) -> SqlResult<Option<(Vec<u8>, String)>> {
        conn.query_row(
            "SELECT imagen_blob, imagen_mime FROM ejemplar_imagen WHERE ejemplar_id = ?1 AND orden = ?2",
            params![ejemplar_id, orden],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
    }

    pub fn guardar_avatar(
        conn: &Connection,
        legajo: i64,
        blob: &[u8],
        mime: &str,
    ) -> SqlResult<()> {
        conn.execute(
            "UPDATE usuarios SET avatar_blob = ?1, avatar_mime = ?2 WHERE legajo = ?3",
            params![blob, mime, legajo],
        )?;
        Ok(())
    }

    pub fn buscar_avatar(conn: &Connection, legajo: i64) -> SqlResult<Option<(Vec<u8>, String)>> {
        conn.query_row(
            "SELECT avatar_blob, avatar_mime FROM usuarios WHERE legajo = ?1",
            params![legajo],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
    }

    pub fn existe_imagen_principal_modelo(conn: &Connection, modelo_id: i64) -> SqlResult<bool> {
        conn.query_row(
            "SELECT 1 FROM modelo_imagen WHERE modelo_id = ?1 AND orden = 0",
            params![modelo_id],
            |_| Ok(()),
        )
        .optional()
        .map(|o| o.is_some())
    }

    pub fn existe_imagen_principal_ejemplar(
        conn: &Connection,
        ejemplar_id: i64,
    ) -> SqlResult<bool> {
        conn.query_row(
            "SELECT 1 FROM ejemplar_imagen WHERE ejemplar_id = ?1 AND orden = 0",
            params![ejemplar_id],
            |_| Ok(()),
        )
        .optional()
        .map(|o| o.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE modelo_imagen (
                modelo_id INTEGER NOT NULL,
                orden INTEGER NOT NULL,
                imagen_blob BLOB NOT NULL,
                imagen_mime TEXT NOT NULL,
                PRIMARY KEY (modelo_id, orden)
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn listar_ordenes_modelo_sin_imagenes_retorna_vacio() {
        let conn = crear_db_test();
        let ordenes = ImageRepository::listar_ordenes_modelo(&conn, 1).unwrap();
        assert!(ordenes.is_empty());
    }

    #[test]
    fn listar_ordenes_modelo_retorna_ordenes_ascendentes() {
        let conn = crear_db_test();
        // Insertados desordenados a proposito.
        ImageRepository::guardar_modelo(&conn, 1, 2, b"c", "image/jpeg").unwrap();
        ImageRepository::guardar_modelo(&conn, 1, 0, b"a", "image/jpeg").unwrap();
        ImageRepository::guardar_modelo(&conn, 1, 1, b"b", "image/jpeg").unwrap();

        let ordenes = ImageRepository::listar_ordenes_modelo(&conn, 1).unwrap();

        assert_eq!(ordenes, vec![0, 1, 2]);
    }

    #[test]
    fn listar_ordenes_modelo_no_mezcla_otros_modelos() {
        let conn = crear_db_test();
        ImageRepository::guardar_modelo(&conn, 1, 0, b"a", "image/jpeg").unwrap();
        ImageRepository::guardar_modelo(&conn, 2, 0, b"b", "image/jpeg").unwrap();
        ImageRepository::guardar_modelo(&conn, 2, 1, b"c", "image/jpeg").unwrap();

        let ordenes = ImageRepository::listar_ordenes_modelo(&conn, 2).unwrap();

        assert_eq!(ordenes, vec![0, 1]);
    }
}
