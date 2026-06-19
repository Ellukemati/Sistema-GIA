use crate::constants::{TIPO_ADMIN, TIPO_PROFESOR};
use rusqlite::{Result as SqlResult, Row};
use serde::Serialize;

/// Representa una Usuario segun la tabla `Usuario`
#[derive(Debug, Serialize)]
pub struct Usuario {
    pub id: i64,
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub legajo: i32,
    pub tipo: String,
    pub password_hash: String,
    pub aprobado: bool, // 0 = Falso (En espera), 1 = Verdadero (Aprobado)
    pub momento_creacion: String,
    pub avatar_blob: Option<Vec<u8>>,
    pub avatar_mime: Option<String>,
}

impl Usuario {
    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let tipo: String = row.get("tipo")?;
        debug_assert!(
            [TIPO_PROFESOR, TIPO_ADMIN].contains(&tipo.as_str()),
            "tipo de usuario invalido"
        );
        let aprobado_int: i32 = row.get("aprobado")?;

        Ok(Usuario {
            id: row.get("id")?,
            nombre: row.get("nombre")?,
            apellido: row.get("apellido")?,
            email: row.get("email")?,
            legajo: row.get("legajo")?,
            tipo,
            password_hash: row.get("password_hash")?,
            aprobado: aprobado_int != 0, // Pasarlo a bool
            momento_creacion: row.get("momento_creacion")?,
            avatar_blob: row.get("avatar_blob")?,
            avatar_mime: row.get("avatar_mime")?,
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

}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_mapeo_usuario_desde_bdd() {
        // base de datos temporal en RAM
        let conn = Connection::open_in_memory().unwrap();

        // ACÁ: Sumamos la columna aprobado a la tabla en memoria
        conn.execute(
            "CREATE TABLE usuarios (
                id INTEGER PRIMARY KEY,
                nombre TEXT,
                apellido TEXT,
                email TEXT,
                legajo INTEGER,
                tipo TEXT,
                password_hash TEXT,
                aprobado BOOLEAN,
                momento_creacion TEXT,
                avatar_blob BLOB,
                avatar_mime TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO usuarios (id, nombre, apellido, email, legajo, tipo, password_hash, aprobado, momento_creacion) 
             VALUES (1, 'Nombre1', 'Apellido1', 'aaa@fi.uba.ar', 12345, 'P', 'hash123', 1, '2026-05-11')",
            [],
        ).unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, nombre, apellido, email, legajo, tipo, password_hash, aprobado, momento_creacion, avatar_blob, avatar_mime 
             FROM usuarios 
             WHERE id = 1"
        ).unwrap();
        let usuario = stmt.query_row([], Usuario::from_row).unwrap();

        assert_eq!(usuario.nombre, "Nombre1");
        assert_eq!(usuario.apellido, "Apellido1");
        assert_eq!(usuario.legajo, 12345);
        assert!(usuario.es_profesor());
        assert!(usuario.aprobado); // Verificamos que se mapeó bien el booleano
        assert_eq!(usuario.nombre_completo(), "Nombre1 Apellido1");
    }

    #[test]
    fn test_identificacion_de_roles() {
        let admin = Usuario {
            id: 2,
            nombre: "Bruce".to_string(),
            apellido: "Wayne".to_string(),
            email: "batman@fi.uba.ar".to_string(),
            legajo: 1,
            tipo: "A".to_string(),
            password_hash: "1234".to_string(),
            aprobado: true, // El admin siempre está aprobado
            momento_creacion: String::new(),
            avatar_blob: None,
            avatar_mime: None,
        };

        let profe = Usuario {
            id: 3,
            nombre: "Walter".to_string(),
            apellido: "White".to_string(),
            email: "heisenberg@fi.uba.ar".to_string(),
            legajo: 2,
            tipo: "P".to_string(),
            password_hash: "1234".to_string(),
            aprobado: false, // Profe recién registrado (en espera)
            momento_creacion: String::new(),
            avatar_blob: None,
            avatar_mime: None,
        };

        // Verificamos Admin
        assert!(admin.es_admin());
        assert!(!admin.es_profesor());

        // Verificamos Profe
        assert!(profe.es_profesor());
        assert!(!profe.es_admin());
    }
}