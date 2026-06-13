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
        match templates::render("usuario_registro.html", &ctx) {
            Ok(html) => Response::html(html),
            Err(e) => Response::text(format!("Error renderizando plantilla: {}", e))
                .with_status_code(500),
        }
    }

    pub fn mostrar_formulario_login() -> Response {
        let html = include_str!("../../templates/usuario_login.html");
        Response::html(html)
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
            Err(_) => return Response::html("<div style='color:red;'>Legajo inválido</div>"),
        };

        match AuthService::registrar_cuenta(conn, legajo, nombre, apellido, email, &tipo, &password)
        {
            Ok(usuario) => {
                let exito_html = format!(
                    "<div style='color:green;'>¡Éxito! Bienvenido/a {} (ID: {})</div>",
                    usuario.nombre_completo(),
                    usuario.id
                );
                Response::html(exito_html)
            }
            Err(e) => {
                let error_html = format!("<div style='color:red;'>Error: {}</div>", e);
                Response::html(error_html)
            }
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
                // cookie. HttpOnly por seguridad, Max-Age de 24 hs (86400 segundos)
                let cookie_str =
                    format!("session_token={}; HttpOnly; Path=/; Max-Age=86400", token);

                let exito_html = format!(
                    "<div style='color:green;'>¡Login exitoso! Bienvenido/a {}</div>",
                    usuario.nombre_completo()
                );

                // Se retorna el HTML inyectando el header de la cookie
                Response::html(exito_html).with_additional_header("Set-Cookie", cookie_str)
            }
            Err(e) => {
                let error_html = format!("<div style='color:red;'>Error: {}</div>", e);
                Response::html(error_html).with_status_code(401)
            }
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
