use chrono::NaiveDate;
use rouille::{Request, Response};
use rusqlite::Connection;
use serde::Serialize;
use std::io::Read;
use std::sync::mpsc::SyncSender;
use tera::Context;

use crate::repository::{
    reserva_repository::ReservaRepository, sesion_repository::SesionRepository,
    usuario_repository::UsuarioRepository,
};
use crate::service::{
    mail_service::MailService, pdf_worker_service::PdfRequest, reserva_service::ReservaService,
};
use crate::templates;
use crate::utils::{extraer_token_sesion, parsear_formulario, utc_str_a_arg};

#[derive(Serialize)]
pub struct EjemplarVista {
    pub id: i64,
    pub identificador: String,
}

#[derive(Serialize)]
pub struct ModeloAgrupado {
    pub modelo_id: i64,
    pub marca_modelo: String,
    pub ejemplares: Vec<EjemplarVista>,
}

#[derive(Serialize)]
pub struct CategoriaAgrupada {
    pub nombre_categoria: String,
    pub modelos: Vec<ModeloAgrupado>,
}

#[derive(Serialize)]
struct HistorialReservaVista {
    pub id: i64,
    pub profesor: String,
    pub legajo: i64,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub dias: i64,
    pub estado: String,
    pub texto_estado: String,
    pub clase_estado: String,
    pub motivo: String,
    pub momento_creacion: String,
    pub momento_confirmacion: String,
}

#[derive(Serialize)]
struct ReservaVista {
    pub id: i64,
    pub profe_nombre: String,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub motivo: String,
    pub categorias: Vec<CategoriaAgrupada>,
}

struct FiltrosHistorial {
    docente: String,
    estado: String,
    fecha_desde: String,
    fecha_hasta: String,
    motivo: String,
    ordenar_por: String,
    direccion: String,
    pagina: usize,
}

pub struct AdminHandler;

impl AdminHandler {
    pub fn mostrar_dashboard(req: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(req, conn) {
            return resp;
        }
        let mut ctx = Context::new();
        if let Ok(u) = crate::utils::usuario_actual(req, conn) {
            ctx.insert("usuario_actual", &u);
        }

        ctx.insert("reservas", &Self::obtener_reservas_detalladas(conn));
        ctx.insert(
            "profesores",
            &UsuarioRepository::listar_profesores_pendientes(conn).unwrap_or_default(),
        );
        ctx.insert(
            "docentes_aprobados",
            &UsuarioRepository::listar_docentes_aprobados(conn).unwrap_or_default(),
        );
        ctx.insert(
            "administradores",
            &UsuarioRepository::listar_administradores(conn).unwrap_or_default(),
        );

        templates::response_html(templates::render("admin_dashboard.html", &ctx))
    }

    pub fn recargar_tablas_htmx(req: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(req, conn) {
            return resp;
        }
        let mut ctx = Context::new();

        ctx.insert("reservas", &Self::obtener_reservas_detalladas(conn));
        ctx.insert(
            "profesores",
            &UsuarioRepository::listar_profesores_pendientes(conn).unwrap_or_default(),
        );
        ctx.insert(
            "docentes_aprobados",
            &UsuarioRepository::listar_docentes_aprobados(conn).unwrap_or_default(),
        );
        ctx.insert(
            "administradores",
            &UsuarioRepository::listar_administradores(conn).unwrap_or_default(),
        );
        ctx.insert(
            "tab_activo",
            &req.get_param("tab_activo").unwrap_or_else(|| "0".into()),
        );

        templates::response_html(templates::render("partials/admin_tablas.html", &ctx))
    }

