use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;

use crate::constants::OFFSET_ARG;
use crate::models::usuario::Usuario;
use crate::repository::{
    sesion_repository::SesionRepository, usuario_repository::UsuarioRepository,
};
use crate::templates;

pub fn parsear_formulario(b: &str) -> HashMap<String, String> {
    b.split('&')
        .filter_map(|par| {
            let mut pt = par.split('=');
            Some((
                pt.next()?.to_string(),
                pt.next()?.replace("%40", "@").replace("+", " "),
            ))
        })
        .collect()
}

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

/// Formatea un rango de fechas en español, por ejemplo:
/// "para el próximo 5 de julio" (Si es un solo día)
/// "desde el 5 de julio hasta el 10 de julio" (Si es un rango dentro del mismo año)
/// "desde el 27 de diciembre de 2026 hasta el 10 de enero de 2027" (Si es un rango tiene distintos años de inicio y fin)
pub fn formatear_rango_fechas(fecha_inicio_str: &str, fecha_fin_str: &str) -> String {
    let meses = [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ];

    let inicio = match NaiveDate::parse_from_str(fecha_inicio_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return format!("desde el {} hasta el {}", fecha_inicio_str, fecha_fin_str),
    };

    let fin = match NaiveDate::parse_from_str(fecha_fin_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return format!("desde el {} hasta el {}", fecha_inicio_str, fecha_fin_str),
    };

    let dia_ini = inicio.day();
    let mes_ini = meses[(inicio.month() as usize) - 1];
    let anio_ini = inicio.year();

    let dia_fin = fin.day();
    let mes_fin = meses[(fin.month() as usize) - 1];
    let anio_fin = fin.year();

    if inicio == fin {
        format!("para el próximo {} de {}", dia_ini, mes_ini)
    } else if anio_ini != anio_fin {
        format!(
            "desde el {} de {} de {} hasta el {} de {} de {}",
            dia_ini, mes_ini, anio_ini, dia_fin, mes_fin, anio_fin
        )
    } else {
        format!(
            "desde el {} de {} hasta el {} de {}",
            dia_ini, mes_ini, dia_fin, mes_fin
        )
    }
}

pub fn ahora_utc_string() -> String {
    Utc::now()
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Convierte un instante UTC a la zona horaria de Argentina.
pub fn a_zona_arg(dt: DateTime<Utc>) -> DateTime<FixedOffset> {
    dt.with_timezone(&FixedOffset::west_opt(OFFSET_ARG).unwrap())
}

/// Convierte un string "YYYY-MM-DD HH:MM:SS" guardado en UTC a hora ARG
/// formateada igual. Para mostrar cualquier timestamp de la DB en el front.
pub fn utc_str_a_arg(fecha: &str) -> String {
    if fecha.is_empty() || fecha == "---" {
        return fecha.to_string();
    }
    match NaiveDateTime::parse_from_str(fecha, "%Y-%m-%d %H:%M:%S") {
        Ok(ndt) => a_zona_arg(Utc.from_utc_datetime(&ndt))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        Err(_) => fecha.to_string(),
    }
}

/// Carrito (borrador) de reserva persistido en la cookie `reserva_carrito`.
/// Guarda una unica fecha para toda la reserva y los ejemplares acumulados.
pub struct Carrito {
    pub fecha_inicio: Option<String>,
    pub fecha_fin: Option<String>,
    pub motivo: Option<String>,
    pub ejemplares: Vec<i64>,
}

impl Carrito {
    pub fn vacio() -> Self {
        Carrito {
            fecha_inicio: None,
            fecha_fin: None,
            motivo: None,
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

    let mut partes = valor.splitn(4, '_');

    let inicio = partes.next().unwrap_or("").to_string();
    let fin = partes.next().unwrap_or("").to_string();
    let motivo = partes.next().unwrap_or("").replace("%20", " ");
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
        motivo: if motivo.is_empty() {
            None
        } else {
            Some(motivo)
        },
        ejemplares,
    }
}

/// Arma el header `Set-Cookie` para persistir el carrito.
pub fn cookie_carrito(carrito: &Carrito) -> String {
    let inicio = carrito.fecha_inicio.clone().unwrap_or_default();
    let fin = carrito.fecha_fin.clone().unwrap_or_default();

    let motivo = carrito
        .motivo
        .clone()
        .unwrap_or_default()
        .replace(' ', "%20");

    let ids = carrito
        .ejemplares
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(".");

    format!(
        "reserva_carrito={}_{}_{}_{}; Path=/; Max-Age=86400",
        inicio, fin, motivo, ids
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatear_rango_fechas_mismo_dia() {
        let res = formatear_rango_fechas("2026-08-18", "2026-08-18");
        assert_eq!(res, "para el próximo 18 de agosto");
    }

    #[test]
    fn test_formatear_rango_fechas_mismo_anio() {
        let res = formatear_rango_fechas("2026-08-18", "2026-08-22");
        assert_eq!(res, "desde el 18 de agosto hasta el 22 de agosto");
    }

    #[test]
    fn test_formatear_rango_fechas_distinto_anio() {
        let res = formatear_rango_fechas("2026-12-27", "2027-01-10");
        assert_eq!(
            res,
            "desde el 27 de diciembre de 2026 hasta el 10 de enero de 2027"
        );
    }
}
