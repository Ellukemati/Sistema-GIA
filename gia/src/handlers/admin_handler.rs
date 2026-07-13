use chrono::NaiveDate;
use rouille::{Request, Response};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Read;
use std::sync::mpsc::SyncSender;
use tera::Context;

use crate::repository::{
    reserva_repository::ReservaRepository, sesion_repository::SesionRepository,
    usuario_repository::UsuarioRepository,
};
use crate::service::{
    auth_service::AuthService, mail_service::MailService, pdf_worker_service::PdfRequest,
};
use crate::templates;
use crate::utils::extraer_token_sesion;

pub struct AdminHandler;

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

impl AdminHandler {
    fn verificar_admin(request: &Request, conn: &Connection) -> Result<(), Response> {
        let token = extraer_token_sesion(request)
            .ok_or_else(|| Response::text("No autorizado").with_status_code(401))?;
        let sesion = SesionRepository::buscar_por_token(conn, &token)
            .map_err(|_| Response::text("Error interno").with_status_code(500))?
            .ok_or_else(|| Response::text("Sesión inválida").with_status_code(401))?;
        let usuario = UsuarioRepository::buscar_por_id(conn, sesion.id_usuario)
            .map_err(|_| Response::text("Error interno").with_status_code(500))?
            .unwrap();

        if !usuario.es_admin() {
            return Err(Response::text("Acceso denegado").with_status_code(403));
        }
        Ok(())
    }

    fn obtener_reservas_detalladas(conn: &Connection) -> Vec<ReservaVista> {
        let reservas_db =
            ReservaRepository::listar_por_estado(conn, "pendiente").unwrap_or_default();
        let mut reservas_vista = Vec::new();

        for r in reservas_db {
            let profe_nombre = match UsuarioRepository::buscar_por_id(conn, r.id_usuario) {
                Ok(Some(u)) => format!("{} {}", u.nombre, u.apellido),
                _ => "Usuario Desconocido".to_string(),
            };

            let inicio_fmt = NaiveDate::parse_from_str(&r.fecha_inicio, "%Y-%m-%d")
                .map(|d| d.format("%d/%m").to_string())
                .unwrap_or_else(|_| r.fecha_inicio.clone());

            let fin_fmt = NaiveDate::parse_from_str(&r.fecha_fin, "%Y-%m-%d")
                .map(|d| d.format("%d/%m").to_string())
                .unwrap_or_else(|_| r.fecha_fin.clone());

            let equipos_raw =
                ReservaRepository::obtener_equipos_por_reserva(conn, r.id).unwrap_or_default();
            let mut categorias_agrupadas: Vec<CategoriaAgrupada> = Vec::new();

            for item in equipos_raw {
                let cat_str = item
                    .categoria
                    .unwrap_or_else(|| "Instrumentos Varios".to_string());
                let marca_mod = format!("{} {}", item.marca, item.nombre_modelo);

                let identificador = if let Some(qr) = item.codigo_qr.filter(|s| !s.is_empty()) {
                    format!("QR: {}", qr)
                } else if let Some(ns) = item.numero_serie.filter(|s| !s.is_empty()) {
                    format!("N/S: {}", ns)
                } else if let Some(pat) = item.patrimonio.filter(|s| !s.is_empty()) {
                    format!("Pat: {}", pat)
                } else {
                    format!("ID Interno: {}", item.ejemplar_id)
                };

                let ejemplar_vista = EjemplarVista {
                    id: item.ejemplar_id,
                    identificador,
                };

                let cat_agrupada = match categorias_agrupadas
                    .iter_mut()
                    .find(|c| c.nombre_categoria == cat_str)
                {
                    Some(c) => c,
                    None => {
                        categorias_agrupadas.push(CategoriaAgrupada {
                            nombre_categoria: cat_str.clone(),
                            modelos: Vec::new(),
                        });
                        categorias_agrupadas.last_mut().unwrap()
                    }
                };

                let mod_agrupado = match cat_agrupada
                    .modelos
                    .iter_mut()
                    .find(|m| m.modelo_id == item.modelo_id)
                {
                    Some(m) => m,
                    None => {
                        cat_agrupada.modelos.push(ModeloAgrupado {
                            modelo_id: item.modelo_id,
                            marca_modelo: marca_mod,
                            ejemplares: Vec::new(),
                        });
                        cat_agrupada.modelos.last_mut().unwrap()
                    }
                };

                mod_agrupado.ejemplares.push(ejemplar_vista);
            }

            reservas_vista.push(ReservaVista {
                id: r.id,
                profe_nombre,
                fecha_inicio: inicio_fmt,
                fecha_fin: fin_fmt,
                motivo: r
                    .motivo
                    .unwrap_or_else(|| "Sin motivo especificado".to_string()),
                categorias: categorias_agrupadas,
            });
        }
        reservas_vista
    }

