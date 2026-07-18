use chrono::{Duration, Local, NaiveDate, NaiveDateTime, Utc};
use rouille::{Request, Response};
use rusqlite::Connection;
use std::io::Read;
use std::sync::mpsc::SyncSender;
use tera::Context;

use crate::models::reserva_view::ReservaView;
use crate::templates;
use crate::{
    repository::{
        ejemplar_repository::EjemplarRepository, image_repository::ImageRepository,
        modelo_repository::ModeloRepository,
        reserva_instrumento_repository::ReservaInstrumentoRepository,
        reserva_repository::ReservaRepository, sesion_repository::SesionRepository,
        usuario_repository::UsuarioRepository,
    },
    service::comprobante_service::ComprobanteData,
    service::ejemplar_service::{EjemplarDTO, EjemplarService},
    service::modelo_service::ModeloService,
    service::pdf_worker_service::PdfRequest,
    service::reserva_service::ReservaService,
    utils::{
        Carrito, cookie_carrito, cookie_carrito_vacio, extraer_token_sesion, leer_carrito,
        parsear_formulario, usuario_actual,
    },
};

struct ParamsReserva {
    inicio: String,
    fin: String,
    buscar: String,
    categoria: String,
    orden: String,
}

pub struct ReservaHandler;
impl ReservaHandler {
    pub fn mostrar_formulario_reserva(req: &Request, conn: &Connection) -> Response {
        if let Err(r) = Self::obtener_usuario_sesion(req, conn) {
            return r;
        }
        let mut p = Self::extraer_params(req);
        let c = leer_carrito(req);
        if p.inicio.is_empty() && p.fin.is_empty() {
            p.inicio = c.fecha_inicio.clone().unwrap_or_default();
            p.fin = c.fecha_fin.clone().unwrap_or_default();
        }
        let g = Self::filtrar_grupos(conn, &p).unwrap_or_default();

        let mut ctx = Context::new();
        Self::insertar_fechas_limite(&mut ctx);
        Self::insertar_datos_busqueda(&mut ctx, &p, conn);
        Self::insertar_datos_carrito(&mut ctx, &c);

        ctx.insert("grupos", &g);
        ctx.insert("oob", &false);
        ctx.insert("avatar_cache_buster", &0);
        if let Ok(u) = usuario_actual(req, conn) {
            ctx.insert("usuario_actual", &u);
        }

        templates::response_html(templates::render("reserva_formulario.html", &ctx))
    }

    pub fn listar_modelos_disponibles(req: &Request, conn: &Connection) -> Response {
        if let Err(r) = Self::obtener_usuario_sesion(req, conn) {
            return r;
        }

        match Self::procesar_busqueda_htmx(req, conn, false) {
            Ok((g, c, p)) => match Self::renderizar_parciales_htmx(&g, &p.buscar, &c) {
                Ok(html) => {
                    Response::html(html).with_additional_header("Set-Cookie", cookie_carrito(&c))
                }
                Err(r) => r,
            },
            Err(r) => r,
        }
    }

    pub fn buscar_modelos(req: &Request, conn: &Connection) -> Response {
        if let Err(r) = Self::obtener_usuario_sesion(req, conn) {
            return r;
        }

        match Self::procesar_busqueda_htmx(req, conn, true) {
            Ok((g, c, p)) => {
                let mut ctx = Context::new();
                ctx.insert("grupos", &g);
                ctx.insert("busqueda", &p.buscar);
                match templates::render("partials/reserva_modelos.html", &ctx) {
                    Ok(html) => Response::html(html)
                        .with_additional_header("Set-Cookie", cookie_carrito(&c)),
                    Err(e) => Response::text(e.to_string()).with_status_code(500),
                }
            }
            Err(r) => r,
        }
    }

