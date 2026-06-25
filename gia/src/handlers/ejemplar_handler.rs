use crate::repository::ejemplar_repository::EjemplarRepository;
use crate::repository::modelo_repository::ModeloRepository;
use crate::repository::reserva_repository::ReservaRepository;
use crate::service::ejemplar_service::{CrearEjemplarData, EjemplarService};
use crate::templates;
use crate::utils::usuario_actual;

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
    pub fn mostrar_formulario_registro(request: &Request, conn: &Connection) -> Response {
        let usuario = match usuario_actual(request, conn) {
            Ok(u) => u,
            Err(response) => return response,
        };

        if !usuario.es_admin() {
            return templates::response_mensaje_error_con_status(
                "Acceso denegado",
                "Esta acción requiere permisos de administrador.",
                403,
            );
        }

        let ctx = Context::new();
        templates::response_html(templates::render("ejemplar_registro.html", &ctx))
    }

    pub fn mostrar_formulario_edicion(request: &Request, conn: &Connection, id: i64) -> Response {
        let usuario = match usuario_actual(request, conn) {
            Ok(u) => u,
            Err(response) => return response,
        };

        if !usuario.es_admin() {
            return templates::response_mensaje_error_con_status(
                "Acceso denegado",
                "Esta acción requiere permisos de administrador.",
                403,
            );
        }

        let ejemplar = match EjemplarRepository::buscar_por_id(conn, id) {
            Ok(Some(e)) => e,
            Ok(None) => {
                return templates::response_mensaje_error_con_status(
                    "Ejemplar no encontrado",
                    "El ejemplar solicitado no existe.",
                    404,
                );
            }
            Err(e) => {
                return templates::response_mensaje_error_con_status(
                    "Error interno",
                    &format!("No se pudo cargar el ejemplar: {}", e),
                    500,
                );
            }
        };

        let bloqueado =
            ReservaRepository::tiene_reserva_activa_o_pendiente(conn, id).unwrap_or(false);

        let modelos = match Self::cargar_opciones_modelos(conn) {
            Ok(opciones) => opciones,
            Err(mensaje) => {
                return templates::response_mensaje_error(
                    "No se pudieron cargar los modelos",
                    &mensaje,
                );
            }
        };

        let mut ctx = Context::new();
        ctx.insert("ejemplar", &ejemplar);
        ctx.insert("modelos", &modelos);
        ctx.insert("bloqueado", &bloqueado);
        ctx.insert(
            "mensaje_bloqueo",
            "Este ejemplar tiene una reserva pendiente o activa y no puede modificarse.",
        );
        templates::response_html(templates::render("ejemplar_editar.html", &ctx))
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
                ctx.insert(
                    "mensaje",
                    &format!("No se pudieron cargar los modelos: {}", mensaje),
                );
                templates::response_html(templates::render(
                    "partials/modelo_select_error.html",
                    &ctx,
                ))
                .with_status_code(500)
            }
        }
    }

    fn cargar_opciones_modelos(conn: &Connection) -> Result<Vec<ModeloOption>, String> {
        let modelos = ModeloRepository::listar_todos(conn).map_err(|e| format!("{}", e))?;

        Ok(modelos
            .into_iter()
            .map(|m| ModeloOption {
                id: m.id,
                nombre_modelo: m.nombre_modelo,
            })
            .collect())
    }

    pub fn procesar_registro(request: &Request, conn: &Connection) -> Response {
        if let Err(response) = Self::verificar_admin(request, conn) {
            return response;
        }

        let datos_parseados = match Self::leer_formulario(request) {
            Ok(d) => d,
            Err(response) => return response,
        };

        let data = match Self::datos_desde_formulario(&datos_parseados) {
            Ok(d) => d,
            Err(response) => return response,
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

    pub fn procesar_edicion(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(response) = Self::verificar_admin(request, conn) {
            return response;
        }

        let datos_parseados = match Self::leer_formulario(request) {
            Ok(d) => d,
            Err(response) => return response,
        };

        let data = match Self::datos_desde_formulario(&datos_parseados) {
            Ok(d) => d,
            Err(response) => return response,
        };

        match EjemplarService::actualizar_ejemplar(conn, id, data) {
            Ok(ejemplar) => templates::response_mensaje_exito(
                "Ejemplar actualizado",
                &format!(
                    "El ejemplar fue actualizado correctamente (modelo ID: {}).",
                    ejemplar.modelo_id
                ),
            ),
            Err(e) => templates::response_mensaje_error("No se pudo actualizar el ejemplar", &e),
        }
    }

    fn verificar_admin(request: &Request, conn: &Connection) -> Result<(), Response> {
        let usuario = usuario_actual(request, conn)?;
        if !usuario.es_admin() {
            return Err(templates::response_mensaje_error_con_status(
                "Acceso denegado",
                "Esta acción requiere permisos de administrador.",
                403,
            ));
        }
        Ok(())
    }

    fn leer_formulario(request: &Request) -> Result<HashMap<String, String>, Response> {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }
        Ok(Self::parsear_formulario(&body))
    }

    fn datos_desde_formulario(
        datos_parseados: &HashMap<String, String>,
    ) -> Result<CrearEjemplarData, Response> {
        let modelo_id = match datos_parseados
            .get("modelo_id")
            .and_then(|v| v.parse::<i64>().ok())
        {
            Some(id) => id,
            None => {
                return Err(templates::response_mensaje_error_con_status(
                    "Datos inválidos",
                    "Debe seleccionar un modelo válido.",
                    400,
                ));
            }
        };

        let numero_serie = Self::campo_opcional(datos_parseados, "numero_serie");
        let codigo_qr = Self::campo_opcional(datos_parseados, "codigo_qr");
        let patrimonio = Self::campo_opcional(datos_parseados, "patrimonio");
        let observaciones = Self::campo_opcional(datos_parseados, "observaciones");
        let accesorios = match datos_parseados.get("tiene_accesorios").map(|v| v.as_str()) {
            Some("si") => match Self::campo_opcional(datos_parseados, "accesorios") {
                Some(valor) => Some(valor),
                None => {
                    return Err(templates::response_mensaje_error(
                        "Datos inválidos",
                        "Indique los accesorios o seleccione No.",
                    ));
                }
            },
            _ => None,
        };
        let ubicacion = Self::campo_opcional(datos_parseados, "ubicacion");
        let esta_disponible = datos_parseados
            .get("esta_disponible")
            .map(|v| v == "true")
            .unwrap_or(true);

        Ok(CrearEjemplarData {
            modelo_id,
            numero_serie,
            codigo_qr,
            patrimonio,
            observaciones,
            accesorios,
            esta_disponible,
            ubicacion,
        })
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
                    .replace("+", " ")
                    .replace("%20", " ")
                    .replace("%0A", "\n");
                mapa.insert(clave.to_string(), valor_decodificado);
            }
        }
        mapa
    }
}
