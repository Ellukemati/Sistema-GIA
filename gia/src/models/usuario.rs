use rusqlite::{Result as SqlResult, Row};
pub const TIPO_ADMIN: &str = "A";
pub const TIPO_PROFESOR: &str = "P";
pub const TIPO_ESTUDIANTE: &str = "E";
/// Representa una Usuario segun la tabla `Usuario`
pub struct Usuario {

    pub id: i64,            // ver si quedarnos con id o legajo como clave primaria
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub legajo: i32,
    pub tipo: String,
    pub password_hash: String,
    pub momento_creacion: String,
    pub imagen: Option<String>,
}

impl Usuario {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Usuario {
            id: row.get("id")?,
            nombre: row.get("nombre")?,
            //segundo_nombre: row.get("segundo_nombre")?,
            apellido: row.get("apellido")?,
            //segundo_apellido: row.get("segundo_apellido")?,
            email: row.get("email")?,
            legajo: row.get("legajo")?,
            tipo: row.get("tipo")?,
            password_hash: row.get("password_hash")?,
            momento_creacion: row.get("momento_creacion")?,
            imagen: row.get("imagen")?,
        })
    }

    pub fn nombre_completo(&self) -> String {
        format!("{} {}", self.nombre, self.apellido)
    }

    pub fn es_admin(&self) -> bool {
        self.tipo == TIPO_ADMIN
    }

    pub fn es_profesor(&self) -> bool {
        self.tipo == TIPO_PROFESOR
    }

    pub fn es_estudiante(&self) -> bool {
        self.tipo == TIPO_ESTUDIANTE
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
        ).unwrap();

        conn.execute(
            "INSERT INTO Usuario (id, nombre, apellido, email, legajo, tipo, password_hash, momento_creacion) 
             VALUES (1, 'nombre1', 'apellido1', 'napellido1@fi.uba.ar', 12345, 'E', 'hash123', '2026-05-11')",
            [],
        ).unwrap();

        let mut stmt = conn.prepare("SELECT * FROM Usuario WHERE id = 1").unwrap();
        let usuario = stmt.query_row([], |row| Usuario::from_row(row)).unwrap();

        assert_eq!(usuario.nombre, "nombre1");
        assert_eq!(usuario.apellido, "apellido1");
        assert_eq!(usuario.legajo, 12345);
        assert!(usuario.es_estudiante());
        assert_eq!(usuario.nombre_completo(), "nombre1 apellido1");
    }
}