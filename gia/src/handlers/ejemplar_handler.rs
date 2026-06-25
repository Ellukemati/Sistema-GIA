use crate::handlers::image_handler::ImageHandler;
use crate::repository::ejemplar_repository::EjemplarRepository;
use crate::repository::image_repository::ImageRepository;
use crate::repository::modelo_repository::ModeloRepository;
use crate::repository::reserva_repository::ReservaRepository;
use crate::service::ejemplar_service::{CrearEjemplarData, EjemplarService};
use crate::templates;
use crate::utils::usuario_actual;

use rouille::input::multipart;
use rouille::{Request, Response};
use rusqlite::Connection;
use serde::Serialize;
use std::io::Read;
use tera::Context;

#[derive(Serialize)]
struct ModeloOption {
    id: i64,
    nombre_modelo: String,
}

struct DatosFormularioEjemplar {
    modelo_id: i64,
    numero_serie: Option<String>,
    codigo_qr: Option<String>,
    patrimonio: Option<String>,
    observaciones: Option<String>,
    accesorios: Option<String>,
    esta_disponible: bool,
    ubicacion: Option<String>,
    lista_imagenes_bytes: Vec<Vec<u8>>,
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

        let imagen = match ImageRepository::existe_imagen_principal_ejemplar(conn, id) {
            Ok(true) => Some(format!("/imagenes/ejemplares/{}/0", id)),
            _ => None,
        };

        let mut ctx = Context::new();
        ctx.insert("ejemplar", &ejemplar);
        ctx.insert("modelos", &modelos);
        ctx.insert("bloqueado", &bloqueado);
        ctx.insert("imagen", &imagen);
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

        let datos = match Self::parsear_formulario_ejemplar(request) {
            Ok(d) => d,
            Err(response) => return response,
        };

        let data = match Self::crear_ejemplar_data_desde_formulario(&datos) {
            Ok(d) => d,
            Err(response) => return response,
        };

        let ejemplar = match EjemplarService::crear_ejemplar(conn, data) {
            Ok(e) => e,
            Err(e) => return templates::response_mensaje_error("No se pudo crear el ejemplar", &e),
        };

        let (error_reemplazo, errores_imagenes) =
            Self::guardar_imagenes_ejemplar(conn, ejemplar.id, &datos.lista_imagenes_bytes, false);

        if let Some((titulo, mensaje)) = error_reemplazo {
            return templates::response_mensaje_error(&titulo, &mensaje);
        }

