use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;
use std::sync::mpsc::SyncSender;
use tera::Context;

use crate::templates;
use crate::{
    repository::{
        ejemplar_repository::EjemplarRepository, image_repository::ImageRepository,
        modelo_repository::ModeloRepository,
        reserva_instrumento_repository::ReservaInstrumentoRepository,
        sesion_repository::SesionRepository,
    },
    service::ejemplar_service::EjemplarService,
    service::modelo_service::ModeloService,
    service::pdf_worker_service::PdfRequest,
    service::reserva_service::ReservaService,
    utils::{Carrito, cookie_carrito, cookie_carrito_vacio, extraer_token_sesion, leer_carrito},
};

pub struct ReservaHandler;
use crate::models::reserva_view::ReservaView;
impl ReservaHandler {
    pub fn mostrar_formulario_reserva(request: &Request, conn: &Connection) -> Response {
        if let Err(response) = Self::obtener_usuario_sesion(request, conn) {
            return response;
        }

        let fecha_minima = (Local::now().date_naive() + Duration::days(5))
            .format("%Y-%m-%d")
            .to_string();

        let fecha_maxima = (Local::now().date_naive() + Duration::days(180))
            .format("%Y-%m-%d")
            .to_string();

        let carrito = leer_carrito(request);

        // Con fechas en el carrito listamos solo los modelos disponibles para ese
        // rango; sin fechas, listamos todos los modelos.
        let grupos = if carrito.tiene_fechas() {
            let inicio = carrito.fecha_inicio.clone().unwrap_or_default();
            let fin = carrito.fecha_fin.clone().unwrap_or_default();
            ModeloService::listar_cards_disponibles_agrupadas(conn, &inicio, &fin)
        } else {
            ModeloService::listar_cards_agrupadas(conn)
        };

        let grupos = match grupos {
            Ok(g) => g,
            Err(e) => {
                return templates::response_mensaje_error("No se pudieron cargar los modelos", &e);
            }
        };

        let mut ctx = Context::new();
        ctx.insert("fecha_minima", &fecha_minima);
        ctx.insert("fecha_maxima", &fecha_maxima);
        ctx.insert("grupos", &grupos);
        ctx.insert(
            "fecha_inicio",
            &carrito.fecha_inicio.clone().unwrap_or_default(),
        );
        ctx.insert("fecha_fin", &carrito.fecha_fin.clone().unwrap_or_default());
        ctx.insert("carrito_cantidad", &carrito.ejemplares.len());
        ctx.insert("oob", &false);
        templates::response_html(templates::render("reserva_formulario.html", &ctx))
    }

