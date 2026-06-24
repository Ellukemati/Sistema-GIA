use crate::models::usuario::Usuario;
use crate::repository::sesion_repository::SesionRepository;
use crate::repository::usuario_repository::UsuarioRepository;
use crate::templates;
use rouille::{Request, Response};
use rusqlite::Connection;

/// Extrae el valor de una cookie por su clave de los headers HTTP manualmente
pub fn extraer_cookie(request: &Request, clave: &str) -> Option<String> {
    let cookie_header = request.header("Cookie")?;
    let prefijo = format!("{}=", clave);
    // Puede haber varias cookies juntas
    for parte in cookie_header.split(';') {
        let parte = parte.trim();
        if let Some(valor) = parte.strip_prefix(&prefijo) {
            return Some(valor.to_string());
        }
    }
    None
}

/// Extrae el valor de 'session_token' de los headers HTTP manualmente
pub fn extraer_token_sesion(request: &Request) -> Option<String> {
    extraer_cookie(request, "session_token")
}

/// Carrito (borrador) de reserva persistido en la cookie `reserva_carrito`.
/// Guarda una unica fecha para toda la reserva y los ejemplares acumulados.
pub struct Carrito {
    pub fecha_inicio: Option<String>,
    pub fecha_fin: Option<String>,
    pub ejemplares: Vec<i64>,
}

impl Carrito {
    pub fn vacio() -> Self {
        Carrito {
            fecha_inicio: None,
            fecha_fin: None,
            ejemplares: Vec::new(),
        }
    }

    pub fn tiene_fechas(&self) -> bool {
        self.fecha_inicio.is_some() && self.fecha_fin.is_some()
    }
}

/// Lee el carrito desde la cookie `reserva_carrito`.
/// Formato: `<fecha_inicio>_<fecha_fin>_<id.id.id>` (ej: `2026-07-01_2026-07-10_3.7.12`).
pub fn leer_carrito(request: &Request) -> Carrito {
    let valor = match extraer_cookie(request, "reserva_carrito") {
        Some(v) => v,
        None => return Carrito::vacio(),
    };

    let mut partes = valor.splitn(3, '_');
    let inicio = partes.next().unwrap_or("").to_string();
    let fin = partes.next().unwrap_or("").to_string();
    let ids = partes.next().unwrap_or("");

    let ejemplares = ids
        .split('.')
        .filter_map(|s| s.parse::<i64>().ok())
        .collect();

    Carrito {
        fecha_inicio: if inicio.is_empty() {
            None
        } else {
            Some(inicio)
        },
        fecha_fin: if fin.is_empty() { None } else { Some(fin) },
        ejemplares,
    }
}

/// Arma el header `Set-Cookie` para persistir el carrito.
pub fn cookie_carrito(carrito: &Carrito) -> String {
    let inicio = carrito.fecha_inicio.clone().unwrap_or_default();
    let fin = carrito.fecha_fin.clone().unwrap_or_default();
    let ids = carrito
        .ejemplares
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(".");

    format!(
        "reserva_carrito={}_{}_{}; Path=/; Max-Age=86400",
        inicio, fin, ids
    )
}

/// Header `Set-Cookie` que borra el carrito (al finalizar la reserva).
pub fn cookie_carrito_vacio() -> String {
    "reserva_carrito=; Path=/; Max-Age=0".to_string()
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