        if errores_imagenes > 0 {
            templates::response_mensaje_exito(
                "Ejemplar creado",
                &format!(
                    "El ejemplar fue registrado, pero {} imágenes fallaron al procesarse.",
                    errores_imagenes
                ),
            )
        } else if datos.lista_imagenes_bytes.is_empty() {
            templates::response_mensaje_exito(
                "Ejemplar creado",
                "El ejemplar fue registrado correctamente.",
            )
        } else {
            templates::response_mensaje_exito(
                "Ejemplar creado con éxito",
                &format!(
                    "El ejemplar y sus {} imágenes se subieron correctamente.",
                    datos.lista_imagenes_bytes.len()
                ),
            )
        }
    }

    pub fn procesar_edicion(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(response) = Self::verificar_admin(request, conn) {
            return response;
        }

        let datos = match Self::parsear_formulario_ejemplar(request) {
            Ok(d) => d,
            Err(response) => return response,
        };

        let data = match Self::crear_ejemplar_data_desde_formulario(&datos) {
            Ok(d) => d,
            Err(response) => return response,
        };

        let ejemplar = match EjemplarService::actualizar_ejemplar(conn, id, data) {
            Ok(e) => e,
            Err(e) => {
                return templates::response_mensaje_error("No se pudo actualizar el ejemplar", &e);
            }
        };

        let reemplazar_imagenes = !datos.lista_imagenes_bytes.is_empty();
        let (error_reemplazo, errores_imagenes) = Self::guardar_imagenes_ejemplar(
            conn,
            ejemplar.id,
            &datos.lista_imagenes_bytes,
            reemplazar_imagenes,
        );

        if let Some((titulo, mensaje)) = error_reemplazo {
            return templates::response_mensaje_error(&titulo, &mensaje);
        }

        if errores_imagenes > 0 {
            templates::response_mensaje_exito(
                "Ejemplar actualizado con advertencias",
                &format!(
                    "El ejemplar fue actualizado, pero {} imágenes fallaron al procesarse.",
                    errores_imagenes
                ),
            )
        } else {
            templates::response_mensaje_exito(
                "Ejemplar actualizado",
                "El ejemplar fue actualizado correctamente.",
            )
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

    fn parsear_formulario_ejemplar(request: &Request) -> Result<DatosFormularioEjemplar, Response> {
        let mut multipart = match multipart::get_multipart_input(request) {
            Ok(m) => m,
            Err(_) => {
                return Err(templates::response_mensaje_error_con_status(
                    "Error de solicitud",
                    "El formulario no tiene el formato multipart correcto.",
                    400,
                ));
            }
        };

        let mut modelo_id_raw = String::new();
        let mut numero_serie: Option<String> = None;
        let mut codigo_qr: Option<String> = None;
        let mut patrimonio: Option<String> = None;
        let mut observaciones: Option<String> = None;
        let mut tiene_accesorios = String::new();
        let mut accesorios: Option<String> = None;
        let mut esta_disponible = String::new();
        let mut ubicacion: Option<String> = None;
        let mut lista_imagenes_bytes: Vec<Vec<u8>> = Vec::new();

        while let Some(mut field) = multipart.next() {
            let name = field.headers.name.to_string();

            match name.as_str() {
                "modelo_id" => {
                    if field.is_text() {
                        let _ = field.data.read_to_string(&mut modelo_id_raw);
                    }
                }
                "numero_serie" => {
                    if field.is_text() {
                        let mut valor = String::new();
                        let _ = field.data.read_to_string(&mut valor);
                        numero_serie = Self::campo_opcional_texto(valor);
                    }
                }
                "codigo_qr" => {
                    if field.is_text() {
                        let mut valor = String::new();
                        let _ = field.data.read_to_string(&mut valor);
                        codigo_qr = Self::campo_opcional_texto(valor);
                    }
                }
                "patrimonio" => {
                    if field.is_text() {
                        let mut valor = String::new();
                        let _ = field.data.read_to_string(&mut valor);
                        patrimonio = Self::campo_opcional_texto(valor);
                    }
                }
                "observaciones" => {
                    if field.is_text() {
                        let mut valor = String::new();
                        let _ = field.data.read_to_string(&mut valor);
                        observaciones = Self::campo_opcional_texto(valor);
                    }
                }
                "tiene_accesorios" => {
                    if field.is_text() {
                        let _ = field.data.read_to_string(&mut tiene_accesorios);
                    }
                }
                "accesorios" => {
                    if field.is_text() {
                        let mut valor = String::new();
                        let _ = field.data.read_to_string(&mut valor);
                        accesorios = Self::campo_opcional_texto(valor);
                    }
                }
                "esta_disponible" => {
                    if field.is_text() {
                        esta_disponible.clear();
                        let _ = field.data.read_to_string(&mut esta_disponible);
                    }
                }
                "ubicacion" => {
                    if field.is_text() {
                        let mut valor = String::new();
                        let _ = field.data.read_to_string(&mut valor);
                        ubicacion = Self::campo_opcional_texto(valor);
                    }
                }
                "imagenes[]" if field.headers.filename.is_some() => {
                    let mut foto_bytes = Vec::new();
                    if field.data.read_to_end(&mut foto_bytes).is_ok() && !foto_bytes.is_empty() {
                        lista_imagenes_bytes.push(foto_bytes);
                    }
                }
                _ => {}
            }
        }

        let modelo_id = match modelo_id_raw.parse::<i64>() {
            Ok(id) => id,
            Err(_) => {
                return Err(templates::response_mensaje_error_con_status(
                    "Datos inválidos",
                    "Debe seleccionar un modelo válido.",
                    400,
                ));
            }
        };

        let accesorios = match tiene_accesorios.as_str() {
            "si" => match accesorios {
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

        Ok(DatosFormularioEjemplar {
            modelo_id,
            numero_serie,
            codigo_qr,
            patrimonio,
            observaciones,
            accesorios,
            esta_disponible: parsear_esta_disponible(&esta_disponible),
            ubicacion,
            lista_imagenes_bytes,
        })
    }

    fn crear_ejemplar_data_desde_formulario(
        datos: &DatosFormularioEjemplar,
    ) -> Result<CrearEjemplarData, Response> {
        Ok(CrearEjemplarData {
            modelo_id: datos.modelo_id,
            numero_serie: datos.numero_serie.clone(),
            codigo_qr: datos.codigo_qr.clone(),
            patrimonio: datos.patrimonio.clone(),
            observaciones: datos.observaciones.clone(),
            accesorios: datos.accesorios.clone(),
            esta_disponible: datos.esta_disponible,
            ubicacion: datos.ubicacion.clone(),
        })
    }

    fn guardar_imagenes_ejemplar(
        conn: &Connection,
        ejemplar_id: i64,
        lista_imagenes_bytes: &[Vec<u8>],
        reemplazar_imagenes: bool,
    ) -> (Option<(String, String)>, usize) {
        let mut errores_imagenes = 0;

        if lista_imagenes_bytes.is_empty() {
            return (None, errores_imagenes);
        }

        if reemplazar_imagenes
            && let Err(e) = ImageRepository::eliminar_por_ejemplar(conn, ejemplar_id)
        {
            return (
                Some((
                    "Error al reemplazar imágenes".to_string(),
                    format!(
                        "Los datos del ejemplar se guardaron pero no se pudieron eliminar las imágenes previas: {}",
                        e
                    ),
                )),
                errores_imagenes,
            );
        }

        for (index, foto_bytes) in lista_imagenes_bytes.iter().enumerate() {
            let orden_imagen = index as i32;
            if ImageHandler::guardar_imagen_ejemplar_bytes(
                conn,
                ejemplar_id,
                orden_imagen,
                foto_bytes,
            )
            .is_err()
            {
                errores_imagenes += 1;
            }
        }

        (None, errores_imagenes)
    }

    /// Devuelve None si el campo llegó vacío, para preservar los NULL
    /// y no romper las restricciones UNIQUE de la tabla ejemplares.
    fn campo_opcional_texto(valor: String) -> Option<String> {
        if valor.trim().is_empty() {
            None
        } else {
            Some(valor)
        }
    }
}

fn parsear_esta_disponible(valor: &str) -> bool {
    match valor.trim() {
        "" | "true" => true,
        "false" => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::parsear_esta_disponible;

    #[test]
    fn disponibilidad_true_cuando_el_formulario_envia_true() {
        assert!(parsear_esta_disponible("true"));
    }

    #[test]
    fn disponibilidad_true_por_defecto_si_el_campo_viene_vacio() {
        assert!(parsear_esta_disponible(""));
    }

    #[test]
    fn disponibilidad_false_cuando_el_formulario_envia_false() {
        assert!(!parsear_esta_disponible("false"));
    }
}