    /// Endpoint HTMX: devuelve el parcial con los modelos a listar.
    /// - Si falta alguna fecha (campo vacio), lista todos los modelos.
    /// - Si ambas fechas son validas, filtra por disponibilidad en el rango.
    /// - Si hay fechas pero son invalidas (mal formadas o fin <= inicio), muestra error.
    ///   En todos los casos validos reinicia el carrito (cambiar la fecha vacia los ejemplares).
    pub fn listar_modelos_disponibles(request: &Request, conn: &Connection) -> Response {
        if let Err(response) = Self::obtener_usuario_sesion(request, conn) {
            return response;
        }

        let inicio = request.get_param("fecha_inicio").unwrap_or_default();
        let fin = request.get_param("fecha_fin").unwrap_or_default();

        // Cambiar la fecha reinicia el carrito: las fechas nuevas reemplazan a las
        // anteriores y se vacia la lista de ejemplares.
        let (grupos_res, carrito) = if inicio.trim().is_empty() || fin.trim().is_empty() {
            // Sin fechas (el usuario las limpio): listamos todos los modelos.
            (
                ModeloService::listar_cards_agrupadas(conn),
                Carrito {
                    fecha_inicio: None,
                    fecha_fin: None,
                    ejemplares: Vec::new(),
                },
            )
        } else if Self::fechas_validas(&inicio, &fin) {
            (
                ModeloService::listar_cards_disponibles_agrupadas(conn, &inicio, &fin),
                Carrito {
                    fecha_inicio: Some(inicio.clone()),
                    fecha_fin: Some(fin.clone()),
                    ejemplares: Vec::new(),
                },
            )
        } else {
            return templates::response_mensaje_error(
                "Fechas inválidas",
                "Seleccioná una fecha de inicio y una de fin válidas. La fecha de fin debe ser posterior a la de inicio.",
            );
        };

        let grupos = match grupos_res {
            Ok(g) => g,
            Err(e) => {
                return templates::response_mensaje_error("No se pudieron cargar los modelos", &e);
            }
        };

        let mut ctx_modelos = Context::new();
        ctx_modelos.insert("grupos", &grupos);
        let modelos_html = match templates::render("partials/reserva_modelos.html", &ctx_modelos) {
            Ok(h) => h,
            Err(e) => {
                return Response::text(format!("Error renderizando plantilla: {}", e))
                    .with_status_code(500);
            }
        };

        // Resumen del carrito (vacio) actualizado fuera de banda (hx-swap-oob).
        let mut ctx_resumen = Context::new();
        ctx_resumen.insert("carrito_cantidad", &0usize);
        ctx_resumen.insert("oob", &true);
        let resumen_html = match templates::render("partials/carrito_resumen.html", &ctx_resumen) {
            Ok(h) => h,
            Err(e) => {
                return Response::text(format!("Error renderizando plantilla: {}", e))
                    .with_status_code(500);
            }
        };

        Response::html(format!("{}{}", modelos_html, resumen_html))
            .with_additional_header("Set-Cookie", cookie_carrito(&carrito))
    }

    /// Valida que ambas fechas esten presentes, sean parseables y que fin sea
    /// posterior a inicio. Las cotas (min/max) las aplica el input del formulario.
    fn fechas_validas(inicio: &str, fin: &str) -> bool {
        match (
            NaiveDate::parse_from_str(inicio, "%Y-%m-%d"),
            NaiveDate::parse_from_str(fin, "%Y-%m-%d"),
        ) {
            (Ok(i), Ok(f)) => f > i,
            _ => false,
        }
    }
    pub fn mostrar_ejemplares_modelo(
        request: &Request,
        conn: &Connection,
        modelo_id: i64,
    ) -> Response {
        if let Err(response) = Self::obtener_usuario_sesion(request, conn) {
            return response;
        }

        let carrito = leer_carrito(request);

        // El detalle no edita fechas: las toma del carrito. Si no hay fechas se
        // listan igual todos los ejemplares, pero no se permite agregarlos al
        // carrito (eso lo controla `con_fechas` en la plantilla).
        let con_fechas = carrito.tiene_fechas();
        let inicio = carrito.fecha_inicio.clone().unwrap_or_default();
        let fin = carrito.fecha_fin.clone().unwrap_or_default();

        let modelo = match ModeloRepository::buscar_por_id(conn, modelo_id) {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Response::text("Modelo inexistente").with_status_code(404);
            }
            Err(e) => {
                return Response::text(format!("Error cargando modelo: {}", e))
                    .with_status_code(500);
            }
        };

        let ejemplares = if con_fechas {
            EjemplarService::listar_ejemplares_para_modelo(
                conn,
                modelo_id,
                &inicio,
                &fin,
                &carrito.ejemplares,
            )
        } else {
            EjemplarService::listar_ejemplares_basico(conn, modelo_id)
        };

        let ejemplares = match ejemplares {
            Ok(e) => e,
            Err(e) => {
                return templates::response_mensaje_error(
                    "No se pudieron cargar los ejemplares",
                    &e,
                );
            }
        };

        let tiene_imagen =
            ImageRepository::existe_imagen_principal_modelo(conn, modelo.id).unwrap_or(false);
        let imagen = if tiene_imagen {
            Some(format!("/imagenes/modelos/{}/0", modelo.id))
        } else {
            None
        };

        let tiene_manual = ModeloRepository::tiene_manual(conn, modelo.id).unwrap_or(false);