    pub fn mostrar_ejemplares_modelo(req: &Request, conn: &Connection, mod_id: i64) -> Response {
        if let Err(r) = Self::obtener_usuario_sesion(req, conn) {
            return r;
        }
        let c = leer_carrito(req);
        let modelo = match ModeloRepository::buscar_por_id(conn, mod_id) {
            Ok(Some(m)) => m,
            _ => return Response::text("Modelo inexistente").with_status_code(404),
        };

        let inicio = c.fecha_inicio.clone().unwrap_or_default();
        let fin = c.fecha_fin.clone().unwrap_or_default();
        let ejemplares =
            Self::obtener_ejemplares_modelo(conn, mod_id, &inicio, &fin, &c).unwrap_or_default();
        let img = ImageRepository::existe_imagen_principal_modelo(conn, mod_id)
            .ok()
            .filter(|&x| x)
            .map(|_| format!("/imagenes/modelos/{}/0", mod_id));

        let mut ctx = Context::new();
        ctx.insert("modelo", &modelo);
        ctx.insert("imagen", &img);
        ctx.insert("ejemplares", &ejemplares);
        ctx.insert("fecha_inicio", &inicio);
        ctx.insert("fecha_fin", &fin);
        ctx.insert("con_fechas", &c.tiene_fechas());
        ctx.insert(
            "tiene_manual",
            &ModeloRepository::tiene_manual(conn, mod_id).unwrap_or(false),
        );

        templates::response_html(templates::render("reserva_modelo.html", &ctx))
    }

    pub fn agregar_al_carrito(req: &Request, conn: &Connection, mod_id: i64) -> Response {
        if let Err(r) = Self::obtener_usuario_sesion(req, conn) {
            return r;
        }
        let mut c = leer_carrito(req);

        if !c.tiene_fechas() {
            return templates::response_mensaje_error("Sin fechas", "Elegí fechas antes.");
        }

        let seleccionados = Self::obtener_ids_desde_body(req);
        let del_modelo: Vec<i64> = EjemplarRepository::listar_por_modelo(conn, mod_id)
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.id)
            .collect();

        c.ejemplares.retain(|id| !del_modelo.contains(id));
        for id in seleccionados {
            if del_modelo.contains(&id) && !c.ejemplares.contains(&id) {
                c.ejemplares.push(id);
            }
        }

