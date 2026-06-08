use crate::repository::sesion_repository::SesionRepository;
use crate::repository::usuario_repository::UsuarioRepository;
use crate::service::modelo_service::ModeloService;
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
                return Response::html(
                    "<div style='color:red;'>No autorizado. Debe iniciar sesión.</div>",
                )
                .with_status_code(401);
            }
        };

        // Buscar la sesión en la base de datos
        let sesion =
            match SesionRepository::buscar_por_token(conn, &token) {
                Ok(Some(s)) => s,
                _ => return Response::html(
                    "<div style='color:red;'>Sesión inválida o expirada. Vuelva a loguearse.</div>",
                )
                .with_status_code(401),
            };

        // Buscar al usuario dueño de la sesión
        let usuario = match UsuarioRepository::buscar_por_id(conn, sesion.usuario_id) {
            Ok(Some(u)) => u,
            _ => {
                return Response::html(
                    "<div style='color:red;'>Error interno cargando el usuario.</div>",
                )
                .with_status_code(500);
            }
        };

        // Verificar permisos (Solo administradores)
        if !usuario.es_admin() {
            return Response::html("<div style='color:red;'>Acceso denegado: Esta acción requiere permisos de Administrador.</div>").with_status_code(403);
        }

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
