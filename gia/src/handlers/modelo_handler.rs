use crate::errors::ManualStorageError;
use crate::repository::ejemplar_repository::EjemplarRepository;
use crate::repository::image_repository::ImageRepository;
use crate::repository::modelo_repository::ModeloRepository;
use crate::service::ejemplar_service::EjemplarService;
use crate::service::image_service::procesar_modelo;
use crate::service::manual_service::validar_y_procesar_manual;
use crate::service::modelo_service::{CrearModeloData, ModeloService};
use crate::templates;
use crate::utils::usuario_actual;

use rouille::input::multipart;
use rouille::{Request, Response};
use rusqlite::Connection;
use std::io::Read;
use tera::Context;

pub struct ModeloHandler;

struct DatosFormularioModelo {
    marca: String,
    nombre_modelo: String,
    categoria: Option<String>,
    descripcion: Option<String>,
    manual_bytes: Vec<u8>,
    lista_imagenes_bytes: Vec<Vec<u8>>,
}

impl ModeloHandler {
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
        templates::response_html(templates::render("modelo_registro.html", &ctx))
    }

    pub fn procesar_registro(request: &Request, conn: &Connection) -> Response {
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

        let datos = match Self::parsear_formulario_modelo(request) {
            Ok(d) => d,
            Err(response) => return response,
        };

        let data = CrearModeloData {
            marca: datos.marca,
            nombre_modelo: datos.nombre_modelo,
            categoria: datos.categoria,
            descripcion: datos.descripcion,
        };

        let modelo = match ModeloService::crear_modelo(conn, data) {
            Ok(m) => m,
            Err(e) => return templates::response_mensaje_error("No se pudo crear el modelo", &e),
        };

        let (errores_manual, errores_imagenes) = Self::guardar_assets_modelo(
            conn,
            modelo.id,
            &datos.manual_bytes,
            &datos.lista_imagenes_bytes,
            false,
        );

        if let Some((titulo, mensaje)) = errores_manual {
            return templates::response_mensaje_error(&titulo, &mensaje);
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
                    datos.lista_imagenes_bytes.len()
                ),
            )
        }
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

        let modelo = match ModeloRepository::buscar_por_id(conn, id) {
            Ok(Some(m)) if !m.eliminado => m,
            Ok(Some(_)) | Ok(None) => {
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

        let mut ctx = Context::new();
        ctx.insert("modelo", &modelo);
        ctx.insert("imagen", &imagen);
        ctx.insert("tiene_manual", &tiene_manual);
        templates::response_html(templates::render("modelo_editar.html", &ctx))
    }

    pub fn procesar_edicion(request: &Request, conn: &Connection, id: i64) -> Response {
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

        let datos = match Self::parsear_formulario_modelo(request) {
            Ok(d) => d,
            Err(response) => return response,
        };

        let data = CrearModeloData {
            marca: datos.marca,
            nombre_modelo: datos.nombre_modelo.clone(),
            categoria: datos.categoria.clone(),
            descripcion: datos.descripcion.clone(),
        };

        let modelo = match ModeloService::actualizar_modelo(conn, id, data) {
            Ok(m) => m,
            Err(e) => {
                return templates::response_mensaje_error("No se pudo actualizar el modelo", &e);
            }
        };

        let reemplazar_imagenes = !datos.lista_imagenes_bytes.is_empty();
        let (errores_manual, errores_imagenes) = Self::guardar_assets_modelo(
            conn,
            modelo.id,
            &datos.manual_bytes,
            &datos.lista_imagenes_bytes,
            reemplazar_imagenes,
        );

        if let Some((titulo, mensaje)) = errores_manual {
            return templates::response_mensaje_error(&titulo, &mensaje);
        }

        let mut advertencias = Vec::new();
        if errores_imagenes > 0 {
            advertencias.push(format!(
                "{} imágenes fallaron al procesarse",
                errores_imagenes
            ));
        }

        if advertencias.is_empty() {
            templates::response_mensaje_exito(
                "Modelo actualizado",
                &format!(
                    "El modelo \"{}\" fue actualizado correctamente.",
                    modelo.nombre_modelo
                ),
            )
        } else {
            templates::response_mensaje_exito(
                "Modelo actualizado con advertencias",
                &format!(
                    "El modelo \"{}\" fue actualizado, pero {}.",
                    modelo.nombre_modelo,
                    advertencias.join("; ")
                ),
            )
        }
    }

    pub fn listar_modelos(request: &Request, conn: &Connection) -> Response {
        let busqueda = request.get_param("buscar").unwrap_or_default();

        let grupos = if busqueda.trim().is_empty() {
            ModeloService::listar_cards_agrupadas(conn)
        } else {
            ModeloService::listar_cards_filtradas(conn, &busqueda)
        };

        match grupos {
            Ok(grupos) => {
                let mut ctx = Context::new();
                ctx.insert("grupos", &grupos);
                ctx.insert("busqueda", &busqueda);

                templates::response_html(templates::render("modelo_listado.html", &ctx))
            }

            Err(e) => templates::response_mensaje_error("No se pudieron cargar los modelos", &e),
        }
    }
    pub fn mostrar_detalle(request: &Request, conn: &Connection, id: i64) -> Response {
        let modelo = match ModeloRepository::buscar_por_id(conn, id) {
            Ok(Some(m)) if !m.eliminado => m,
            Ok(Some(_)) | Ok(None) => return templates::response_mensaje_error_con_status("No encontrado", "El modelo no existe.", 404),
            Err(e) => return templates::response_mensaje_error_con_status("Error", &e.to_string(), 500),
        };
        let es_admin = usuario_actual(request, conn).map(|u| u.es_admin()).unwrap_or(false);
        let ejemplares = match if es_admin { EjemplarService::listar_ejemplares_para_detalle(conn, id) } else { EjemplarService::listar_ejemplares_basico(conn, id) } {
            Ok(e) => e, Err(e) => return templates::response_mensaje_error("Error", &e),
        };

        let ctx = Self::armar_contexto_detalle(conn, modelo, ejemplares, es_admin);
        templates::response_html(templates::render("modelo_detalle.html", &ctx))
    }

    fn armar_contexto_detalle(conn: &Connection, modelo: crate::models::modelo::Modelo, ejemplares: Vec<crate::service::ejemplar_service::EjemplarDTO>, es_admin: bool) -> Context {
        let id = modelo.id;
        let imagen = match ImageRepository::existe_imagen_principal_modelo(conn, id) { Ok(true) => Some(format!("/imagenes/modelos/{}/0", id)), _ => None };
        let cantidad_imagenes = ImageRepository::listar_ordenes_modelo(conn, id).map(|o| o.len()).unwrap_or(0);
        let tiene_manual = ModeloRepository::tiene_manual(conn, id).unwrap_or(false);
        let tiene_ejemplares_activos = if es_admin { EjemplarRepository::tiene_ejemplares_activos(conn, id).unwrap_or(true) } else { false };

        let mut ctx = Context::new();
        ctx.insert("modelo", &modelo); ctx.insert("imagen", &imagen); ctx.insert("cantidad_imagenes", &cantidad_imagenes);
        ctx.insert("tiene_manual", &tiene_manual); ctx.insert("ejemplares", &ejemplares); ctx.insert("con_fechas", &false);
        ctx.insert("es_admin", &es_admin); ctx.insert("mostrar_edicion", &es_admin); ctx.insert("tiene_ejemplares_activos", &tiene_ejemplares_activos);
        ctx
    }

    pub fn procesar_eliminacion(request: &Request, conn: &Connection, id: i64) -> Response {
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

        match ModeloService::eliminar_modelo(conn, id) {
            Ok(()) => {
                let mut ctx = Context::new();
                ctx.insert("titulo", "Modelo eliminado");
                ctx.insert("mensaje", "El modelo fue eliminado correctamente.");
                ctx.insert("exito", &true);
                templates::response_html(templates::render("modelo_eliminado.html", &ctx))
            }
            Err(e) => {
                let mut ctx = Context::new();
                ctx.insert("titulo", "No se pudo eliminar el modelo");
                ctx.insert("mensaje", &e);
                ctx.insert("exito", &false);
                templates::response_html(templates::render("modelo_eliminado.html", &ctx))
            }
        }
    }

    fn parsear_formulario_modelo(request: &Request) -> Result<DatosFormularioModelo, Response> {
        let mut multipart = multipart::get_multipart_input(request).map_err(|_| {
            templates::response_mensaje_error_con_status("Error", "Formato multipart inválido", 400)
        })?;
        
        let mut datos = DatosFormularioModelo {
            marca: String::new(), nombre_modelo: String::new(), categoria: None, 
            descripcion: None, manual_bytes: Vec::new(), lista_imagenes_bytes: Vec::new(),
        };

        while let Some(mut field) = multipart.next() {
            let name = field.headers.name.to_string(); // <- Esta es la clave para que compile
            match name.as_str() {
                "marca" if field.is_text() => { let _ = field.data.read_to_string(&mut datos.marca); },
                "nombre_modelo" if field.is_text() => { let _ = field.data.read_to_string(&mut datos.nombre_modelo); },
                "categoria" if field.is_text() => {
                    let mut cat = String::new();
                    if field.data.read_to_string(&mut cat).is_ok() && !cat.trim().is_empty() { datos.categoria = Some(cat); }
                },
                "descripcion" if field.is_text() => {
                    let mut desc = String::new();
                    if field.data.read_to_string(&mut desc).is_ok() && !desc.trim().is_empty() { datos.descripcion = Some(desc); }
                },
                "manual_pdf" if field.headers.filename.is_some() => { let _ = field.data.read_to_end(&mut datos.manual_bytes); },
                "imagenes[]" if field.headers.filename.is_some() => {
                    let mut foto = Vec::new();
                    if field.data.read_to_end(&mut foto).is_ok() && !foto.is_empty() { datos.lista_imagenes_bytes.push(foto); }
                },
                _ => {}
            }
        }
        Ok(datos)
    }

    // Esta funcion se usa para almacenar el manual y las imagenes de un modelo, tanto para la creacion como para la edicion.
    // Tiene una flag para reemplazar las imagenes, que se usa para la edicion. En la creacion, no se reemplazan las imagenes.
    fn guardar_assets_modelo(
        conn: &Connection,
        modelo_id: i64,
        manual_bytes: &[u8],
        lista_imagenes_bytes: &[Vec<u8>],
        reemplazar_imagenes: bool,
    ) -> (Option<(String, String)>, usize) {
        let mut errores_imagenes = 0;

        if !manual_bytes.is_empty() {
            match validar_y_procesar_manual(manual_bytes) {
                Ok((pdf_data, mime_type)) => {
                    if let Err(e) =
                        ModeloRepository::actualizar_manual(conn, modelo_id, &pdf_data, &mime_type)
                    {
                        return (
                            Some((
                                "Error al guardar el manual".to_string(),
                                format!(
                                    "Los datos del modelo se guardaron pero falló el manual: {}",
                                    e
                                ),
                            )),
                            errores_imagenes,
                        );
                    }
                }
                Err(ManualStorageError::InvalidManual(msg)) => {
                    return (
                        Some((
                            "Manual rechazado".to_string(),
                            format!(
                                "Los datos del modelo se guardaron, pero el manual no se guardó: {}",
                                msg
                            ),
                        )),
                        errores_imagenes,
                    );
                }
                Err(e) => {
                    return (
                        Some((
                            "Error de almacenamiento de manual".to_string(),
                            e.to_string(),
                        )),
                        errores_imagenes,
                    );
                }
            }
        }

        if !lista_imagenes_bytes.is_empty() {
            if reemplazar_imagenes
                && let Err(e) = ImageRepository::eliminar_por_modelo(conn, modelo_id)
            {
                return (
                    Some((
                        "Error al reemplazar imágenes".to_string(),
                        format!(
                            "Los datos del modelo se guardaron pero no se pudieron eliminar las imágenes previas: {}",
                            e
                        ),
                    )),
                    errores_imagenes,
                );
            }

            for (index, foto_bytes) in lista_imagenes_bytes.iter().enumerate() {
                let orden_imagen = index as i32;

                match procesar_modelo(foto_bytes) {
                    Ok((blob_final, mime)) => {
                        if ImageRepository::guardar_modelo(
                            conn,
                            modelo_id,
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
        }

        (None, errores_imagenes)
    }
}