        Response::redirect_303("/reservas").with_additional_header("Set-Cookie", cookie_carrito(&c))
    }

    pub fn actualizar_motivo_carrito(req: &Request, conn: &Connection) -> Response {
        if let Err(r) = Self::obtener_usuario_sesion(req, conn) {
            return r;
        }
        let mut c = leer_carrito(req);
        let mut body = String::new();
        if let Some(mut reader) = req.data() {
            let _ = reader.read_to_string(&mut body);
        }

        c.motivo = parsear_formulario(&body)
            .get("motivo")
            .cloned()
            .filter(|m| !m.trim().is_empty());
        Response::text("").with_additional_header("Set-Cookie", cookie_carrito(&c))
    }

    pub fn mostrar_carrito(req: &Request, conn: &Connection) -> Response {
        if let Err(r) = Self::obtener_usuario_sesion(req, conn) {
            return r;
        }
        let c = leer_carrito(req);
        let items = ReservaService::listar_carrito_detalle(conn, &c.ejemplares).unwrap_or_default();

        let mut ctx = Context::new();
        ctx.insert("items", &items);
        Self::insertar_datos_carrito(&mut ctx, &c);
        ctx.insert("avatar_cache_buster", &0);
        if let Ok(u) = usuario_actual(req, conn) {
            ctx.insert("usuario_actual", &u);
        }

        templates::response_html(templates::render("carrito_detalle.html", &ctx))
    }

    pub fn remover_del_carrito(req: &Request, conn: &Connection, ej_id: i64) -> Response {
        if let Err(r) = Self::obtener_usuario_sesion(req, conn) {
            return r;
        }
        let mut c = leer_carrito(req);
        c.ejemplares.retain(|id| *id != ej_id);
        Response::redirect_303("/reservas/carrito")
            .with_additional_header("Set-Cookie", cookie_carrito(&c))
    }

    pub fn finalizar_reserva(req: &Request, conn: &Connection) -> Response {
        let user_id = match Self::obtener_usuario_sesion(req, conn) {
            Ok(id) => id,
            Err(r) => return r,
        };
        let c = leer_carrito(req);

        let (inicio, fin) = match (c.fecha_inicio.clone(), c.fecha_fin.clone()) {
            (Some(i), Some(f)) => (i, f),
            _ => {
                return Self::render_reserva_finalizada(false, "Sin fechas", "Elegí fechas antes.");
            }
        };

        match ReservaService::crear_reserva(conn, user_id, inicio, fin, c.motivo, c.ejemplares) {
            Ok(_) => {
                Self::render_reserva_finalizada(true, "Reserva creada", "Registrada correctamente.")
                    .with_additional_header("Set-Cookie", cookie_carrito_vacio())
            }
            Err(e) => Self::render_reserva_finalizada(false, "Error", &e),
        }
    }

    pub fn descargar_comprobante_pdf(
        req: &Request,
        conn: &Connection,
        id: i64,
        tx: SyncSender<PdfRequest>,
    ) -> Response {
        let user_id = match Self::obtener_usuario_sesion(req, conn) {
            Ok(id) => id,
            Err(r) => return r,
        };

        if let Err(r) = Self::validar_acceso_comprobante(conn, id, user_id) {
            return r;
        }

        let data = match ReservaService::preparar_datos_comprobante(conn, id) {
            Ok(d) => d,
            Err(crate::errors::ErrorComprobante::NoConfirmada) => {
                return Response::text("Solo confirmadas").with_status_code(403);
            }
            _ => return Response::text("Error interno").with_status_code(500),
        };

        Self::generar_y_enviar_pdf(data, tx, id)
    }

    pub fn mostrar_mis_reservas(req: &Request, conn: &Connection) -> Response {
        let id_user = match Self::obtener_usuario_sesion(req, conn) {
            Ok(id) => id,
            Err(r) => return r,
        };
        ReservaService::sincronizar_si_necesario(conn);

        let reservas = ReservaService::obtener_reservas_usuario(conn, id_user).unwrap_or_default();
        let vistas: Vec<ReservaView> = reservas
            .into_iter()
            .map(|r| Self::mapear_reserva_a_vista(conn, &r))
            .collect();
        Self::renderizar_vista_reservas(req, conn, &vistas)
    }

    pub fn cancelar_reserva(req: &Request, conn: &Connection, id: i64) -> Response {
        let uid = match Self::obtener_usuario_sesion(req, conn) {
            Ok(id) => id,
            Err(r) => return r,
        };
        match ReservaRepository::cancelar_por_usuario(conn, id, uid) {
            Ok(0) => templates::response_mensaje_error("Error", "No existe o ya cancelada"),
            Ok(_) => Response::redirect_303("/mis-reservas"),
            Err(e) => templates::response_mensaje_error("Error", &e.to_string()),
        }
    }

    fn procesar_busqueda_htmx(
        req: &Request,
        conn: &Connection,
        retener: bool,
    ) -> Result<
        (
            Vec<crate::service::modelo_service::GrupoCategoriaDTO>,
            Carrito,
            ParamsReserva,
        ),
        Response,
    > {
        let p = Self::extraer_params(req);
        let c = leer_carrito(req);
        let mut nc = Carrito {
            motivo: c.motivo.clone(),
            ejemplares: vec![],
            fecha_inicio: None,
            fecha_fin: None,
        };

        if p.inicio.trim().is_empty() || p.fin.trim().is_empty() {
            let g = ModeloService::filtrar_y_ordenar_cards(conn, &p.buscar, &p.categoria, &p.orden)
                .map_err(|e| templates::response_mensaje_error("Err", &e))?;
            return Ok((g, nc, p));
        }

        if !Self::fechas_validas(&p.inicio, &p.fin) {
            return Err(templates::response_mensaje_error(
                "Invalido",
                "Fechas incorrectas.",
            ));
        }

        nc.fecha_inicio = Some(p.inicio.clone());
        nc.fecha_fin = Some(p.fin.clone());
        if retener && c.fecha_inicio == nc.fecha_inicio && c.fecha_fin == nc.fecha_fin {
            nc.ejemplares = c.ejemplares;
        }

        let g = ModeloService::filtrar_y_ordenar_cards_disponibles(
            conn,
            &p.inicio,
            &p.fin,
            &p.buscar,
            &p.categoria,
            &p.orden,
        )
        .map_err(|e| templates::response_mensaje_error("Err", &e))?;
        Ok((g, nc, p))
    }

    fn extraer_params(req: &Request) -> ParamsReserva {
        ParamsReserva {
            inicio: req.get_param("fecha_inicio").unwrap_or_default(),
            fin: req.get_param("fecha_fin").unwrap_or_default(),
            buscar: req.get_param("buscar").unwrap_or_default(),
            categoria: req.get_param("categoria").unwrap_or_default(),
            orden: req.get_param("orden").unwrap_or_default(),
        }
    }

    fn obtener_ejemplares_modelo(
        conn: &Connection,
        id: i64,
        ini: &str,
        fin: &str,
        c: &Carrito,
    ) -> Result<Vec<EjemplarDTO>, String> {
        if c.tiene_fechas() {
            EjemplarService::listar_ejemplares_para_modelo(conn, id, ini, fin, &c.ejemplares)
        } else {
            EjemplarService::listar_ejemplares_basico(conn, id)
        }
    }

    fn filtrar_grupos(
        conn: &Connection,
        p: &ParamsReserva,
    ) -> Result<Vec<crate::service::modelo_service::GrupoCategoriaDTO>, String> {
        if !p.inicio.is_empty() && !p.fin.is_empty() {
            ModeloService::filtrar_y_ordenar_cards_disponibles(
                conn,
                &p.inicio,
                &p.fin,
                &p.buscar,
                &p.categoria,
                &p.orden,
            )
        } else {
            ModeloService::filtrar_y_ordenar_cards(conn, &p.buscar, &p.categoria, &p.orden)
        }
    }

    fn validar_acceso_comprobante(
        conn: &Connection,
        res_id: i64,
        user_id: i64,
    ) -> Result<(), Response> {
        let reserva = match crate::repository::reserva_repository::ReservaRepository::buscar_por_id(
            conn, res_id,
        ) {
            Ok(Some(r)) => r,
            _ => return Err(Response::text("No encontrada").with_status_code(404)),
        };
        if reserva.id_usuario != user_id {
            let es_admin = UsuarioRepository::buscar_por_id(conn, user_id)
                .map(|u| u.map(|x| x.es_admin()).unwrap_or(false))
                .unwrap_or(false);
            if !es_admin {
                return Err(templates::response_mensaje_error_con_status(
                    "Denegado",
                    "Sin permisos",
                    403,
                ));
            }
        }
        Ok(())
    }

    fn generar_y_enviar_pdf(
        data: ComprobanteData,
        pdf_tx: SyncSender<PdfRequest>,
        id: i64,
    ) -> Response {
        let (tx, rx) = oneshot::channel();
        if pdf_tx
            .send(PdfRequest {
                data,
                responder: tx,
            })
            .is_err()
        {
            return Response::text("PDF engine error").with_status_code(503);
        }
        match rx.recv() {
            Ok(Ok(bytes)) => Response::from_data("application/pdf", bytes).with_additional_header(
                "Content-Disposition",
                format!("inline; filename=comprobante_{}.pdf", id),
            ),
            _ => Response::text("Worker error").with_status_code(500),
        }
    }

    fn renderizar_parciales_htmx(
        g: &[crate::service::modelo_service::GrupoCategoriaDTO],
        b: &str,
        c: &Carrito,
    ) -> Result<String, Response> {
        let mut ctx_m = Context::new();
        ctx_m.insert("grupos", g);
        ctx_m.insert("busqueda", b);
        let html_m = templates::render("partials/reserva_modelos.html", &ctx_m)
            .map_err(|e| Response::text(e.to_string()).with_status_code(500))?;

        let mut ctx_r = Context::new();
        ctx_r.insert("carrito_cantidad", &c.ejemplares.len());
        ctx_r.insert("motivo", &c.motivo.clone().unwrap_or_default());
        ctx_r.insert("oob", &true);
        let html_r = templates::render("partials/carrito_resumen.html", &ctx_r)
            .map_err(|e| Response::text(e.to_string()).with_status_code(500))?;

        Ok(format!("{}{}", html_m, html_r))
    }

    fn insertar_fechas_limite(ctx: &mut Context) {
        let hoy = Local::now().date_naive();
        ctx.insert(
            "fecha_minima",
            &(hoy + Duration::days(5)).format("%Y-%m-%d").to_string(),
        );
        ctx.insert(
            "fecha_maxima",
            &(hoy + Duration::days(120)).format("%Y-%m-%d").to_string(),
        );
        ctx.insert(
            "fecha_minima_display",
            &(hoy + Duration::days(5)).format("%d-%m-%Y").to_string(),
        );
        ctx.insert(
            "fecha_maxima_display",
            &(hoy + Duration::days(120)).format("%d-%m-%Y").to_string(),
        );
    }

    fn insertar_datos_busqueda(ctx: &mut Context, p: &ParamsReserva, conn: &Connection) {
        ctx.insert("busqueda", &p.buscar);
        ctx.insert("categoria", &p.categoria);
        ctx.insert("orden", &p.orden);
        ctx.insert("categorias", &ModeloService::obtener_lista_categorias(conn));
    }

    fn insertar_datos_carrito(ctx: &mut Context, c: &Carrito) {
        ctx.insert("fecha_inicio", &c.fecha_inicio.clone().unwrap_or_default());
        ctx.insert("fecha_fin", &c.fecha_fin.clone().unwrap_or_default());
        ctx.insert("motivo", &c.motivo.clone().unwrap_or_default());
        ctx.insert("carrito_cantidad", &c.ejemplares.len());
    }

    fn mapear_reserva_a_vista(
        conn: &Connection,
        r: &crate::models::reserva::Reserva,
    ) -> ReservaView {
        let ini = NaiveDate::parse_from_str(&r.fecha_inicio, "%Y-%m-%d").unwrap();
        let fin = NaiveDate::parse_from_str(&r.fecha_fin, "%Y-%m-%d").unwrap();
        let cr = NaiveDateTime::parse_from_str(&r.momento_creacion, "%Y-%m-%d %H:%M:%S").unwrap();

        let dias_d = (Utc::now().naive_utc() - cr).num_days();
        let txt_cr = if dias_d == 0 {
            "Hoy".to_string()
        } else {
            format!("Hace {} días", dias_d)
        };
        let eqs = ReservaInstrumentoRepository::obtener_detalle_equipos_reserva(conn, r.id)
            .unwrap_or_default();

        ReservaView {
            id: r.id,
            fecha_inicio: ini.format("%d-%m-%Y").to_string(),
            fecha_fin: fin.format("%d-%m-%Y").to_string(),
            estado: r.estado.clone(),
            texto_estado: Self::txt_estado(&r.estado),
            clase_estado: Self::cls_estado(&r.estado),
            motivo: r.motivo.clone().unwrap_or_else(|| "Sin motivo".to_string()),
            equipos: eqs,
            dias: (fin - ini).num_days() + 1,
            creada: txt_cr,
        }
    }

    fn txt_estado(estado: &str) -> String {
        match estado {
            "activa" => "Aceptada",
            "concluida" => "Finalizada",
            "cancelada" => "Cancelada",
            _ => "Pendiente",
        }
        .into()
    }

    fn cls_estado(estado: &str) -> String {
        match estado {
            "activa" => "estado-aprobada",
            "concluida" => "estado-concluida",
            "cancelada" => "estado-cancelada",
            _ => "estado-pendiente",
        }
        .into()
    }

    fn renderizar_vista_reservas(
        req: &Request,
        conn: &Connection,
        res: &[ReservaView],
    ) -> Response {
        let mut ctx = Context::new();
        ctx.insert("reservas", res);
        ctx.insert("avatar_cache_buster", &0);
        if let Ok(u) = usuario_actual(req, conn) {
            ctx.insert("usuario_actual", &u);
        }
        match templates::render("mis_reservas.html", &ctx) {
            Ok(h) => templates::response_html(Ok(h)),
            Err(e) => Response::text(e.to_string()).with_status_code(500),
        }
    }

    fn render_reserva_finalizada(exito: bool, t: &str, m: &str) -> Response {
        let mut ctx = Context::new();
        ctx.insert("exito", &exito);
        ctx.insert("titulo", t);
        ctx.insert("mensaje", m);
        templates::response_html(templates::render("reserva_finalizada.html", &ctx))
    }

    fn fechas_validas(i: &str, f: &str) -> bool {
        match (
            NaiveDate::parse_from_str(i, "%Y-%m-%d"),
            NaiveDate::parse_from_str(f, "%Y-%m-%d"),
        ) {
            (Ok(i), Ok(f)) => f >= i,
            _ => false,
        }
    }

    fn obtener_ids_desde_body(req: &Request) -> Vec<i64> {
        let mut body = String::new();
        if let Some(mut r) = req.data() {
            let _ = r.read_to_string(&mut body);
        }
        body.split('&')
            .filter_map(|p| p.strip_prefix("ejemplar_id=").and_then(|v| v.parse().ok()))
            .collect()
    }

    fn obtener_usuario_sesion(req: &Request, conn: &Connection) -> Result<i64, Response> {
        let token = extraer_token_sesion(req).ok_or_else(|| {
            Response::html("<div style='color:red'>Debe iniciar sesión</div>").with_status_code(401)
        })?;
        SesionRepository::buscar_por_token(conn, &token)
            .ok()
            .flatten()
            .map(|s| s.id_usuario)
            .ok_or_else(|| {
                Response::html("<div style='color:red'>Sesión inválida</div>").with_status_code(401)
            })
    }
}