    pub fn mostrar_dashboard(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let mut ctx = Context::new();

        if let Some(token) = extraer_token_sesion(request)
            && let Ok(Some(sesion)) = SesionRepository::buscar_por_token(conn, &token)
            && let Ok(Some(usuario)) = UsuarioRepository::buscar_por_id(conn, sesion.id_usuario)
        {
            ctx.insert("usuario_actual", &usuario);
        }

        let reservas = Self::obtener_reservas_detalladas(conn);
        let profes = UsuarioRepository::listar_profesores_pendientes(conn).unwrap_or_default();
        let docentes_aprobados =
            UsuarioRepository::listar_docentes_aprobados(conn).unwrap_or_default();
        let administradores = UsuarioRepository::listar_administradores(conn).unwrap_or_default();

        ctx.insert("reservas", &reservas);
        ctx.insert("profesores", &profes);
        ctx.insert("docentes_aprobados", &docentes_aprobados);
        ctx.insert("administradores", &administradores);
        templates::response_html(templates::render("admin_dashboard.html", &ctx))
    }

    pub fn recargar_tablas_htmx(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let reservas = Self::obtener_reservas_detalladas(conn);
        let profes = UsuarioRepository::listar_profesores_pendientes(conn).unwrap_or_default();
        let tab_activo = request
            .get_param("tab_activo")
            .unwrap_or_else(|| "0".to_string());
        let docentes_aprobados =
            UsuarioRepository::listar_docentes_aprobados(conn).unwrap_or_default();
        let administradores = UsuarioRepository::listar_administradores(conn).unwrap_or_default();

        let mut ctx = Context::new();
        ctx.insert("reservas", &reservas);
        ctx.insert("profesores", &profes);
        ctx.insert("docentes_aprobados", &docentes_aprobados);
        ctx.insert("administradores", &administradores);
        ctx.insert("tab_activo", &tab_activo);
        templates::response_html(templates::render("partials/admin_tablas.html", &ctx))
    }

    pub fn previsualizar_comprobante(
        request: &Request,
        conn: &Connection,
        id: i64,
        pdf_tx: &SyncSender<PdfRequest>,
    ) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let data_comprobante =
            match crate::service::reserva_service::ReservaService::preparar_datos_previsualizacion(
                conn, id,
            ) {
                Ok(d) => d,
                Err(e) => {
                    return Response::text(format!("Error simulando comprobante: {}", e))
                        .with_status_code(500);
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
            return Response::text("Generador de PDF no disponible").with_status_code(503);
        }

        match rx_respuesta.recv() {
            Ok(Ok(pdf_bytes)) => Response::from_data("application/pdf", pdf_bytes)
                .with_additional_header(
                    "Content-Disposition",
                    "inline; filename=previsualizacion.pdf",
                ),
            _ => Response::text("Error en el motor worker de PDF").with_status_code(500),
        }
    }

    pub fn descargar_comprobante_admin(
        request: &Request,
        conn: &Connection,
        id: i64,
        pdf_tx: &SyncSender<PdfRequest>,
    ) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let data_comprobante =
            match crate::service::reserva_service::ReservaService::preparar_datos_comprobante(
                conn, id,
            ) {
                Ok(d) => d,
                Err(e) => return Response::text(format!("Error: {:?}", e)).with_status_code(400),
            };

        let (tx_respuesta, rx_respuesta) = oneshot::channel();
        if pdf_tx
            .send(PdfRequest {
                data: data_comprobante,
                responder: tx_respuesta,
            })
            .is_err()
        {
            return Response::text("Generador no disponible").with_status_code(503);
        }

