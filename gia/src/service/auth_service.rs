use crate::models::usuario::Usuario;
use crate::repository::sesion_repository::SesionRepository;
use crate::repository::usuario_repository::UsuarioRepository;
use bcrypt::{hash, verify};
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AuthService;

impl AuthService {
    pub fn registrar_cuenta(
        conn: &Connection, // Agrego conexión
        legajo: i32,
        nombre: String,
        apellido: String,
        email: String,
        tipo: &str, // asi esta en las constantes (&str)
        password: &str,
    ) -> Result<Usuario, String> {
        if !Self::validar_email_fiuba(&email) {
            return Err("El email debe pertenecer a FIUBA".to_string());
        }

        // Verificar que el usuario no exista
        match UsuarioRepository::buscar_por_email(conn, &email) {
            Ok(Some(_)) => return Err("Ya existe un usuario con ese email".to_string()),
            Ok(None) => {} // Todo en orden, continuamos
            Err(e) => return Err(format!("Error consultando usuarios: {}", e)),
        }

        // Hashear la contraseña
        let password_hash = Self::hashear_password(password);

        // Armar struct
        let nuevo_usuario = Usuario {
            id: 0, // se asigna en la db
            legajo,
            nombre,
            apellido,
            email: email.clone(),
            tipo: tipo.to_string(),
            password_hash,
            aprobado: false, 
            momento_creacion: String::new(), // se asigna en la db
            avatar_blob: None,
            avatar_mime: None,
        };

        // Insertar en la base de datos y volver a obtener el usuario con los campos actualizados
        match UsuarioRepository::crear(conn, &nuevo_usuario) {
            Ok(_) => {
                // con esto se obtiene el id y el momento de creación
                match UsuarioRepository::buscar_por_email(conn, &email) {
                    Ok(Some(user)) => Ok(user),
                    _ => Err("Usuario creado, pero hubo un error al recuperarlo".to_string()),
                }
            }
            Err(e) => Err(format!("Error en la base de datos al crear cuenta: {}", e)),
        }
    }

    #[allow(dead_code)]
    pub fn login(
        conn: &Connection,
        email: &str,
        password: &str,
    ) -> Result<(Usuario, String), String> {
        let _ = SesionRepository::limpiar_expiradas(conn);

        match UsuarioRepository::buscar_por_email(conn, email) {
            Ok(Some(usuario)) => {
                if !usuario.aprobado {
                    return Err("Tu cuenta fue registrada pero está pendiente de aprobación.".to_string());
                }
                if Self::verificar_password(password, &usuario.password_hash) {
                    // Generar un token único basado en el tiempo actual
                    let time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    let token = format!("token_{}_{}", usuario.id, time);

                    match SesionRepository::crear(conn, &token, usuario.id) {
                        Ok(_) => Ok((usuario, token)),
                        Err(e) => Err(format!("Error al crear sesión: {}", e)),
                    }
                } else {
                    Err("Contraseña incorrecta".to_string())
                }
            }
            Ok(None) => Err("Usuario no encontrado".to_string()),
            Err(e) => Err(format!("Error al consultar la base de datos: {}", e)),
        }
    }

    pub fn validar_email_fiuba(email: &str) -> bool {
        email.ends_with("@fi.uba.ar")
    }

    // Metodos auxiliares

    // Aca podriamos usar un crate, por ahora queda asi
    fn hashear_password(password: &str) -> String {
        hash(password, 4).unwrap_or_else(|_| String::new())
    }

    fn verificar_password(password: &str, hash_guardado: &str) -> bool {
        verify(password, hash_guardado).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE usuarios (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                nombre TEXT NOT NULL, apellido TEXT NOT NULL, email TEXT UNIQUE NOT NULL,
                legajo INTEGER UNIQUE NOT NULL, tipo TEXT NOT NULL, password_hash TEXT NOT NULL,
                momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP, avatar_blob BLOB, avatar_mime TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE sesiones (
                token TEXT PRIMARY KEY, id_usuario INTEGER NOT NULL,
                momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_validar_email_fiuba() {
        assert!(AuthService::validar_email_fiuba("sdeluque@fi.uba.ar"));
        assert!(!AuthService::validar_email_fiuba("vegetta777@gmail.com"));
        assert!(!AuthService::validar_email_fiuba("rubius@uba.ar"));
    }

    #[test]
    fn test_registrar_cuenta_y_login_exitoso() {
        let conn = crear_db_test();

        let usuario = AuthService::registrar_cuenta(
            &conn,
            77777,
            "Samuel".to_string(),
            "De Luque".to_string(),
            "sdeluque@fi.uba.ar".to_string(),
            "S",
            "planeta_vegetta",
        )
        .unwrap();

        assert_eq!(usuario.nombre, "Samuel");

        let resultado_login = AuthService::login(&conn, "sdeluque@fi.uba.ar", "planeta_vegetta");
        assert!(resultado_login.is_ok());

        let (usuario_logueado, token) = resultado_login.unwrap();
        assert_eq!(usuario_logueado.email, "sdeluque@fi.uba.ar");
        assert!(token.starts_with("token_"));
    }

    #[test]
    fn test_login_falla_con_password_incorrecta() {
        let conn = crear_db_test();

        AuthService::registrar_cuenta(
            &conn,
            40404,
            "Ruben".to_string(),
            "Doblas".to_string(),
            "rdoblas@fi.uba.ar".to_string(),
            "S",
            "12345",
        )
        .unwrap();

        let resultado = AuthService::login(&conn, "rdoblas@fi.uba.ar", "clave_equivocada");
        assert!(resultado.is_err());
        assert_eq!(resultado.unwrap_err(), "Contraseña incorrecta");
    }

    #[test]
    fn test_registro_falla_email_duplicado() {
        let conn = crear_db_test();

        AuthService::registrar_cuenta(
            &conn,
            12345,
            "Ibai".to_string(),
            "Llanos".to_string(),
            "ibai@fi.uba.ar".to_string(),
            "S",
            "ibaiMason",
        )
        .unwrap();

        let resultado_duplicado = AuthService::registrar_cuenta(
            &conn,
            54321,
            "Gerard".to_string(),
            "Pique".to_string(),
            "ibai@fi.uba.ar".to_string(),
            "S",
            "123123",
        );

        assert!(resultado_duplicado.is_err());
        assert_eq!(
            resultado_duplicado.unwrap_err(),
            "Ya existe un usuario con ese email"
        );
    }
}
