use crate::models::usuario::Usuario;
use crate::repository::modelo_instrumento_repository::ModeloInstrumentoRepository;
use crate::repository::usuario_repository::UsuarioRepository;
use crate::service::ejemplar_service::EjemplarService;
use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;

pub struct EjemplarHandler;

impl EjemplarHandler {
    pub fn mostrar_formulario_registro(conn: &Connection) -> Response {
        let modelos = match ModeloInstrumentoRepository::listar_todos(conn) {
            Ok(m) => m,
            Err(e) => {
                return Response::text(format!("Error al listar modelos: {}", e))
                    .with_status_code(500)
            }
        };

        let mut opciones = String::new();
        for modelo in modelos {
            opciones.push_str(&format!(
                "<option value=\"{}\">{}</option>",
                modelo.id, modelo.nombre_modelo
            ));
        }

        let html = include_str!("../../templates/ejemplar_registro.html");
        let html = html.replace("{{opciones_modelos}}", &opciones);
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

        let modelo_id = match datos_parseados
            .get("modelo_id")
            .and_then(|v| v.parse::<i64>().ok())
        {
            Some(id) => id,
            None => {
                return Response::text("Debe seleccionar un modelo valido").with_status_code(400)
            }
        };

        let numero_serie = Self::campo_opcional(&datos_parseados, "numero_serie");
        let codigo_qr = Self::campo_opcional(&datos_parseados, "codigo_qr");
        let patrimonio = Self::campo_opcional(&datos_parseados, "patrimonio");
        let observaciones = Self::campo_opcional(&datos_parseados, "observaciones");
        let ubicacion = Self::campo_opcional(&datos_parseados, "ubicacion");
        let esta_disponible = datos_parseados
            .get("esta_disponible")
            .map(|v| v == "true")
            .unwrap_or(true);

        match EjemplarService::crear_ejemplar(
            conn,
            modelo_id,
            numero_serie,
            codigo_qr,
            patrimonio,
            observaciones,
            esta_disponible,
            ubicacion,
        ) {
            Ok(ejemplar) => {
                let exito_html = format!(
                    "<div style='color:green;'>Ejemplar creado para el modelo {}!</div>",
                    ejemplar.modelo_id
                );
                Response::html(exito_html)
            }
            Err(e) => {
                let error_html = format!("<div style='color:red;'>Error: {}</div>", e);
                Response::html(error_html)
            }
        }
    }

    /// Devuelve None si el campo no fue enviado o llego vacio, para preservar los NULL
    /// y no romper las restricciones UNIQUE de la tabla ejemplares.
    fn campo_opcional(datos: &HashMap<String, String>, clave: &str) -> Option<String> {
        match datos.get(clave) {
            Some(valor) if !valor.trim().is_empty() => Some(valor.clone()),
            _ => None,
        }
    }

    fn parsear_formulario(body: &str) -> HashMap<String, String> {
        let mut mapa = HashMap::new();
        for par in body.split('&') {
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
