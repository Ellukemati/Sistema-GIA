use crate::service::auth_service::AuthService;
use crate::templates;

use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;
use tera::Context;
pub struct AuthHandler;

impl AuthHandler {
    pub fn mostrar_formulario_registro() -> Response {
        let ctx = Context::new();
        templates::response_html(templates::render("usuario_registro.html", &ctx))
    }

    pub fn mostrar_formulario_login() -> Response {
        let ctx = Context::new();

        templates::response_html(templates::render("usuario_login.html", &ctx))
    }

    pub fn procesar_registro(request: &Request, conn: &Connection) -> Response {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);

        let nombre = datos_parseados.get("nombre").cloned().unwrap_or_default();
        let apellido = datos_parseados.get("apellido").cloned().unwrap_or_default();
        let email = datos_parseados.get("email").cloned().unwrap_or_default();
        let tipo = datos_parseados.get("tipo").cloned().unwrap_or_default();
        let password = datos_parseados.get("password").cloned().unwrap_or_default();
        let legajo = match datos_parseados
            .get("legajo")
            .unwrap_or(&String::new())
            .parse::<i32>()
        {
            Ok(val) => val,
            Err(_) => {
                return templates::response_mensaje_error(
                    "Datos inválidos",
                    "Legajo inválido. Ingresá un número válido.",
                );
            }
        };

        match AuthService::registrar_cuenta(conn, legajo, nombre, apellido, email, &tipo, &password)
        {
            Ok(usuario) => templates::response_mensaje_exito(
                "¡Cuenta creada!",
                &format!(
                    "Bienvenido/a {} (ID: {}).",
                    usuario.nombre_completo(),
                    usuario.id
                ),
            ),

            Err(e) => templates::response_mensaje_error(
                "No se pudo completar el registro",
                &e.to_string(),
            ),
        }
    }

    pub fn procesar_login(request: &Request, conn: &Connection) -> Response {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);
        let email = datos_parseados.get("email").cloned().unwrap_or_default();
        let password = datos_parseados.get("password").cloned().unwrap_or_default();

        match AuthService::login(conn, &email, &password) {
            Ok((usuario, token)) => {
                let cookie_str =
                    format!("session_token={}; HttpOnly; Path=/; Max-Age=86400", token);
                templates::response_mensaje_exito(
                    "¡Inicio de sesión exitoso!",
                    &format!("Bienvenido/a {}.", usuario.nombre_completo()),
                )
                .with_additional_header("Set-Cookie", cookie_str)
            }

            Err(e) => templates::response_mensaje_error_con_status(
                "No se pudo iniciar sesión",
                &e.to_string(),
                401,
            ),
        }
    }

    fn parsear_formulario(cuerpo: &str) -> HashMap<String, String> {
        let mut mapa = HashMap::new();
        for par in cuerpo.split('&') {
            let mut partes = par.split('=');
            if let (Some(clave), Some(valor)) = (partes.next(), partes.next()) {
                let valor_decodificado = valor.replace("%40", "@").replace("+", " ");
                mapa.insert(clave.to_string(), valor_decodificado.trim().to_string());
            }
        }
        mapa
    }
}
