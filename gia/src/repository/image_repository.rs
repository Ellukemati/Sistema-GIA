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

    pub fn existe_imagen_principal_ejemplar(conn: &Connection, ejemplar_id: i64) -> SqlResult<bool> {
        conn.query_row(
            "SELECT 1 FROM ejemplar_imagen WHERE ejemplar_id = ?1 AND orden = 0",
            params![ejemplar_id],
            |_| Ok(()),
        )
        .optional()
        .map(|o| o.is_some())
    }
}
