use crate::service::auth_service::AuthService;
use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;

pub struct AuthHandler;

impl AuthHandler {
    pub fn mostrar_formulario_registro() -> Response {
        let html = include_str!("../templates/registro.html");
        Response::html(html)
    }

    pub fn procesar_registro(request: &Request, conn: &Connection) -> Response {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }
    
        // deberia llegar algo asi: "nombre=Juan&apellido=Lopez&email=jlo..."
        let datos_parseados = Self::parsear_formulario_manual(&body);

        // revisar si se puede extraer los campos asi
        let nombre = datos_parseados.get("nombre").cloned().unwrap_or_default();
        let apellido = datos_parseados.get("apellido").cloned().unwrap_or_default();
        let email = datos_parseados.get("email").cloned().unwrap_or_default();
        let tipo = datos_parseados.get("tipo").cloned().unwrap_or_default();
        let password = datos_parseados.get("password").cloned().unwrap_or_default();
        let legajo = match datos_parseados.get("legajo").unwrap_or(&String::new()).parse::<i32>() {
            Ok(val) => val,
            Err(_) => return Response::html("<div style='color:red;'>Legajo inválido</div>"),
        };
    
        match AuthService::registrar_cuenta(conn, legajo, nombre, apellido, email, &tipo, &password) {
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

    fn parsear_formulario_manual(cuerpo: &str) -> HashMap<String, String> {
        let mut mapa = HashMap::new();
        for par in cuerpo.split('&') {
            let mut partes = par.split('=');
            if let (Some(clave), Some(valor)) = (partes.next(), partes.next()) {
                let valor_decodificado = valor
                    .replace("%40", "@")
                    .replace("+", " ");
                mapa.insert(clave.to_string(), valor_decodificado);
            }
        }
        mapa
    }
}


