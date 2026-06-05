use crate::service::modelo_service::ModeloService;
use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;
pub struct ModeloHandler;

impl ModeloHandler {
    pub fn mostrar_formulario_registro() -> Response {
        let html = include_str!("../../templates/modelo_registro.html");
        Response::html(html)
    }

    pub fn procesar_registro(request: &Request, conn: &Connection) -> Response {
        /*
        let email = match request.header("X-Usuario-Email") {
            Some(e) => e,
            None => {
                return Response::text("Falta el header X-Usuario-Email").with_status_code(400)
            }
        };

        let usuario: Usuario = match UsuarioRepository::buscar_por_email(conn, email) {
            Ok(Some(u)) => u,
            Ok(None) => return Response::text("Usuario no encontrado").with_status_code(401),
            Err(e) => {
                return Response::text(format!("Error consultando usuarios: {}", e))
                    .with_status_code(500)
            }
        };

        if !usuario.es_admin() {
            return Response::text("El usuario no tiene permisos de administrador")
                .with_status_code(403);
        }
        */

        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);

        let marca = datos_parseados.get("marca").cloned();
        let modelo = datos_parseados.get("modelo").cloned().unwrap_or_default();
        let categoria = datos_parseados.get("categoria").cloned();
        let descripcion = datos_parseados.get("descripcion").cloned();
        let manual_url = datos_parseados.get("manual_url").cloned();
        let direccion_imagen_principal = datos_parseados.get("direccion_imagen_principal").cloned();

        match ModeloService::crear_modelo(
            conn,
            marca,
            modelo,
            categoria,
            descripcion,
            manual_url,
            direccion_imagen_principal,
        ) {
            Ok(modelo) => {
                let exito_html = format!(
                    "<div style='color:green;'>Modelo {} creado!</div>",
                    modelo.modelo
                );
                Response::html(exito_html)
            }
            Err(e) => {
                let error_html = format!("<div style='color:red;'>Error: {}</div>", e);
                Response::html(error_html)
            }
        }
    }

    fn parsear_formulario(cuerpo: &str) -> HashMap<String, String> {
        // llega asi: "marca=Yamaha&nombre_modelo=P-45&categoria=Microscopio..."
        let mut mapa = HashMap::new();
        for par in cuerpo.split('&') {
            let mut partes = par.split('=');
            if let (Some(clave), Some(valor)) = (partes.next(), partes.next()) {
                let valor_decodificado = valor.replace("%40", "@").replace("+", " ");
                mapa.insert(clave.to_string(), valor_decodificado);
            }
        }
        mapa
    }
}
