use crate::templates;
use crate::{
    repository::{
        ejemplar_repository::EjemplarRepository, image_repository::ImageRepository,
        modelo_repository::ModeloRepository, sesion_repository::SesionRepository,
    },
    service::modelo_service::ModeloService,
    service::reserva_service::ReservaService,
    utils::{
        cookie_carrito, cookie_carrito_vacio, extraer_token_sesion, leer_carrito, Carrito,
    },
};

use chrono::{Duration, Local, NaiveDate};
use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;
use tera::Context;

pub struct ReservaHandler;

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
        ctx.insert("fecha_inicio", &carrito.fecha_inicio.clone().unwrap_or_default());
        ctx.insert("fecha_fin", &carrito.fecha_fin.clone().unwrap_or_default());
        ctx.insert("carrito_cantidad", &carrito.ejemplares.len());
        ctx.insert("oob", &false);
        templates::response_html(templates::render("reserva_formulario.html", &ctx))
    }

    /// Endpoint HTMX: devuelve el parcial con los modelos a listar.
    /// Si llegan ambas fechas validas, filtra por disponibilidad en el rango;
    /// si no, lista todos los modelos.
    pub fn listar_modelos_disponibles(request: &Request, conn: &Connection) -> Response {
        if let Err(response) = Self::obtener_usuario_sesion(request, conn) {
            return response;
        }

        let inicio = request.get_param("fecha_inicio").unwrap_or_default();
        let fin = request.get_param("fecha_fin").unwrap_or_default();

        if !Self::fechas_validas(&inicio, &fin) {
            return templates::response_mensaje_error(
                "Fechas inválidas",
                "Seleccioná una fecha de inicio y una de fin válidas. La fecha de fin debe ser posterior a la de inicio.",
            );
        }

        let grupos = match ModeloService::listar_cards_disponibles_agrupadas(conn, &inicio, &fin) {
            Ok(g) => g,
            Err(e) => {
                return templates::response_mensaje_error("No se pudieron cargar los modelos", &e);
            }
        };

        // Cambiar la fecha reinicia el carrito: las fechas nuevas reemplazan a las
        // anteriores y se vacia la lista de ejemplares.
        let carrito = Carrito {
            fecha_inicio: Some(inicio.clone()),
            fecha_fin: Some(fin.clone()),
            ejemplares: Vec::new(),
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

        // El detalle no edita fechas: las toma del carrito. Sin fechas no se puede
        // evaluar disponibilidad, asi que se invita a elegirlas en /reservas.
        if !carrito.tiene_fechas() {
            return templates::response_mensaje_error(
                "Elegí las fechas primero",
                "Volvé a la pantalla de reservas y seleccioná las fechas antes de elegir ejemplares.",
            );
        }

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

        let ejemplares = match ReservaService::listar_ejemplares_para_modelo(
            conn,
            modelo_id,
            &inicio,
            &fin,
            &carrito.ejemplares,
        ) {
            Ok(e) => e,
            Err(e) => {
                return templates::response_mensaje_error("No se pudieron cargar los ejemplares", &e);
            }
        };

        let tiene_imagen =
            ImageRepository::existe_imagen_principal_modelo(conn, modelo.id).unwrap_or(false);
        let imagen = if tiene_imagen {
            Some(format!("/imagenes/modelos/{}/0", modelo.id))
        } else {
            None
        };

        let mut ctx = Context::new();
        ctx.insert("modelo", &modelo);
        ctx.insert("imagen", &imagen);
        ctx.insert("ejemplares", &ejemplares);
        ctx.insert("fecha_inicio", &inicio);
        ctx.insert("fecha_fin", &fin);
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
                    return templates::response_mensaje_error(
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
        let motivo = datos
            .get("motivo")
            .cloned()
            .filter(|m| !m.trim().is_empty());

        match ReservaService::crear_reserva(
            conn,
            id_usuario,
            fecha_inicio,
            fecha_fin,
            motivo,
            carrito.ejemplares.clone(),
        ) {
            Ok(_) => templates::response_mensaje_exito(
                "Reserva creada",
                "La reserva fue registrada correctamente.",
            )
            .with_additional_header("Set-Cookie", cookie_carrito_vacio()),

            Err(e) => templates::response_mensaje_error("No se pudo crear la reserva", &e),
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
}
