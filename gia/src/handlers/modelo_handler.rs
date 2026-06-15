use crate::repository::sesion_repository::SesionRepository;
use crate::repository::usuario_repository::UsuarioRepository;
use crate::service::modelo_service::{CrearModeloData, ModeloService};
use crate::utils::extraer_token_sesion;
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
        // Extraer token de la cookie
        let token = match extraer_token_sesion(request) {
            Some(t) => t,
            None => {
                return Response::html("<div style='color:red;'>No autorizado.</div>")
                    .with_status_code(401);
            }
        };

        // Buscar la sesión en la base de datos
        let sesion = match SesionRepository::buscar_por_token(conn, &token) {
            Ok(Some(s)) => s,
            _ => {
                return Response::html("<div style='color:red;'>Sesión inválida.</div>")
                    .with_status_code(401);
            }
        };

        // Buscar al usuario dueño de la sesión
        let usuario = match UsuarioRepository::buscar_por_id(conn, sesion.usuario_id) {
            Ok(Some(u)) => u,
            _ => {
                return Response::html("<div style='color:red;'>Error de usuario.</div>")
                    .with_status_code(500);
            }
        };

        // Verificar permisos (Solo administradores)
        if !usuario.es_admin() {
            return Response::html(
                "<div style='color:red;'>Acceso denegado: Requiere rol Admin.</div>",
            )
            .with_status_code(403);
        }

        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);

        let marca = datos_parseados.get("marca").cloned().unwrap_or_default();
        let nombre_modelo = datos_parseados
            .get("nombre_modelo")
            .cloned()
            .unwrap_or_default();
        let categoria = Self::campo_opcional(&datos_parseados, "categoria");
        let descripcion = Self::campo_opcional(&datos_parseados, "descripcion");

        let data = CrearModeloData {
            marca,
            nombre_modelo,
            categoria,
            descripcion,
        };

        match ModeloService::crear_modelo(conn, data) {
            Ok(modelo) => {
                let exito_html = format!(
                    "<div style='color:green;'>Modelo N° {} ({}) creado exitosamente!</div>",
                    modelo.id, modelo.nombre_modelo
                );
                Response::html(exito_html)
            }
            Err(e) => {
                let error_html = format!("<div style='color:red;'>Error: {}</div>", e);
                Response::html(error_html)
            }
        }
    }

    fn campo_opcional(datos: &HashMap<String, String>, clave: &str) -> Option<String> {
        match datos.get(clave) {
            Some(valor) if !valor.trim().is_empty() => Some(valor.clone()),
            _ => None,
        }
    }

    fn parsear_formulario(cuerpo: &str) -> HashMap<String, String> {
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
