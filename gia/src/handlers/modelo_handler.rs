use crate::errors::ManualStorageError;
use crate::repository::image_repository::ImageRepository;
use crate::repository::modelo_repository::ModeloRepository;
use crate::repository::sesion_repository::SesionRepository;
use crate::repository::usuario_repository::UsuarioRepository;
use crate::service::ejemplar_service::EjemplarService;
use crate::service::image_service::procesar_modelo;
use crate::service::manual_service::validar_y_procesar_manual;
use crate::service::modelo_service::{CrearModeloData, ModeloService};
use crate::templates;
use crate::utils::extraer_token_sesion;

use rouille::input::multipart;
use rouille::{Request, Response};
use rusqlite::Connection;
use std::io::Read;
use tera::Context;

pub struct ModeloHandler;

impl ModeloHandler {
    pub fn mostrar_formulario_registro() -> Response {
        let ctx = Context::new();
        templates::response_html(templates::render("modelo_registro.html", &ctx))
    }

    pub fn procesar_registro(request: &Request, conn: &Connection) -> Response {
        // Extraer token de la cookie
        let token = match extraer_token_sesion(request) {
            Some(t) => t,
            None => {
                return templates::response_mensaje_error_con_status(
                    "No autorizado",
                    "Debe iniciar sesión.",
                    401,
                );
            }
        };

        // Buscar la sesión en la base de datos
        let sesion = match SesionRepository::buscar_por_token(conn, &token) {
            Ok(Some(s)) => s,
            _ => {
                return templates::response_mensaje_error_con_status(
                    "Sesión inválida",
                    "Su sesión expiró. Volvé a iniciar sesión.",
                    401,
                );
            }
        };

        // Buscar al usuario dueño de la sesión
        let usuario = match UsuarioRepository::buscar_por_id(conn, sesion.id_usuario) {
            Ok(Some(u)) => u,
            _ => {
                return templates::response_mensaje_error_con_status(
                    "Error interno",
                    "No se pudo cargar el usuario.",
                    500,
                );
            }
        };

        // Verificar permisos (Solo administradores)
        if !usuario.es_admin() {
            return templates::response_mensaje_error_con_status(
                "Acceso denegado",
                "Esta acción requiere permisos de administrador.",
                403,
            );
        }

        let mut multipart = match multipart::get_multipart_input(request) {
            Ok(m) => m,
            Err(_) => {
                return templates::response_mensaje_error_con_status(
                    "Error de solicitud",
                    "El formulario no tiene el formato multipart correcto.",
                    400,
                );
            }
        };

        let mut marca = String::new();
        let mut nombre_modelo = String::new();
        let mut categoria: Option<String> = None;
        let mut descripcion: Option<String> = None;
        let mut manual_bytes: Vec<u8> = Vec::new();
        let mut lista_imagenes_bytes: Vec<Vec<u8>> = Vec::new();

        while let Some(mut field) = multipart.next() {
            let name = field.headers.name.to_string();

            match name.as_str() {
                "marca" => {
                    if field.is_text() {
                        let _ = field.data.read_to_string(&mut marca);
                    }
                }
                "nombre_modelo" => {
                    if field.is_text() {
                        let _ = field.data.read_to_string(&mut nombre_modelo);
                    }
                }
                "categoria" => {
                    if field.is_text() {
                        let mut cat = String::new();
                        let _ = field.data.read_to_string(&mut cat);
                        if !cat.trim().is_empty() {
                            categoria = Some(cat);
                        }
                    }
                }
                "descripcion" => {
                    if field.is_text() {
                        let mut desc = String::new();
                        let _ = field.data.read_to_string(&mut desc);
                        if !desc.trim().is_empty() {
                            descripcion = Some(desc);
                        }
                    }
                }
                "manual_pdf" if field.headers.filename.is_some() => {
                    let _ = field.data.read_to_end(&mut manual_bytes);
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

        let data = CrearModeloData {
            marca,
            nombre_modelo,
            categoria,
            descripcion,
        };

        let modelo = match ModeloService::crear_modelo(conn, data) {
            Ok(m) => m,
            Err(e) => return templates::response_mensaje_error("No se pudo crear el modelo", &e),
        };

        if !manual_bytes.is_empty() {
            match validar_y_procesar_manual(&manual_bytes) {
                Ok((pdf_data, mime_type)) => {
                    if let Err(e) =
                        ModeloRepository::actualizar_manual(conn, modelo.id, &pdf_data, &mime_type)
                    {
                        return templates::response_mensaje_error(
                            "Modelo creado con advertencia",
                            &format!("El modelo fue creado pero falló guardar el manual: {}", e),
                        );
                    }
                }
                Err(ManualStorageError::InvalidManual(msg)) => {
                    return templates::response_mensaje_error(
                        "Manual rechazado",
                        &format!("Modelo creado, pero el manual no se guardó: {}", msg),
                    );
                }
                Err(e) => {
                    return templates::response_mensaje_error(
                        "Error de almacenamiento de manual",
                        &e.to_string(),
                    );
                }
            }
        }

        let mut errores_imagenes = 0;
        for (index, foto_bytes) in lista_imagenes_bytes.iter().enumerate() {
            let orden_imagen = index as i32; // La primera 0 será la principal

            match procesar_modelo(foto_bytes) {
                Ok((blob_final, mime)) => {
                    if ImageRepository::guardar_modelo(
                        conn,
                        modelo.id,
                        orden_imagen,
                        &blob_final,
                        &mime,
                    )
                    .is_err()
                    {
                        errores_imagenes += 1;
                    }
                }
                Err(_) => {
                    errores_imagenes += 1;
                }
            }
        }

        if errores_imagenes > 0 {
            templates::response_mensaje_exito(
                "Modelo registrado",
                &format!(
                    "El modelo \"{}\" fue registrado, pero {} imágenes fallaron al procesarse.",
                    modelo.nombre_modelo, errores_imagenes
                ),
            )
        } else {
            templates::response_mensaje_exito(
                "Modelo registrado con éxito",
                &format!(
                    "El modelo \"{}\" y sus {} imágenes se subieron correctamente.",
                    modelo.nombre_modelo,
                    lista_imagenes_bytes.len()
                ),
            )
        }
    }

    pub fn listar_modelos(conn: &Connection) -> Response {
        match ModeloService::listar_cards_agrupadas(conn) {
            Ok(grupos) => {
                let mut ctx = Context::new();
                ctx.insert("grupos", &grupos);
                templates::response_html(templates::render("modelo_listado.html", &ctx))
            }
            Err(e) => templates::response_mensaje_error("No se pudieron cargar los modelos", &e),
        }
    }

    pub fn mostrar_detalle(conn: &Connection, id: i64) -> Response {
        let modelo = match ModeloRepository::buscar_por_id(conn, id) {
            Ok(Some(m)) => m,
            Ok(None) => {
                return templates::response_mensaje_error_con_status(
                    "Modelo no encontrado",
                    "El modelo solicitado no existe.",
                    404,
                );
            }
            Err(e) => {
                return templates::response_mensaje_error_con_status(
                    "Error interno",
                    &format!("No se pudo cargar el modelo: {}", e),
                    500,
                );
            }
        };

        let imagen = match ImageRepository::existe_imagen_principal_modelo(conn, id) {
            Ok(true) => Some(format!("/imagenes/modelos/{}/0", id)),
            _ => None,
        };

        let tiene_manual = ModeloRepository::tiene_manual(conn, id).unwrap_or(false);
        let ejemplares = EjemplarService::listar_ejemplares_basico(conn, id);

        let ejemplares = match ejemplares {
            Ok(e) => e,
            Err(e) => {
                return templates::response_mensaje_error(
                    "No se pudieron cargar los ejemplares",
                    &e,
                );
            }
        };

        let mut ctx = Context::new();
        ctx.insert("modelo", &modelo);
        ctx.insert("imagen", &imagen);
        ctx.insert("tiene_manual", &tiene_manual);
        ctx.insert("ejemplares", &ejemplares);
        ctx.insert("con_fechas", &false);
        templates::response_html(templates::render("modelo_detalle.html", &ctx))
    }
}
