use crate::models::usuario::Usuario;
use crate::repository::usuario_repository::UsuarioRepository;
use rusqlite::Connection;

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
            momento_creacion: String::new(), // se asigna en la db
            avatar_direccion: None,
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

    pub fn login(conn: &Connection, email: &str, password: &str) -> Result<Usuario, String> {
        match UsuarioRepository::buscar_por_email(conn, email) {
            Ok(Some(usuario)) => {
                if Self::verificar_password(password, &usuario.password_hash) {
                    Ok(usuario)
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
        // TODO: Implementar algoritmo real
        format!("hash_{}", password)
    }

    fn verificar_password(password: &str, hash_guardado: &str) -> bool {
        let hash_calculado = Self::hashear_password(password);
        hash_calculado == hash_guardado
    }
}
