use crate::repository::modelo_repository::ModeloRepository;
use crate::service::ejemplar_service::{CrearEjemplarData, EjemplarService};
use crate::templates;
use rouille::{Request, Response};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Read;
use tera::Context;

#[derive(Serialize)]
struct ModeloOption {
    id: i64,
    nombre_modelo: String,
}

pub struct EjemplarHandler;

impl EjemplarHandler {
    pub fn mostrar_formulario_registro() -> Response {
        let ctx = Context::new();
        templates::response_html(templates::render("ejemplar_registro.html", &ctx))
    }

    pub fn listar_opciones_modelos(conn: &Connection) -> Response {
        match Self::cargar_opciones_modelos(conn) {
            Ok(opciones) => {
                let mut ctx = Context::new();
                ctx.insert("modelos", &opciones);
                templates::response_html(templates::render("partials/modelo_select.html", &ctx))
            }
            Err(mensaje) => {
                let mut ctx = Context::new();
                ctx.insert("mensaje", &format!("No se pudieron cargar los modelos: {}", mensaje));
                templates::response_html(templates::render("partials/modelo_select_error.html", &ctx))
                    .with_status_code(500)
            }
        }
    }

    fn cargar_opciones_modelos(conn: &Connection) -> Result<Vec<ModeloOption>, String> {
        let modelos = ModeloRepository::listar_todos(conn)
            .map_err(|e| format!("{}", e))?;

        Ok(modelos
            .into_iter()
            .map(|m| ModeloOption {
                id: m.id,
                nombre_modelo: m.nombre_modelo,
            })
            .collect())
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
                return templates::response_mensaje_error_con_status(
                    "Datos inválidos",
                    "Debe seleccionar un modelo válido.",
                    400,
                );
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

        let data = CrearEjemplarData {
            modelo_id,
            numero_serie,
            codigo_qr,
            patrimonio,
            observaciones,
            esta_disponible,
            ubicacion,
        };

        match EjemplarService::crear_ejemplar(conn, data) {
            Ok(ejemplar) => templates::response_mensaje_exito(
                "Ejemplar creado",
                &format!(
                    "El ejemplar fue registrado correctamente (modelo ID: {}).",
                    ejemplar.modelo_id
                ),
            ),
            Err(e) => templates::response_mensaje_error("No se pudo crear el ejemplar", &e),
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
                let valor_decodificado = valor.replace("%40", "@").replace("+", " ");
                mapa.insert(clave.to_string(), valor_decodificado);
            }
        }
        mapa
    }
}
