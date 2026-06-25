use crate::models::usuario::Usuario;
use rusqlite::{Connection, Result as SqlResult, params};

pub struct UsuarioRepository;

impl UsuarioRepository {
    pub fn buscar_por_email(conn: &Connection, email: &str) -> SqlResult<Option<Usuario>> {
        let mut stmt = conn.prepare(
            "SELECT id, nombre, apellido, email, legajo, tipo, password_hash, aprobado, momento_creacion, avatar_blob, avatar_mime 
             FROM usuarios 
             WHERE email = ?1"
        )?;

        let mut rows = stmt.query([email])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Usuario::from_row(row)?))
        } else {
            Ok(None)
        }
    }
    pub fn listar_administradores(conn: &Connection) -> SqlResult<Vec<Usuario>> {
        let mut stmt = conn.prepare(
            "SELECT id, nombre, apellido, email, legajo, tipo,
                    password_hash, aprobado, momento_creacion,
                    avatar_blob, avatar_mime
            FROM usuarios
            WHERE tipo = 'A'
            ORDER BY apellido, nombre",
        )?;

        let filas = stmt.query_map([], Usuario::from_row)?;

        let mut usuarios = Vec::new();

        for usuario in filas {
            usuarios.push(usuario?);
        }

        Ok(usuarios)
    }
    pub fn listar_docentes_aprobados(conn: &Connection) -> SqlResult<Vec<Usuario>> {
        let mut stmt = conn.prepare(
            "SELECT id, nombre, apellido, email, legajo, tipo,
                    password_hash, aprobado, momento_creacion,
                    avatar_blob, avatar_mime
            FROM usuarios
            WHERE tipo = 'P'
            AND aprobado = 1
            ORDER BY apellido, nombre",
        )?;

        let filas = stmt.query_map([], Usuario::from_row)?;

        let mut usuarios = Vec::new();

        for usuario in filas {
            usuarios.push(usuario?);
        }

        Ok(usuarios)
    }
    pub fn hacer_admin(conn: &Connection, id: i64) -> SqlResult<usize> {
        conn.execute(
            "UPDATE usuarios
            SET tipo = 'A'
            WHERE id = ?1",
            [id],
        )
    }
    pub fn quitar_admin(conn: &Connection, id: i64) -> SqlResult<usize> {
        conn.execute(
            "UPDATE usuarios
            SET tipo = 'P'
            WHERE id = ?1",
            [id],
        )
    }
    pub fn buscar_por_id(conn: &Connection, id: i64) -> SqlResult<Option<Usuario>> {
        let mut stmt = conn.prepare(
            "SELECT id, nombre, apellido, email, legajo, tipo, password_hash, aprobado, momento_creacion, avatar_blob, avatar_mime 
             FROM usuarios 
             WHERE id = ?1"
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Usuario::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn buscar_por_legajo(conn: &Connection, legajo: i32) -> SqlResult<Option<Usuario>> {
        let mut stmt = conn.prepare(
            "SELECT id, nombre, apellido, email, legajo, tipo, password_hash, aprobado, momento_creacion, avatar_blob, avatar_mime 
             FROM usuarios 
             WHERE legajo = ?1"
        )?;

        let mut rows = stmt.query([legajo])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Usuario::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn crear(conn: &Connection, usuario: &Usuario) -> SqlResult<i64> {
        conn.execute(
            "INSERT INTO usuarios (nombre, apellido, email, legajo, tipo, password_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                usuario.nombre,
                usuario.apellido,
                usuario.email,
                usuario.legajo,
                usuario.tipo,
                usuario.password_hash,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn actualizar_avatar(
        conn: &Connection,
        id_usuario: i64,
        avatar_blob: &[u8],
        avatar_mime: &str,
    ) -> SqlResult<usize> {
        conn.execute(
            "UPDATE usuarios
             SET avatar_blob = ?1, avatar_mime = ?2
             WHERE id = ?3",
            rusqlite::params![avatar_blob, avatar_mime, id_usuario],
        )
    }

    pub fn eliminar(conn: &Connection, id_usuario: i64) -> SqlResult<usize> {
        conn.execute("DELETE FROM usuarios WHERE id = ?1", [id_usuario])
    }

    pub fn listar_profesores_pendientes(conn: &Connection) -> SqlResult<Vec<Usuario>> {
        let mut stmt = conn.prepare(
            "SELECT id, nombre, apellido, email, legajo, tipo, password_hash, aprobado, momento_creacion, avatar_blob, avatar_mime 
             FROM usuarios 
             WHERE tipo = 'P' AND aprobado = 0 
             ORDER BY momento_creacion ASC"
        )?;
        let filas = stmt.query_map([], Usuario::from_row)?;
        let mut profes = Vec::new();
        for p in filas {
            profes.push(p?);
        }
        Ok(profes)
    }

    pub fn aprobar_profesor(conn: &Connection, id: i64) -> SqlResult<usize> {
        conn.execute("UPDATE usuarios SET aprobado = 1 WHERE id = ?1", [id])
    }

    pub fn actualizar_rol(conn: &Connection, id: i64, nuevo_tipo: &str) -> SqlResult<usize> {
        conn.execute(
            "UPDATE usuarios SET tipo = ?1 WHERE id = ?2",
            rusqlite::params![nuevo_tipo, id],
        )
    }
    pub fn actualizar_perfil(
        conn: &Connection,
        usuario_id: i64,
        nombre: &str,
        apellido: &str,
    ) -> SqlResult<usize> {
        conn.execute(
            "
            UPDATE usuarios
            SET nombre = ?1,
                apellido = ?2
            WHERE id = ?3
            ",
            rusqlite::params![nombre, apellido, usuario_id,],
        )
    }

    pub fn actualizar_aprobacion(conn: &Connection, id: i64, aprobado: bool) -> SqlResult<usize> {
        conn.execute(
            "UPDATE usuarios SET aprobado = ?1 WHERE id = ?2",
            rusqlite::params![aprobado, id],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::usuario::Usuario;
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE usuarios (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                nombre TEXT NOT NULL,
                apellido TEXT NOT NULL,
                email TEXT UNIQUE NOT NULL,
                legajo INTEGER UNIQUE NOT NULL,
                tipo TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                aprobado BOOLEAN DEFAULT 0,
                momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
                avatar_blob BLOB,
                avatar_mime TEXT
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_crear_y_buscar_usuario_por_email() {
        let conn = crear_db_test();
        let nuevo_usuario = Usuario {
            id: 0,
            nombre: "Lionel".to_string(),
            apellido: "Messi".to_string(),
            email: "lmessi@fi.uba.ar".to_string(),
            legajo: 101010,
            tipo: "P".to_string(),
            password_hash: "hash_messi".to_string(),
            aprobado: false,
            momento_creacion: String::new(),
            avatar_blob: None,
            avatar_mime: None,
        };

        let id_generado = UsuarioRepository::crear(&conn, &nuevo_usuario).unwrap();
        assert_eq!(id_generado, 1);

        let usuario_db = UsuarioRepository::buscar_por_email(&conn, "lmessi@fi.uba.ar")
            .unwrap()
            .expect("Debería encontrar a Messi");

        assert_eq!(usuario_db.nombre, "Lionel");
        assert_eq!(usuario_db.legajo, 101010);
        assert_eq!(usuario_db.tipo, "P");
    }

    #[test]
    fn test_buscar_usuario_inexistente_devuelve_none() {
        let conn = crear_db_test();
        let resultado = UsuarioRepository::buscar_por_email(&conn, "cr7@fi.uba.ar").unwrap();
        assert!(resultado.is_none());
    }

    #[test]
    fn test_actualizar_rol() {
        let conn = crear_db_test();
        let nuevo_usuario = Usuario {
            id: 0,
            nombre: "Sergio".to_string(),
            apellido: "Agüero".to_string(),
            email: "kunaguero@fi.uba.ar".to_string(),
            legajo: 191919,
            tipo: "P".to_string(),
            password_hash: "hash_kun".to_string(),
            aprobado: true,
            momento_creacion: String::new(),
            avatar_blob: None,
            avatar_mime: None,
        };

        let id_generado = UsuarioRepository::crear(&conn, &nuevo_usuario).unwrap();

        let filas = UsuarioRepository::actualizar_rol(&conn, id_generado, "A").unwrap();
        assert_eq!(filas, 1);

        let usuario_db = UsuarioRepository::buscar_por_id(&conn, id_generado)
            .unwrap()
            .unwrap();
        assert_eq!(usuario_db.tipo, "A");
    }

    #[test]
    fn test_actualizar_aprobacion() {
        let conn = crear_db_test();
        let nuevo_usuario = Usuario {
            id: 0,
            nombre: "Gonzalo".to_string(),
            apellido: "Higuain".to_string(),
            email: "pipita@fi.uba.ar".to_string(),
            legajo: 909090,
            tipo: "P".to_string(),
            password_hash: "hash_pipita".to_string(),
            aprobado: false,
            momento_creacion: String::new(),
            avatar_blob: None,
            avatar_mime: None,
        };

        let id_generado = UsuarioRepository::crear(&conn, &nuevo_usuario).unwrap();

        let filas = UsuarioRepository::actualizar_aprobacion(&conn, id_generado, true).unwrap();
        assert_eq!(filas, 1);

        let usuario_db = UsuarioRepository::buscar_por_id(&conn, id_generado)
            .unwrap()
            .unwrap();
        assert!(usuario_db.aprobado);
    }
}