    pub fn mostrar_historial_reservas(req: &Request, conn: &Connection) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }

        ReservaService::sincronizar_si_necesario(conn);

        let f = Self::extraer_filtros_historial(req);
        let mut reservas = Self::obtener_historial_reservas_filtrado(conn, &f);
        let total = reservas.len();
        let por_pagina = 20;
        let tot_pag = if total == 0 {
            1
        } else {
            (total as f64 / por_pagina as f64).ceil() as usize
        };
        let pag_actual = f.pagina.min(tot_pag).max(1);

        let inicio = (pag_actual - 1) * por_pagina;
        reservas = reservas.into_iter().skip(inicio).take(por_pagina).collect();

        Self::renderizar_historial(req, conn, reservas, &f, pag_actual, tot_pag)
    }

    pub fn exportar_historial_csv(req: &Request, conn: &Connection) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        let f = Self::extraer_filtros_historial(req);
        let reservas = Self::obtener_historial_reservas_filtrado(conn, &f);

        let mut csv = String::from(
            "ID,Docente,Estado Real,Fecha Inicio,Fecha Fin,Motivo,Creada,Firma Auditoria\n",
        );
        for r in reservas {
            let fila = format!(
                "{},{},{},{},{},{},{},{}\n",
                r.id,
                r.profesor.replace(',', " "),
                r.texto_estado,
                r.fecha_inicio,
                r.fecha_fin,
                r.motivo.replace(',', " "),
                r.momento_creacion,
                r.momento_confirmacion
            );
            csv.push_str(&fila);
        }
        Response::from_data("text/csv", csv).with_additional_header(
            "Content-Disposition",
            "attachment; filename=\"historial_reservas.csv\"",
        )
    }

    pub fn aprobar_reserva(
        req: &Request,
        conn: &Connection,
        id: i64,
        tx: &SyncSender<PdfRequest>,
    ) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        let admin = match crate::utils::usuario_actual(req, conn) {
            Ok(u) => u,
            Err(e) => return e,
        };

        match crate::service::reserva_service::ReservaService::aprobar_reserva(
            conn, id, admin.id, tx,
        ) {
            Ok(_) => Response::html("")
                .with_additional_header("HX-Trigger", format!("{{\"abrirComprobante\": {}}}", id)),
            Err(ref e) => templates::response_mensaje_error("Error al aprobar", e),
        }
    }

    pub fn rechazar_reserva(req: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        if let Ok(Some(r)) = ReservaRepository::buscar_por_id(conn, id)
            && let Ok(Some(u)) = UsuarioRepository::buscar_por_id(conn, r.id_usuario)
        {
            let _ = ReservaRepository::cambiar_estado(conn, id, "cancelada");
            let nom = format!("{} {}", u.nombre, u.apellido);
            let mot = r.motivo.unwrap_or_else(|| "Uso de instrumental".into());
            let _ = MailService::enviar_notificacion_reserva_rechazada(
                &u.email,
                &nom,
                &id.to_string(),
                &mot,
            );
        }
        Response::html("")
    }

    pub fn aprobar_profesor(req: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        if let Ok(Some(u)) = UsuarioRepository::buscar_por_id(conn, id) {
            let _ = UsuarioRepository::aprobar_profesor(conn, id);
            let _ = MailService::enviar_notificacion_profesor_aprobado(
                &u.email,
                &format!("{} {}", u.nombre, u.apellido),
            );
        }
        Response::html("")
    }

    pub fn rechazar_profesor(req: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        if let Ok(Some(u)) = UsuarioRepository::buscar_por_id(conn, id) {
            let _ = MailService::enviar_notificacion_profesor_rechazado(
                &u.email,
                &format!("{} {}", u.nombre, u.apellido),
            );
            let _ = UsuarioRepository::eliminar(conn, id);
        }
        Response::html("")
    }

    pub fn hacer_admin(req: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        let _ = UsuarioRepository::hacer_admin(conn, id);
        Response::html("")
    }

    pub fn quitar_admin(req: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        if UsuarioRepository::listar_administradores(conn)
            .map(|l| l.len())
            .unwrap_or(0)
            <= 1
        {
            return Response::text("Debe existir al menos un administrador.").with_status_code(400);
        }
        let _ = UsuarioRepository::quitar_admin(conn, id);
        Response::html("")
    }

    pub fn procesar_cambio_rol(req: &Request, conn: &Connection) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        let (id, rol) = match Self::extraer_id_rol_body(req) {
            Ok(v) => v,
            Err(r) => return r,
        };

        match UsuarioRepository::actualizar_rol(conn, id, &rol) {
            Ok(_) => templates::response_mensaje_exito(
                "Rol actualizado",
                &format!("Configurado con el rol '{}'.", rol),
            ),
            Err(e) => templates::response_mensaje_error("Error de asignación", &e.to_string()),
        }
    }

    pub fn previsualizar_comprobante(
        req: &Request,
        conn: &Connection,
        id: i64,
        tx: &SyncSender<PdfRequest>,
    ) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        let data =
            match crate::service::reserva_service::ReservaService::preparar_datos_previsualizacion(
                conn, id,
            ) {
                Ok(d) => d,
                Err(e) => {
                    return Response::text(format!("Error simulación: {}", e))
                        .with_status_code(500);
                }
            };
        Self::generar_pdf_interno(data, tx, "previsualizacion.pdf", "inline")
    }

    pub fn descargar_comprobante_admin(
        req: &Request,
        conn: &Connection,
        id: i64,
        tx: &SyncSender<PdfRequest>,
    ) -> Response {
        if let Err(e) = Self::verificar_admin(req, conn) {
            return e;
        }
        let data = match crate::service::reserva_service::ReservaService::preparar_datos_comprobante(
            conn, id,
        ) {
            Ok(d) => d,
            Err(e) => return Response::text(format!("Error: {:?}", e)).with_status_code(400),
        };
        Self::generar_pdf_interno(data, tx, &format!("comprobante_{}.pdf", id), "attachment")
    }

    fn generar_pdf_interno(
        data: crate::service::comprobante_service::ComprobanteData,
        tx: &SyncSender<PdfRequest>,
        fname: &str,
        disp: &str,
    ) -> Response {
        let (tx_r, rx_r) = oneshot::channel();
        if tx
            .send(PdfRequest {
                data,
                responder: tx_r,
            })
            .is_err()
        {
            return Response::text("Generador no disponible").with_status_code(503);
        }
        match rx_r.recv() {
            Ok(Ok(pdf)) => Response::from_data("application/pdf", pdf).with_additional_header(
                "Content-Disposition",
                format!("{}; filename=\"{}\"", disp, fname),
            ),
            _ => Response::text("Error al generar PDF").with_status_code(500),
        }
    }

    fn verificar_admin(req: &Request, conn: &Connection) -> Result<(), Response> {
        let token = extraer_token_sesion(req)
            .ok_or_else(|| Response::text("No autorizado").with_status_code(401))?;
        let s = SesionRepository::buscar_por_token(conn, &token)
            .map_err(|_| Response::text("Error interno").with_status_code(500))?
            .ok_or_else(|| Response::text("Sesión inválida").with_status_code(401))?;
        let u = UsuarioRepository::buscar_por_id(conn, s.id_usuario)
            .map_err(|_| Response::text("Error").with_status_code(500))?
            .unwrap();

        if !u.es_admin() {
            return Err(Response::text("Acceso denegado").with_status_code(403));
        }
        Ok(())
    }

    fn obtener_reservas_detalladas(conn: &Connection) -> Vec<ReservaVista> {
        ReservaRepository::listar_por_estado(conn, "pendiente")
            .unwrap_or_default()
            .into_iter()
            .map(|r| Self::mapear_reserva_admin(conn, r))
            .collect()
    }

    fn mapear_reserva_admin(conn: &Connection, r: crate::models::reserva::Reserva) -> ReservaVista {
        let prof = UsuarioRepository::buscar_por_id(conn, r.id_usuario)
            .map(|op| {
                op.map(|u| format!("{} {}", u.nombre, u.apellido))
                    .unwrap_or_else(|| "Desconocido".into())
            })
            .unwrap_or_else(|_| "Desconocido".into());
        let ini = NaiveDate::parse_from_str(&r.fecha_inicio, "%Y-%m-%d")
            .map(|d| d.format("%d/%m").to_string())
            .unwrap_or_else(|_| r.fecha_inicio.clone());
        let fin = NaiveDate::parse_from_str(&r.fecha_fin, "%Y-%m-%d")
            .map(|d| d.format("%d/%m").to_string())
            .unwrap_or_else(|_| r.fecha_fin.clone());
        let equipos =
            ReservaRepository::obtener_equipos_por_reserva(conn, r.id).unwrap_or_default();

        ReservaVista {
            id: r.id,
            profe_nombre: prof,
            fecha_inicio: ini,
            fecha_fin: fin,
            motivo: r.motivo.unwrap_or_else(|| "Sin motivo".into()),
            categorias: Self::agrupar_equipos(equipos),
        }
    }

    fn agrupar_equipos(
        equipos: Vec<crate::repository::reserva_repository::EquipoRaw>,
    ) -> Vec<CategoriaAgrupada> {
        let mut cats: Vec<CategoriaAgrupada> = Vec::new();
        for eq in equipos {
            let c_str = eq.categoria.clone().unwrap_or_else(|| "Varios".into());
            let iden = if let Some(qr) = eq.codigo_qr.filter(|s| !s.is_empty()) {
                format!("QR: {}", qr)
            } else if let Some(ns) = eq.numero_serie.filter(|s| !s.is_empty()) {
                format!("N/S: {}", ns)
            } else if let Some(pat) = eq.patrimonio.filter(|s| !s.is_empty()) {
                format!("Pat: {}", pat)
            } else {
                format!("ID: {}", eq.ejemplar_id)
            };

            let c_agrup = match cats.iter_mut().find(|c| c.nombre_categoria == c_str) {
                Some(c) => c,
                None => {
                    cats.push(CategoriaAgrupada {
                        nombre_categoria: c_str,
                        modelos: Vec::new(),
                    });
                    cats.last_mut().unwrap()
                }
            };
            let marca = format!("{} {}", eq.marca, eq.nombre_modelo);
            let m_agrup = match c_agrup
                .modelos
                .iter_mut()
                .find(|m| m.modelo_id == eq.modelo_id)
            {
                Some(m) => m,
                None => {
                    c_agrup.modelos.push(ModeloAgrupado {
                        modelo_id: eq.modelo_id,
                        marca_modelo: marca,
                        ejemplares: Vec::new(),
                    });
                    c_agrup.modelos.last_mut().unwrap()
                }
            };
            m_agrup.ejemplares.push(EjemplarVista {
                id: eq.ejemplar_id,
                identificador: iden,
            });
        }
        cats
    }

    fn extraer_filtros_historial(req: &Request) -> FiltrosHistorial {
        FiltrosHistorial {
            docente: req.get_param("docente").unwrap_or_default(),
            estado: req.get_param("estado").unwrap_or_default(),
            fecha_desde: req.get_param("fecha_desde").unwrap_or_default(),
            fecha_hasta: req.get_param("fecha_hasta").unwrap_or_default(),
            motivo: req.get_param("motivo").unwrap_or_default(),
            ordenar_por: req
                .get_param("ordenar_por")
                .unwrap_or_else(|| "momento_creacion".into()),
            direccion: req.get_param("direccion").unwrap_or_else(|| "desc".into()),
            pagina: req
                .get_param("page")
                .unwrap_or_else(|| "1".into())
                .parse()
                .unwrap_or(1),
        }
    }

    fn obtener_historial_reservas_filtrado(
        conn: &Connection,
        f: &FiltrosHistorial,
    ) -> Vec<HistorialReservaVista> {
        let reservas = ReservaRepository::listar_todas(conn).unwrap_or_default();
        let hoy = chrono::Local::now().date_naive();
        let mut res = Vec::new();

        for r in reservas {
            let (prof, leg) = Self::obtener_datos_docente(conn, r.id_usuario);
            let mot = r.motivo.clone().unwrap_or_else(|| "Sin motivo".into());
            if Self::cumple_filtros(&r, &prof, f, &mot) {
                res.push(Self::mapear_historial_reserva(r, prof, leg, mot, hoy));
            }
        }
        Self::ordenar_historial(&mut res, &f.ordenar_por, &f.direccion);
        res
    }

    fn obtener_datos_docente(conn: &Connection, id: i64) -> (String, i32) {
        match UsuarioRepository::buscar_por_id(conn, id) {
            Ok(Some(u)) => (format!("{} {}", u.nombre, u.apellido), u.legajo),
            _ => ("Usuario desconocido".into(), 0),
        }
    }

    fn cumple_filtros(
        r: &crate::models::reserva::Reserva,
        p: &str,
        f: &FiltrosHistorial,
        mr: &str,
    ) -> bool {
        if !f.docente.is_empty() && !p.to_lowercase().contains(&f.docente.to_lowercase()) {
            return false;
        }
        if !f.estado.is_empty() && r.estado != f.estado {
            return false;
        }
        if !f.fecha_desde.is_empty() && r.fecha_inicio < f.fecha_desde {
            return false;
        }
        if !f.fecha_hasta.is_empty() && r.fecha_fin > f.fecha_hasta {
            return false;
        }
        if !f.motivo.is_empty() && !mr.to_lowercase().contains(&f.motivo.to_lowercase()) {
            return false;
        }
        true
    }

    fn mapear_historial_reserva(
        r: crate::models::reserva::Reserva,
        p: String,
        l: i32,
        m: String,
        hoy: NaiveDate,
    ) -> HistorialReservaVista {
        let (txt_est, cls_est) = Self::obtener_estado_historial(&r, hoy);
        let conf = r.momento_confirmacion.unwrap_or_else(|| "---".into());

        let inicio = NaiveDate::parse_from_str(&r.fecha_inicio, "%Y-%m-%d").unwrap();
        let fin = NaiveDate::parse_from_str(&r.fecha_fin, "%Y-%m-%d").unwrap();

        HistorialReservaVista {
            id: r.id,
            profesor: p,
            legajo: l.into(),
            fecha_inicio: r.fecha_inicio,
            fecha_fin: r.fecha_fin,
            dias: (fin - inicio).num_days() + 1,
            estado: r.estado,
            texto_estado: txt_est,
            clase_estado: cls_est,
            motivo: m,
            momento_creacion: utc_str_a_arg(&r.momento_creacion),
            momento_confirmacion: utc_str_a_arg(&conf),
        }
    }

    fn obtener_estado_historial(
        r: &crate::models::reserva::Reserva,
        hoy: NaiveDate,
    ) -> (String, String) {
        if r.estado == "activa" {
            if let Ok(inicio) = NaiveDate::parse_from_str(&r.fecha_inicio, "%Y-%m-%d")
                && hoy < inicio
            {
                return ("Aprobada".into(), "estado-aprobada".into());
            }
            return ("En Curso".into(), "estado-en-curso".into());
        }
        let txt = match r.estado.as_str() {
            "pendiente" => "Pendiente",
            "cancelada" => "Cancelada",
            "concluida" => "Concluida",
            e => e,
        };
        (txt.into(), format!("estado-{}", r.estado))
    }

    fn ordenar_historial(r: &mut [HistorialReservaVista], ord: &str, dir: &str) {
        match ord {
            "docente" => {
                r.sort_by(|a, b| a.profesor.to_lowercase().cmp(&b.profesor.to_lowercase()))
            }
            "fecha_inicio" => r.sort_by(|a, b| a.fecha_inicio.cmp(&b.fecha_inicio)),
            "fecha_fin" => r.sort_by(|a, b| a.fecha_fin.cmp(&b.fecha_fin)),
            "estado" => r.sort_by(|a, b| a.estado.cmp(&b.estado)),
            _ => r.sort_by(|a, b| a.momento_creacion.cmp(&b.momento_creacion)),
        }
        if dir == "desc" {
            r.reverse();
        }
    }

    fn renderizar_historial(
        req: &Request,
        c: &Connection,
        r: Vec<HistorialReservaVista>,
        f: &FiltrosHistorial,
        p: usize,
        t: usize,
    ) -> Response {
        let mut ctx = Context::new();
        if let Ok(u) = crate::utils::usuario_actual(req, c) {
            ctx.insert("usuario_actual", &u);
        }
        ctx.insert("reservas", &r);
        ctx.insert("filtro_docente", &f.docente);
        ctx.insert("filtro_estado", &f.estado);
        ctx.insert("filtro_fecha_desde", &f.fecha_desde);
        ctx.insert("filtro_fecha_hasta", &f.fecha_hasta);
        ctx.insert("filtro_motivo", &f.motivo);
        ctx.insert("ordenar_por", &f.ordenar_por);
        ctx.insert("direccion", &f.direccion);
        ctx.insert("pagina_actual", &p);
        ctx.insert("tiene_anterior", &(p > 1));
        ctx.insert("tiene_siguiente", &(p < t));
        ctx.insert("pagina_anterior", &(p - 1));
        ctx.insert("pagina_siguiente", &(p + 1));
        ctx.insert("total_paginas", &t);
        templates::response_html(templates::render("admin_historial_reservas.html", &ctx))
    }

    fn extraer_id_rol_body(req: &Request) -> Result<(i64, String), Response> {
        let mut body = String::new();
        if let Some(mut r) = req.data() {
            let _ = r.read_to_string(&mut body);
        }
        let d = parsear_formulario(&body);
        let id = d
            .get("id_usuario")
            .unwrap_or(&String::new())
            .parse::<i64>()
            .map_err(|_| templates::response_mensaje_error("ID inválido", "Formato numérico."))?;
        Ok((id, d.get("tipo").cloned().unwrap_or_default()))
    }

    /*
    pub fn procesar_envio_invitacion(request: &Request, conn: &Connection) -> Response {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = parsear_formulario(&body);
        let email = datos_parseados.get("email").cloned().unwrap_or_default();
        let tipo = datos_parseados.get("tipo").cloned().unwrap_or_default();

        if email.is_empty() || tipo.is_empty() {
            return templates::response_mensaje_error(
                "Campos faltantes",
                "Por favor ingrese el correo electrónico institucional y seleccione el rol.",
            );
        }

        match AuthService::invitar_usuario(conn, &email, &tipo) {
            Ok(_) => templates::response_mensaje_exito(
                "Invitación enviada",
                &format!(
                    "Se ha enviado un enlace de alta a la casilla institucional: {}",
                    email
                ),
            ),
            Err(e) => templates::response_mensaje_error("Error al procesar invitación", &e),
        }
    }
    */
}
