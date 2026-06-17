use rouille::{Request, Response};
use rusqlite::Connection;
use crate::models::usuario::Usuario;
use crate::templates;
use crate::repository::sesion_repository::SesionRepository;
use crate::repository::usuario_repository::UsuarioRepository;

/// Extrae el valor de 'session_token' de los headers HTTP manualmente
pub fn extraer_token_sesion(request: &Request) -> Option<String> {
    if let Some(cookie_header) = request.header("Cookie") {
        // Puede haber varias cookies juntas
        for parte in cookie_header.split(';') {
            let parte = parte.trim();

            // Si la parte empieza con la clave que definimos, extraemos el valor
            if let Some(token) = parte.strip_prefix("session_token=") {
                return Some(token.to_string());
            }
        }
    }
    None
}

// Funcion auxiliar para obtener el usuario actual de la sesion
pub fn usuario_actual(request: &Request, conn: &Connection) -> Result<Usuario, Response> {
    // Extraer token de la cookie
    let token = match extraer_token_sesion(request) {
        Some(t) => t,
        None => {
            return Err(templates::response_mensaje_error_con_status(
                "No autorizado",
                "Debe iniciar sesión.",
                401,
            ));
        }
    };

    // Buscar la sesión en la base de datos
    let sesion = match SesionRepository::buscar_por_token(conn, &token) {
        Ok(Some(s)) => s,
        _ => {
            return Err(templates::response_mensaje_error_con_status(
                "Sesión inválida",
                "Su sesión expiró. Volvé a iniciar sesión.",
                401,
            ));
        }
    };

    // Buscar al usuario dueño de la sesión
    let usuario = match UsuarioRepository::buscar_por_id(conn, sesion.id_usuario) {
        Ok(Some(u)) => u,
        _ => {
            return Err(templates::response_mensaje_error_con_status(
                "Error interno",
                "No se pudo cargar el usuario.",
                500,
            ));
        }
    };
    
    Ok(usuario)
}