        let mut ctx = Context::new();
        ctx.insert("modelo", &modelo);
        ctx.insert("imagen", &imagen);
        ctx.insert("ejemplares", &ejemplares);
        ctx.insert("fecha_inicio", &inicio);
        ctx.insert("fecha_fin", &fin);
        ctx.insert("con_fechas", &con_fechas);
        ctx.insert("tiene_manual", &tiene_manual);
        templates::response_html(templates::render("reserva_modelo.html", &ctx))
    }

    /// Agrega/actualiza los ejemplares seleccionados de un modelo en el carrito y
    /// vuelve a la pantalla de reservas. Los checkboxes son la seleccion definitiva
    /// para ese modelo: lo no marcado se quita del carrito.
    pub fn agregar_al_carrito(request: &Request, conn: &Connection, modelo_id: i64) -> Response {
        if let Err(response) = Self::obtener_usuario_sesion(request, conn) {
            return response;
        }

        let mut carrito = leer_carrito(request);
        if !carrito.tiene_fechas() {
            return templates::response_mensaje_error(
                "Sin fechas",
                "Elegí las fechas en la pantalla de reservas antes de agregar ejemplares.",
            );
        }

        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }
        let seleccionados = Self::obtener_ejemplares(&body);

        // Quitar del carrito los ejemplares de este modelo y reemplazarlos por los
        // recien seleccionados, para reflejar tanto altas como bajas.
        let del_modelo: Vec<i64> = match EjemplarRepository::listar_por_modelo(conn, modelo_id) {
            Ok(es) => es.iter().map(|e| e.id).collect(),
            Err(e) => {
                return templates::response_mensaje_error(
                    "No se pudieron cargar los ejemplares",
                    &e.to_string(),
                );
            }
        };

        carrito.ejemplares.retain(|id| !del_modelo.contains(id));
        for id in seleccionados {
            if del_modelo.contains(&id) && !carrito.ejemplares.contains(&id) {
                carrito.ejemplares.push(id);
            }
        }

        Response::redirect_303("/reservas")
            .with_additional_header("Set-Cookie", cookie_carrito(&carrito))
    }

    pub fn mostrar_carrito(request: &Request, conn: &Connection) -> Response {
        if let Err(response) = Self::obtener_usuario_sesion(request, conn) {
            return response;
        }

        let carrito = leer_carrito(request);

        let items = match ReservaService::listar_carrito_detalle(conn, &carrito.ejemplares) {
            Ok(i) => i,
            Err(e) => {
                return templates::response_mensaje_error("No se pudo cargar el carrito", &e);
            }
        };

        let mut ctx = Context::new();
        ctx.insert("items", &items);
        ctx.insert(
            "fecha_inicio",
            &carrito.fecha_inicio.clone().unwrap_or_default(),
        );
        ctx.insert("fecha_fin", &carrito.fecha_fin.clone().unwrap_or_default());
        ctx.insert("carrito_cantidad", &carrito.ejemplares.len());
        templates::response_html(templates::render("carrito_detalle.html", &ctx))
    }

    pub fn remover_del_carrito(request: &Request, conn: &Connection, ejemplar_id: i64) -> Response {
        if let Err(response) = Self::obtener_usuario_sesion(request, conn) {
            return response;
        }

        let mut carrito = leer_carrito(request);
        carrito.ejemplares.retain(|id| *id != ejemplar_id);

        Response::redirect_303("/reservas/carrito")
            .with_additional_header("Set-Cookie", cookie_carrito(&carrito))
    }

    /// Finaliza el carrito creando una unica reserva con la fecha comun y todos los
    /// ejemplares acumulados, y luego borra la cookie del carrito.
    pub fn finalizar_reserva(request: &Request, conn: &Connection) -> Response {
        let id_usuario = match Self::obtener_usuario_sesion(request, conn) {
            Ok(id) => id,
            Err(response) => {
                return response;
            }
        };

        let carrito = leer_carrito(request);
        let (fecha_inicio, fecha_fin) =
            match (carrito.fecha_inicio.clone(), carrito.fecha_fin.clone()) {
                (Some(i), Some(f)) => (i, f),
                _ => {
                    return Self::render_reserva_finalizada(
                        false,
                        "Sin fechas",
                        "Elegí las fechas antes de finalizar la reserva.",
                    );
                }
            };

        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }
        let datos = Self::parsear_formulario(&body);
        let motivo = datos.get("motivo").cloned();

        match ReservaService::crear_reserva(
            conn,
            id_usuario,
            fecha_inicio,
            fecha_fin,
            motivo,
            carrito.ejemplares.clone(),
        ) {
            Ok(_) => Self::render_reserva_finalizada(
                true,
                "Reserva creada",
                "La reserva fue registrada correctamente.",
            )
            .with_additional_header("Set-Cookie", cookie_carrito_vacio()),

            Err(e) => Self::render_reserva_finalizada(false, "No se pudo crear la reserva", &e),
        }
    }

    fn render_reserva_finalizada(exito: bool, titulo: &str, mensaje: &str) -> Response {
        let mut ctx = Context::new();
        ctx.insert("exito", &exito);
        ctx.insert("titulo", titulo);
        ctx.insert("mensaje", mensaje);
        templates::response_html(templates::render("reserva_finalizada.html", &ctx))
    }

    pub fn descargar_comprobante_pdf(
        _request: &Request,
        conn: &Connection,
        reserva_id: i64,
        pdf_tx: SyncSender<PdfRequest>,
    ) -> Response {
        let data_comprobante = match ReservaService::preparar_datos_comprobante(conn, reserva_id) {
            Ok(d) => d,
            Err(crate::errors::ErrorComprobante::NoEncontrada) => {
                return Response::text("Reserva no encontrada").with_status_code(404);
            }
            Err(crate::errors::ErrorComprobante::NoConfirmada) => {
                return Response::text(
                    "El comprobante solo esta disponible para reservas confirmadas",
                )
                .with_status_code(403);
            }
            Err(e) => {
                return Response::text(format!("Error: {}", e)).with_status_code(500);
            }
        };

        let (tx_respuesta, rx_respuesta) = oneshot::channel();

        if pdf_tx
            .send(PdfRequest {
                data: data_comprobante,
                responder: tx_respuesta,
            })
            .is_err()
        {
            return Response::text("El generador de PDF no esta disponible").with_status_code(503);
        }

        match rx_respuesta.recv() {
            Ok(Ok(pdf_bytes)) => Response::from_data("application/pdf", pdf_bytes)
                .with_additional_header(
                    "Content-Disposition",
                    format!("inline; filename=comprobante_{}.pdf", reserva_id),
                ),
            Ok(Err(e)) => {
                Response::text(format!("Error al generar PDF: {}", e)).with_status_code(500)
            }
            Err(_) => Response::text("El hilo worker de PDF no respondio").with_status_code(500),
        }
    }

    fn obtener_ejemplares(body: &str) -> Vec<i64> {
        let mut ids = Vec::new();
        for par in body.split('&') {
            if let Some(id) = par
                .strip_prefix("ejemplar_id=")
                .and_then(|v| v.parse::<i64>().ok())
            {
                ids.push(id);
            }
        }
        ids
    }

    fn parsear_formulario(body: &str) -> HashMap<String, String> {
        let mut mapa = HashMap::new();

        for par in body.split('&') {
            let mut partes = par.split('=');

            if let (Some(clave), Some(valor)) = (partes.next(), partes.next()) {
                mapa.insert(
                    clave.to_string(),
                    valor.replace("%40", "@").replace("+", " "),
                );
            }
        }

        mapa
    }

    fn obtener_usuario_sesion(request: &Request, conn: &Connection) -> Result<i64, Response> {
        let token = match extraer_token_sesion(request) {
            Some(t) => t,

            None => {
                return Err(Response::html(
                    "<div style='color:red'>
                            Debe iniciar sesión
                        </div>",
                )
                .with_status_code(401));
            }
        };

        let sesion = match SesionRepository::buscar_por_token(conn, &token) {
            Ok(Some(s)) => s,

            _ => {
                return Err(Response::html(
                    "<div style='color:red'>
                            Sesión inválida
                        </div>",
                )
                .with_status_code(401));
            }
        };

        Ok(sesion.id_usuario)
    }

    pub fn mostrar_mis_reservas(request: &Request, conn: &Connection) -> Response {
        let id_usuario = match Self::obtener_usuario_sesion(request, conn) {
            Ok(id) => id,
            Err(r) => return r,
        };

        let reservas = match ReservaService::obtener_reservas_usuario(conn, id_usuario) {
            Ok(r) => r,
            Err(e) => {
                return Response::text(format!("Error cargando reservas: {}", e))
                    .with_status_code(500);
            }
        };

        let mut reservas_vista: Vec<ReservaView> = Vec::new();

        for reserva in reservas {
            let clase_estado = match reserva.estado.as_str() {
                "activa" => "estado-aprobada",
                "concluida" => "estado-concluida",
                "pendiente" => "estado-pendiente",
                "cancelada" => "estado-cancelada",
                _ => "",
            };

            let texto_estado = match reserva.estado.as_str() {
                "activa" => "Aceptada",
                "concluida" => "Finalizada",
                "pendiente" => "Pendiente",
                "cancelada" => "Cancelada",
                _ => &reserva.estado,
            };

            let equipos =
                ReservaInstrumentoRepository::obtener_nombres_equipos_reserva(conn, reserva.id)
                    .unwrap_or(vec![]);

            let inicio = NaiveDate::parse_from_str(&reserva.fecha_inicio, "%Y-%m-%d").unwrap();
            let fin = NaiveDate::parse_from_str(&reserva.fecha_fin, "%Y-%m-%d").unwrap();
            let dias = (fin - inicio).num_days();

            let creada =
                NaiveDateTime::parse_from_str(&reserva.momento_creacion, "%Y-%m-%d %H:%M:%S")
                    .unwrap();

            let ahora = Local::now().naive_local();
            let dias_desde = (ahora - creada).num_days();

            let creada_txt = if dias_desde == 0 {
                "Hoy".to_string()
            } else {
                format!("Hace {} días", dias_desde)
            };

            reservas_vista.push(ReservaView {
                id: reserva.id,
                fecha_inicio: reserva.fecha_inicio,
                fecha_fin: reserva.fecha_fin,
                estado: reserva.estado.clone(),
                texto_estado: texto_estado.to_string(),
                clase_estado: clase_estado.to_string(),
                motivo: reserva.motivo.unwrap_or("Sin motivo".to_string()),
                equipos,
                dias,
                creada: creada_txt,
            });
        }

        let mut ctx = Context::new();
        ctx.insert("reservas", &reservas_vista);

        let html = match templates::render("mis_reservas.html", &ctx) {
            Ok(html) => html,

            Err(e) => {
                eprintln!("ERROR TERA: {:?}", e);

                return Response::text(format!("Error Tera: {:?}", e)).with_status_code(500);
            }
        };

        templates::response_html(Ok(html))
    }

    pub fn cancelar_reserva(request: &Request, conn: &Connection, reserva_id: i64) -> Response {
        let usuario_id = match Self::obtener_usuario_sesion(request, conn) {
            Ok(id) => id,

            Err(response) => {
                return response;
            }
        };

        match crate::repository::reserva_repository::ReservaRepository::cancelar_por_usuario(
            conn, reserva_id, usuario_id,
        ) {
            Ok(filas) => {
                if filas == 0 {
                    return templates::response_mensaje_error(
                        "Error cancelando reserva",
                        "La reserva no existe o ya fue cancelada",
                    );
                }

                Response::redirect_303("/mis-reservas")
            }

            Err(e) => templates::response_mensaje_error("Error cancelando reserva", &e.to_string()),
        }
    }
}