        match rx_respuesta.recv() {
            Ok(Ok(pdf_bytes)) => Response::from_data("application/pdf", pdf_bytes)
                .with_additional_header(
                    "Content-Disposition",
                    format!("attachment; filename=comprobante_{}.pdf", id),
                ),
            _ => Response::text("Error al generar PDF").with_status_code(500),
        }
    }

    pub fn aprobar_reserva(
        request: &Request,
        conn: &Connection,
        id: i64,
        pdf_tx: &SyncSender<PdfRequest>,
    ) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let admin = match crate::utils::usuario_actual(request, conn) {
            Ok(u) => u,
            Err(resp) => return resp,
        };

        match crate::service::reserva_service::ReservaService::aprobar_reserva(
            conn, id, admin.id, pdf_tx,
        ) {
            Ok(_) => Response::html("")
                .with_additional_header("HX-Trigger", format!("{{\"abrirComprobante\": {}}}", id)),
            Err(ref e) => templates::response_mensaje_error("No se pudo aprobar la reserva", e),
        }
    }

    pub fn rechazar_reserva(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        if let Ok(Some(reserva)) = ReservaRepository::buscar_por_id(conn, id)
            && let Ok(Some(u)) = UsuarioRepository::buscar_por_id(conn, reserva.id_usuario)
        {
            let _ = ReservaRepository::cambiar_estado(conn, id, "cancelada");

            let nombre_completo = format!("{} {}", u.nombre, u.apellido);
            let motivo = reserva
                .motivo
                .unwrap_or_else(|| "Uso de instrumental".to_string());

            let _ = MailService::enviar_notificacion_reserva_rechazada(
                &u.email,
                &nombre_completo,
                &id.to_string(),
                &motivo,
            );
        }
        Response::html("")
    }

    pub fn aprobar_profesor(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        if let Ok(Some(u)) = UsuarioRepository::buscar_por_id(conn, id) {
            let _ = UsuarioRepository::aprobar_profesor(conn, id);
            let nombre_completo = format!("{} {}", u.nombre, u.apellido);
            let _ = MailService::enviar_notificacion_profesor_aprobado(&u.email, &nombre_completo);
        }
        Response::html("")
    }

    pub fn rechazar_profesor(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        if let Ok(Some(u)) = UsuarioRepository::buscar_por_id(conn, id) {
            let nombre_completo = format!("{} {}", u.nombre, u.apellido);
            let _ = MailService::enviar_notificacion_profesor_rechazado(&u.email, &nombre_completo);
            let _ = UsuarioRepository::eliminar(conn, id);
        }
        Response::html("")
    }

    pub fn hacer_admin(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }
        let _ = UsuarioRepository::hacer_admin(conn, id);
        Response::html("")
    }

    pub fn quitar_admin(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let cantidad_admins = UsuarioRepository::listar_administradores(conn)
            .map(|lista| lista.len())
            .unwrap_or(0);

        if cantidad_admins <= 1 {
            return Response::text("Debe existir al menos un administrador en el sistema.")
                .with_status_code(400);
        }

        let _ = UsuarioRepository::quitar_admin(conn, id);
        Response::html("")
    }

    pub fn procesar_envio_invitacion(request: &Request, conn: &Connection) -> Response {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);
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

    pub fn procesar_cambio_rol(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);
        let nuevo_tipo = datos_parseados.get("tipo").cloned().unwrap_or_default();

        let id_usuario = match datos_parseados
            .get("id_usuario")
            .unwrap_or(&String::new())
            .parse::<i64>()
        {
            Ok(val) => val,
            Err(_) => {
                return templates::response_mensaje_error(
                    "Identificador inválido",
                    "El ID de usuario provisto no tiene un formato numérico correcto.",
                );
            }
        };

        match crate::repository::usuario_repository::UsuarioRepository::actualizar_rol(
            conn,
            id_usuario,
            &nuevo_tipo,
        ) {
            Ok(_) => templates::response_mensaje_exito(
                "Rol actualizado",
                &format!(
                    "El usuario ha sido configurado con el rol '{}' exitosamente.",
                    nuevo_tipo
                ),
            ),
            Err(e) => templates::response_mensaje_error("Error de asignación", &e.to_string()),
        }
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

    #[allow(clippy::too_many_arguments)]
    fn obtener_historial_reservas_filtrado(
        conn: &Connection,
        docente: &str,
        estado: &str,
        fecha_desde: &str,
        fecha_hasta: &str,
        motivo: &str,
        ordenar_por: &str,
        direccion: &str,
    ) -> Vec<HistorialReservaVista> {
        let reservas = ReservaRepository::listar_todas(conn).unwrap_or_default();
        let mut resultado = Vec::new();
        let ahora_local = chrono::Local::now().date_naive();

        for r in reservas {
            let (profesor, legajo) = match UsuarioRepository::buscar_por_id(conn, r.id_usuario) {
                Ok(Some(u)) => (format!("{} {}", u.nombre, u.apellido), u.legajo),
                _ => ("Usuario desconocido".to_string(), 0),
            };

            if !docente.is_empty() && !profesor.to_lowercase().contains(&docente.to_lowercase()) {
                continue;
            }

            if !estado.is_empty() && r.estado != estado {
                continue;
            }

            if !fecha_desde.is_empty() && r.fecha_inicio.as_str() < fecha_desde {
                continue;
            }

            if !fecha_hasta.is_empty() && r.fecha_fin.as_str() > fecha_hasta {
                continue;
            }

            let motivo_reserva = r.motivo.clone().unwrap_or_else(|| "Sin motivo".to_string());
            if !motivo.is_empty()
                && !motivo_reserva
                    .to_lowercase()
                    .contains(&motivo.to_lowercase())
            {
                continue;
            }

            let inicio_parsed = NaiveDate::parse_from_str(&r.fecha_inicio, "%Y-%m-%d");
            let fin_parsed = NaiveDate::parse_from_str(&r.fecha_fin, "%Y-%m-%d");

            let mut texto_estado = r.estado.clone();
            let mut clase_estado = format!("estado-{}", r.estado);

            if r.estado == "activa" {
                if let (Ok(ini), Ok(fin)) = (inicio_parsed, fin_parsed) {
                    if ahora_local < ini {
                        texto_estado = "Aprobada".to_string();
                        clase_estado = "estado-aprobada".to_string();
                    } else if ahora_local >= ini && ahora_local <= fin {
                        texto_estado = "En Curso".to_string();
                        clase_estado = "estado-en-curso".to_string();
                    } else {
                        texto_estado = "Concluida".to_string();
                        clase_estado = "estado-concluida".to_string();
                    }
                }
            } else if r.estado == "pendiente" {
                texto_estado = "Pendiente".to_string();
            } else if r.estado == "cancelada" {
                texto_estado = "Cancelada".to_string();
            } else if r.estado == "concluida" {
                texto_estado = "Concluida".to_string();
            }

            let confirmacion_txt = r
                .momento_confirmacion
                .clone()
                .unwrap_or_else(|| "---".to_string());

            resultado.push(HistorialReservaVista {
                id: r.id,
                profesor,
                legajo: legajo.into(),
                fecha_inicio: r.fecha_inicio,
                fecha_fin: r.fecha_fin,
                estado: r.estado,
                texto_estado,
                clase_estado,
                motivo: motivo_reserva,
                momento_creacion: r.momento_creacion,
                momento_confirmacion: confirmacion_txt,
            });
        }

        match ordenar_por {
            "docente" => {
                resultado.sort_by(|a, b| a.profesor.to_lowercase().cmp(&b.profesor.to_lowercase()));
            }
            "fecha_inicio" => {
                resultado.sort_by(|a, b| a.fecha_inicio.cmp(&b.fecha_inicio));
            }
            "fecha_fin" => {
                resultado.sort_by(|a, b| a.fecha_fin.cmp(&b.fecha_fin));
            }
            "estado" => {
                resultado.sort_by(|a, b| a.estado.cmp(&b.estado));
            }
            _ => {
                resultado.sort_by(|a, b| a.momento_creacion.cmp(&b.momento_creacion));
            }
        }

        if direccion == "desc" {
            resultado.reverse();
        }
        resultado
    }

    pub fn mostrar_historial_reservas(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let docente = request.get_param("docente").unwrap_or_default();
        let estado = request.get_param("estado").unwrap_or_default();
        let fecha_desde = request.get_param("fecha_desde").unwrap_or_default();
        let fecha_hasta = request.get_param("fecha_hasta").unwrap_or_default();
        let motivo = request.get_param("motivo").unwrap_or_default();

        let ordenar_por = request
            .get_param("ordenar_por")
            .unwrap_or_else(|| "momento_creacion".to_string());

        let direccion = request
            .get_param("direccion")
            .unwrap_or_else(|| "desc".to_string());

        let pagina = request
            .get_param("page")
            .unwrap_or_else(|| "1".to_string())
            .parse::<usize>()
            .unwrap_or(1);

        let por_pagina = 20usize;

        let mut reservas = Self::obtener_historial_reservas_filtrado(
            conn,
            &docente,
            &estado,
            &fecha_desde,
            &fecha_hasta,
            &motivo,
            &ordenar_por,
            &direccion,
        );

        let total_reservas = reservas.len();

        let total_paginas = if total_reservas == 0 {
            1
        } else {
            ((total_reservas as f64) / (por_pagina as f64)).ceil() as usize
        };

        let inicio = (pagina - 1) * por_pagina;

        reservas = reservas.into_iter().skip(inicio).take(por_pagina).collect();

        let mut ctx = Context::new();

        if let Some(token) = extraer_token_sesion(request)
            && let Ok(Some(sesion)) = SesionRepository::buscar_por_token(conn, &token)
            && let Ok(Some(usuario)) = UsuarioRepository::buscar_por_id(conn, sesion.id_usuario)
        {
            ctx.insert("usuario_actual", &usuario);
        }

        ctx.insert("reservas", &reservas);

        ctx.insert("filtro_docente", &docente);
        ctx.insert("filtro_estado", &estado);
        ctx.insert("filtro_fecha_desde", &fecha_desde);
        ctx.insert("filtro_fecha_hasta", &fecha_hasta);
        ctx.insert("filtro_motivo", &motivo);

        ctx.insert("ordenar_por", &ordenar_por);
        ctx.insert("direccion", &direccion);

        ctx.insert("pagina_actual", &pagina);

        let tiene_anterior = pagina > 1;
        let tiene_siguiente = pagina < total_paginas;

        ctx.insert("tiene_anterior", &tiene_anterior);
        ctx.insert("tiene_siguiente", &tiene_siguiente);

        ctx.insert("pagina_anterior", &(pagina - 1));
        ctx.insert("pagina_siguiente", &(pagina + 1));

        ctx.insert("total_paginas", &total_paginas);

        templates::response_html(templates::render("admin_historial_reservas.html", &ctx))
    }

    pub fn exportar_historial_csv(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let docente = request.get_param("docente").unwrap_or_default();
        let estado = request.get_param("estado").unwrap_or_default();
        let fecha_desde = request.get_param("fecha_desde").unwrap_or_default();
        let fecha_hasta = request.get_param("fecha_hasta").unwrap_or_default();
        let motivo = request.get_param("motivo").unwrap_or_default();
        let ordenar_por = request
            .get_param("ordenar_por")
            .unwrap_or_else(|| "momento_creacion".to_string());
        let direccion = request
            .get_param("direccion")
            .unwrap_or_else(|| "desc".to_string());

        let reservas = Self::obtener_historial_reservas_filtrado(
            conn,
            &docente,
            &estado,
            &fecha_desde,
            &fecha_hasta,
            &motivo,
            &ordenar_por,
            &direccion,
        );

        let mut csv = String::new();
        csv.push_str(
            "ID,Docente,Estado Real,Fecha Inicio,Fecha Fin,Motivo,Creada,Firma Auditoria\n",
        );

        for r in reservas {
            let fila = format!(
                "{},{},{},{},{},{},{},{}\n",
                r.id,
                r.profesor.replace(",", " "),
                r.texto_estado,
                r.fecha_inicio,
                r.fecha_fin,
                r.motivo.replace(",", " "),
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
}
