use crate::constants::{TIPO_ADMIN, TIPO_ALUMNO, TIPO_PROFESOR};
use rusqlite::{Result as SqlResult, Row};

/// Representa una Usuario segun la tabla `Usuario`
pub struct Usuario {
    pub id: i64,
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub legajo: i32,
    pub tipo: String,
    pub password_hash: String,
    //pub momento_creacion: String,
    pub imagen: Option<String>,
}

impl Usuario {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let tipo: String = row.get("tipo")?;
        debug_assert!(
            [TIPO_ALUMNO, TIPO_PROFESOR, TIPO_ADMIN].contains(&tipo.as_str()),
            "tipo de usuario invalido"
        );

        Ok(Usuario {
            id: row.get("id")?,
            nombre: row.get("nombre")?,
            apellido: row.get("apellido")?,
            email: row.get("email")?,
            legajo: row.get("legajo")?,
            tipo,
            password_hash: row.get("password_hash")?,
            //momento_creacion: row.get("momento_creacion")?,
            imagen: row.get("imagen")?,
        })
    }

    pub fn nombre_completo(&self) -> String {
        format!("{} {}", self.nombre, self.apellido)
    }

    #[allow(dead_code)]
    pub fn es_admin(&self) -> bool {
        self.tipo == TIPO_ADMIN
    }

    #[allow(dead_code)]
    pub fn es_profesor(&self) -> bool {
        self.tipo == TIPO_PROFESOR
    }

    #[allow(dead_code)]
    pub fn es_alumno(&self) -> bool {
        self.tipo == TIPO_ALUMNO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_mapeo_usuario_desde_bdd() {
        // base de datos temporal en RAM
        let conn = Connection::open_in_memory().unwrap();

        conn.execute(
            "CREATE TABLE Usuario (
                id INTEGER PRIMARY KEY,
                nombre TEXT,
                apellido TEXT,
                email TEXT,
                legajo INTEGER,
                tipo TEXT,
                password_hash TEXT,
                momento_creacion TEXT,
                imagen TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO Usuario (id, nombre, apellido, email, legajo, tipo, password_hash, momento_creacion) 
             VALUES (1, 'nombre1', 'apellido1', 'napellido1@fi.uba.ar', 12345, 'S', 'hash123', '2026-05-11')",
            [],
        ).unwrap();

        let mut stmt = conn.prepare("SELECT * FROM Usuario WHERE id = 1").unwrap();
        let usuario = stmt.query_row([], Usuario::from_row).unwrap();

        assert_eq!(usuario.nombre, "nombre1");
        assert_eq!(usuario.apellido, "apellido1");
        assert_eq!(usuario.legajo, 12345);
        assert!(usuario.es_alumno());
        assert_eq!(usuario.nombre_completo(), "nombre1 apellido1");
    }
}
