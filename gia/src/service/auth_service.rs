use bcrypt::{hash, verify};
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::{BCRYPT_COST_FACTOR, EXPIRACION_RESTABLECIMIENTO_PASSWORD_SEGUNDOS};
use crate::models::usuario::Usuario;
use crate::repository::sesion_repository::SesionRepository;
use crate::repository::token_repository::TokenRepository;
use crate::repository::usuario_repository::UsuarioRepository;
use crate::service::mail_service::MailService;

/*
use crate::constants::EXPIRACION_INVITACION;
use crate::models::invitacion::Invitacion;
use crate::repository::invitacion_repository::InvitacionRepository;
*/

pub struct AuthService;

impl AuthService {
    pub fn registrar_cuenta(
        conn: &Connection,
        legajo: i32,
        nombre: String,
        apellido: String,
        email: String,
        tipo: &str,
        password: &str,
    ) -> Result<Usuario, String> {
        if !Self::validar_email_fiuba(&email) {
            return Err("El email debe pertenecer a FIUBA".to_string());
        }

        match UsuarioRepository::buscar_por_email(conn, &email) {
            Ok(Some(_)) => {
                return Err("Ya existe un usuario registrado con el email ingresado.".to_string());
            }
            Ok(None) => {}
            Err(e) => return Err(format!("Error consultando usuarios: {}", e)),
        }

        let password_hash = Self::hashear_password(password);

        let nuevo_usuario = Usuario {
            id: 0,
            legajo,
            nombre,
            apellido,
            email: email.clone(),
            tipo: tipo.to_string(),
            password_hash,
            aprobado: false,
            momento_creacion: String::new(),
            avatar_blob: None,
            avatar_mime: None,
        };

        match UsuarioRepository::crear(conn, &nuevo_usuario) {
            Ok(_) => match UsuarioRepository::buscar_por_email(conn, &email) {
                Ok(Some(user)) => Ok(user),
                _ => Err("Usuario creado, pero hubo un error al recuperarlo".to_string()),
            },
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("usuarios.legajo")
                    || err_str.contains("UNIQUE constraint failed")
                {
                    Err("Ya hay un usuario registrado con el legajo ingresado.".to_string())
                } else {
                    Err(format!(
                        "Error en la base de datos al crear cuenta: {}",
                        err_str
                    ))
                }
            }
        }
    }

    pub fn login(
        conn: &Connection,
        email: &str,
        password: &str,
    ) -> Result<(Usuario, String), String> {
        let _ = SesionRepository::limpiar_expiradas(conn);

        match UsuarioRepository::buscar_por_email(conn, email) {
            Ok(Some(usuario)) => {
                if !usuario.aprobado {
                    return Err(
                        "Tu cuenta está registrada pero está pendiente de aprobación.\n\n\
                         Esperá a que un administrador la apruebe.\n\n\
                         Te llegará una notificación al correo cuando ocurra."
                            .to_string(),
                    );
                }

                if Self::verificar_password(password, &usuario.password_hash) {
                    let time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    let token = format!("token_{}_{}", usuario.id, time);

                    match SesionRepository::crear(conn, &token, usuario.id) {
                        Ok(_) => {
                            crate::logger::info(&format!(
                                "Inicio de sesión exitoso: {}",
                                usuario.email
                            ));
                            Ok((usuario, token))
                        }
                        Err(e) => {
                            crate::logger::error(&format!(
                                "Error al crear sesión para {}: {}",
                                usuario.email, e
                            ));
                            Err(format!("Error al crear sesión: {}", e))
                        }
                    }
                } else {
                    Err("Contraseña incorrecta.".to_string())
                }
            }
            Ok(None) => Err("El email ingresado no está registrado.".to_string()),
            Err(e) => Err(format!("Error al consultar la base de datos: {}", e)),
        }
    }

    pub fn validar_email_fiuba(email: &str) -> bool {
        email.ends_with("@fi.uba.ar")
    }

    pub fn solicitar_restablecimiento_password(
        conn: &Connection,
        email: &str,
    ) -> Result<(), String> {
        let usuario =
            match UsuarioRepository::buscar_por_email(conn, email).map_err(|e| e.to_string())? {
                Some(u) => u,
                None => {
                    return Ok(());
                }
            };

        if !usuario.aprobado {
            return Ok(());
        }

        let ahora_nano = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let token = format!("{:x}_{}", ahora_nano, usuario.id);

        let ahora_segundos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let expira_en = ahora_segundos + EXPIRACION_RESTABLECIMIENTO_PASSWORD_SEGUNDOS;

        TokenRepository::guardar(conn, usuario.id, &token, expira_en)
            .map_err(|e| format!("Error en repositorio de tokens: {}", e))?;

        let link = format!(
            "http://localhost:8080/restablecer-contrasena?token={}",
            token
        );
        let nombre_completo = format!("{} {}", usuario.nombre, usuario.apellido);

        MailService::enviar_link_restablecimiento_password(
            &usuario.email,
            &nombre_completo,
            &link,
        )?;

        Ok(())
    }

    pub fn cambiar_password_usuario(
        conn: &Connection,
        id_usuario: i64,
        nuevo_password: &str,
    ) -> Result<(), String> {
        let nuevo_hash = Self::hashear_password(nuevo_password);

        conn.execute(
            "
            UPDATE usuarios
            SET password_hash = ?
            WHERE id = ?
            ",
            rusqlite::params![nuevo_hash, id_usuario,],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn restablecer_password(
        conn: &Connection,
        token: &str,
        nuevo_password: &str,
    ) -> Result<(), String> {
        let ahora_segundos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let id_usuario = match TokenRepository::buscar_valido(conn, token, ahora_segundos)
            .map_err(|e| e.to_string())?
        {
            Some(id) => id,
            None => {
                return Err("El enlace de restablecimiento es inválido o ha expirado.".to_string());
            }
        };

        let nuevo_hash = Self::hashear_password(nuevo_password);

        conn.execute(
            "UPDATE usuarios SET password_hash = ? WHERE id = ?",
            rusqlite::params![nuevo_hash, id_usuario],
        )
        .map_err(|e| format!("Error al actualizar contraseña: {}", e))?;

        let _ = TokenRepository::eliminar(conn, id_usuario);

        Ok(())
    }

    fn hashear_password(password: &str) -> String {
        hash(password, BCRYPT_COST_FACTOR).unwrap_or_else(|_| String::new())
    }

    fn verificar_password(password: &str, hash_guardado: &str) -> bool {
        verify(password, hash_guardado).unwrap_or(false)
    }

    /* INVITACIÓN DE NUEVOS USUARIOS POR MAIL
    pub fn invitar_usuario(conn: &Connection, email: &str, tipo: &str) -> Result<(), String> {
        if !Self::validar_email_fiuba(email) {
            return Err("El email debe ser FIUBA.".to_string());
        }

        if let Ok(Some(_)) = UsuarioRepository::buscar_por_email(conn, email) {
            return Err("Ya hay un usuario registrado con el email ingresado.".to_string());
        }

        let ahora_nano = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let token = format!("inv_{:x}", ahora_nano);

        let ahora_segundos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expira_en = ahora_segundos + EXPIRACION_INVITACION_SEGUNDOS;

        // Instanciamos el modelo intermedio
        let nueva_invitacion = Invitacion {
            email: email.to_string(),
            token: token.clone(),
            tipo: tipo.to_string(),
            expira_en,
        };

        InvitacionRepository::guardar(conn, &nueva_invitacion)
            .map_err(|e| format!("Error en repositorio de invitaciones: {}", e))?;

        let link = format!("http://localhost:8080/registro-invitacion?token={}", token);
        MailService::enviar_link_invitacion(email, tipo, &link)?;

        Ok(())
    }

    pub fn registrar_por_invitacion(
        conn: &Connection,
        token: &str,
        nombre: String,
        apellido: String,
        legajo: i32,
        password: &str,
    ) -> Result<(), String> {
        let ahora_segundos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let invitacion = match InvitacionRepository::buscar_valido(conn, token, ahora_segundos)
            .map_err(|e| e.to_string())?
        {
            Some(inv) => inv,
            None => {
                let _ = InvitacionRepository::eliminar(conn, token);
                return Err("El enlace de invitación es inválido o ha expirado.".to_string());
            }
        };

        match UsuarioRepository::buscar_por_legajo(conn, legajo) {
            Ok(Some(_)) => {
                return Err("Ya hay un usuario registrado con el legajo ingresado.".to_string());
            }
            Ok(None) => {}
            Err(e) => return Err(format!("Error consultando legajos: {}", e)),
        }

        let password_hash = Self::hashear_password(password);

        let nuevo_usuario = Usuario {
            id: 0,
            legajo,
            nombre,
            apellido,
            email: invitacion.email.clone(),
            tipo: invitacion.tipo,
            password_hash,
            aprobado: false,
            momento_creacion: String::new(),
            avatar_blob: None,
            avatar_mime: None,
        };

        let id_generado = UsuarioRepository::crear(conn, &nuevo_usuario)
            .map_err(|e| format!("Error en el repositorio al crear cuenta: {}", e))?;

        // Es aprobado automáticamente porque fue invitado por un admin y no necesita aprobación manual
        UsuarioRepository::actualizar_aprobacion(conn, id_generado, true)
            .map_err(|e| format!("Error al activar los permisos en el repositorio: {}", e))?;

        let _ = InvitacionRepository::eliminar(conn, token);

        Ok(())
    }
    */
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::Mutex;

    // Bloqueo global para evitar que los hilos le peguen en simultáneo a Mailtrap
    static LOCK_MAILTRAP: Mutex<()> = Mutex::new(());

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE usuarios (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                nombre TEXT NOT NULL, apellido TEXT NOT NULL, email TEXT UNIQUE NOT NULL,
                legajo INTEGER UNIQUE NOT NULL, tipo TEXT NOT NULL, password_hash TEXT NOT NULL, aprobado BOOLEAN DEFAULT 0,
                momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP, avatar_blob BLOB, avatar_mime TEXT
            )",
            [],
        ).unwrap();
        conn.execute(
            "CREATE TABLE sesiones (
                token TEXT PRIMARY KEY, id_usuario INTEGER NOT NULL,
                momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE tokens_restablecimiento_contrasena (
                id_usuario INTEGER NOT NULL PRIMARY KEY,
                token TEXT NOT NULL UNIQUE,
                expira_en INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE tokens_invitacion (
                email TEXT NOT NULL PRIMARY KEY,
                token TEXT NOT NULL UNIQUE,
                tipo TEXT NOT NULL,
                expira_en INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();

        conn
    }

    fn esperar_cuota_mailtrap() {
        if !crate::constants::MOCK_MAILS {
            println!("Esperando 10.5 segundos para liberar la cuota de Mailtrap...");
            std::thread::sleep(std::time::Duration::from_millis(10500));
        }
    }

    #[test]
    fn test_validar_email_fiuba() {
        assert!(AuthService::validar_email_fiuba("sdeluque@fi.uba.ar"));
        assert!(!AuthService::validar_email_fiuba("vegetta777@gmail.com"));
        assert!(!AuthService::validar_email_fiuba("rubius@uba.ar"));
    }

    #[test]
    fn test_registrar_cuenta_y_login() {
        let conn = crear_db_test();

        let usuario = AuthService::registrar_cuenta(
            &conn,
            123456,
            "Samuel".to_string(),
            "De Luque".to_string(),
            "sdeluque@fi.uba.ar".to_string(),
            "P",
            "samuel123",
        )
        .unwrap();

        assert_eq!(usuario.nombre, "Samuel");

        conn.execute(
            "UPDATE usuarios SET aprobado = 1 WHERE email = 'sdeluque@fi.uba.ar'",
            [],
        )
        .unwrap();

        let resultado_login = AuthService::login(&conn, "sdeluque@fi.uba.ar", "samuel123");
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
            "P",
            "12345",
        )
        .unwrap();

        conn.execute(
            "UPDATE usuarios SET aprobado = 1 WHERE email = 'rdoblas@fi.uba.ar'",
            [],
        )
        .unwrap();

        let resultado = AuthService::login(&conn, "rdoblas@fi.uba.ar", "clave_equivocada");
        assert!(resultado.is_err());
        assert_eq!(resultado.unwrap_err(), "Contraseña incorrecta.");
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
            "P",
            "ibaiMason",
        )
        .unwrap();

        let resultado_duplicado = AuthService::registrar_cuenta(
            &conn,
            54321,
            "Gerard".to_string(),
            "Pique".to_string(),
            "ibai@fi.uba.ar".to_string(),
            "P",
            "123123",
        );

        assert!(resultado_duplicado.is_err());
        assert_eq!(
            resultado_duplicado.unwrap_err(),
            "Ya existe un usuario registrado con el email ingresado."
        );
    }

    #[test]
    fn test_solicitar_restablecimiento_password_usuario_inexistente() {
        let conn = crear_db_test();
        let resultado =
            AuthService::solicitar_restablecimiento_password(&conn, "incognito@fi.uba.ar");
        assert!(resultado.is_ok())
    }

    #[test]
    fn test_restablecer_password_falla_por_token_invalido() {
        let conn = crear_db_test();

        let resultado =
            AuthService::restablecer_password(&conn, "token_inexistente_123", "nueva_clave_larga");

        assert!(resultado.is_err());
        assert_eq!(
            resultado.unwrap_err(),
            "El enlace de restablecimiento es inválido o ha expirado."
        );
    }

    // Ignorado para no enviar mails reales a Mailtrap durante pruebas automáticas (salvo usando cargo test -- --include-ignored),
    // pero se puede ejecutar manualmente para verificar el flujo completo. MOCK_MAILS = true para no enviar mails a Mailtrap.
    #[test]
    #[ignore]
    fn test_circuito_completo_restablecimiento_password() {
        let _guard = LOCK_MAILTRAP.lock().unwrap();

        let conn = crear_db_test();

        let _ = AuthService::registrar_cuenta(
            &conn,
            99999,
            "Van".to_string(),
            "Gogh".to_string(),
            "vgogh@fi.uba.ar".to_string(),
            "P",
            "clavegogh123",
        )
        .unwrap();

        // Hay que aprobar el usuario para poder restablecer la contraseña
        conn.execute("UPDATE usuarios SET aprobado = 1 WHERE id = 1", [])
            .unwrap();

        let resultado_solicitud =
            AuthService::solicitar_restablecimiento_password(&conn, "vgogh@fi.uba.ar");
        esperar_cuota_mailtrap();
        assert!(resultado_solicitud.is_ok());

        let token_guardado: String = conn
            .query_row(
                "SELECT token FROM tokens_restablecimiento_contrasena WHERE id_usuario = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let resultado_cambio =
            AuthService::restablecer_password(&conn, &token_guardado, "clavenueva789");
        assert!(resultado_cambio.is_ok());

        let token_existe: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens_restablecimiento_contrasena WHERE token = ?",
                [token_guardado],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(token_existe, 0);

        conn.execute("UPDATE usuarios SET aprobado = 1 WHERE id = 1", [])
            .unwrap();

        let login_viejo = AuthService::login(&conn, "vgogh@fi.uba.ar", "clavegogh123");
        assert!(login_viejo.is_err());

        let login_nuevo = AuthService::login(&conn, "vgogh@fi.uba.ar", "clavenueva789");
        assert!(login_nuevo.is_ok());
    }

    #[test]
    #[ignore]
    fn test_restablecer_password_falla_por_token_expirado() {
        let _guard = LOCK_MAILTRAP.lock().unwrap();

        let conn = crear_db_test();

        let usuario = AuthService::registrar_cuenta(
            &conn,
            55555,
            "Lionel".to_string(),
            "Messi".to_string(),
            "lmessi@fi.uba.ar".to_string(),
            "P",
            "clavebase123",
        )
        .unwrap();

        // Hay que aprobar el usuario para poder restablecer la contraseña
        conn.execute("UPDATE usuarios SET aprobado = 1 WHERE id = 1", [])
            .unwrap();

        AuthService::solicitar_restablecimiento_password(&conn, "lmessi@fi.uba.ar").unwrap();
        esperar_cuota_mailtrap();

        let token_guardado: String = conn
            .query_row(
                "SELECT token FROM tokens_restablecimiento_contrasena WHERE id_usuario = ?",
                [usuario.id],
                |row| row.get(0),
            )
            .unwrap();

        conn.execute(
            "UPDATE tokens_restablecimiento_contrasena SET expira_en = 0 WHERE token = ?",
            [&token_guardado],
        )
        .unwrap();

        let resultado =
            AuthService::restablecer_password(&conn, &token_guardado, "nueva_clave_larga");

        assert!(resultado.is_err());
        assert_eq!(
            resultado.unwrap_err(),
            "El enlace de restablecimiento es inválido o ha expirado."
        );
    }

    /*
    #[test]
    #[ignore]
    fn test_circuito_invitacion_admin() {
        let _guard = LOCK_MAILTRAP.lock().unwrap();

        let conn = crear_db_test();

        let resultado_invitacion =
            AuthService::invitar_usuario(&conn, "admin_nuevo@fi.uba.ar", "A");
        esperar_cuota_mailtrap();
        assert!(resultado_invitacion.is_ok());

        let token_generado: String = conn
            .query_row(
                "SELECT token FROM tokens_invitacion WHERE email = 'admin_nuevo@fi.uba.ar'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let resultado_alta = AuthService::registrar_por_invitacion(
            &conn,
            &token_generado,
            "Admin".to_string(),
            "Nuevo".to_string(),
            85214,
            "adminpass123",
        );
        assert!(resultado_alta.is_ok());

        let cantidad_tokens: i32 = conn
            .query_row("SELECT COUNT(*) FROM tokens_invitacion", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(cantidad_tokens, 0);

        let usuario_creado: Usuario =
            UsuarioRepository::buscar_por_email(&conn, "admin_nuevo@fi.uba.ar")
                .unwrap()
                .unwrap();

        assert_eq!(usuario_creado.nombre, "Admin");
        assert_eq!(usuario_creado.tipo, "A");
        assert!(usuario_creado.aprobado);

        let login = AuthService::login(&conn, "admin_nuevo@fi.uba.ar", "adminpass123");
        assert!(login.is_ok());
    }

    #[test]
    fn test_invitacion_falla_si_usuario_ya_existe() {
        let conn = crear_db_test();

        AuthService::registrar_cuenta(
            &conn,
            7777,
            "Existente".to_string(),
            "User".to_string(),
            "registrado@fi.uba.ar".to_string(),
            "P",
            "123456",
        )
        .unwrap();

        let resultado_invitar = AuthService::invitar_usuario(&conn, "registrado@fi.uba.ar", "A");
        assert!(resultado_invitar.is_err());
        assert_eq!(
            resultado_invitar.unwrap_err(),
            "Ya hay un usuario registrado con el email ingresado."
        );
    }

    #[test]
    #[ignore]
    fn test_registro_invitacion_falla_legajo_duplicado() {
        let _guard = LOCK_MAILTRAP.lock().unwrap();

        let conn = crear_db_test();

        AuthService::registrar_cuenta(
            &conn,
            4444,
            "Juan".to_string(),
            "Perez".to_string(),
            "jperez@fi.uba.ar".to_string(),
            "P",
            "clave123",
        )
        .unwrap();

        AuthService::invitar_usuario(&conn, "invitado_nuevo@fi.uba.ar", "P").unwrap();
        esperar_cuota_mailtrap();

        let token_generado: String = conn
            .query_row(
                "SELECT token FROM tokens_invitacion WHERE email = 'invitado_nuevo@fi.uba.ar'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let resultado_alta = AuthService::registrar_por_invitacion(
            &conn,
            &token_generado,
            "Mateo".to_string(),
            "Gomez".to_string(),
            4444,
            "pass789",
        );

        assert!(resultado_alta.is_err());
        assert_eq!(
            resultado_alta.unwrap_err(),
            "Ya hay un usuario registrado con el legajo ingresado."
        );
    }

    #[test]
    #[ignore]
    fn test_registro_invitacion_falla_token_expirado() {
        let _guard = LOCK_MAILTRAP.lock().unwrap();

        let conn = crear_db_test();

        AuthService::invitar_usuario(&conn, "viejo@fi.uba.ar", "P").unwrap();
        esperar_cuota_mailtrap();

        let token_generado: String = conn
            .query_row(
                "SELECT token FROM tokens_invitacion WHERE email = 'viejo@fi.uba.ar'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        conn.execute(
            "UPDATE tokens_invitacion SET expira_en = 0 WHERE token = ?",
            [&token_generado],
        )
        .unwrap();

        let resultado_alta = AuthService::registrar_por_invitacion(
            &conn,
            &token_generado,
            "Test".to_string(),
            "User".to_string(),
            998877,
            "password123",
        );

        assert!(resultado_alta.is_err());
        assert_eq!(
            resultado_alta.unwrap_err(),
            "El enlace de invitación es inválido o ha expirado."
        );
    }
    */
}
