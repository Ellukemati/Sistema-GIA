use rouille::{Request, Response};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Read;
use tera::Context;

use crate::repository::{
    reserva_repository::ReservaRepository, sesion_repository::SesionRepository,
    usuario_repository::UsuarioRepository,
};
use crate::service::mail_service::MailService;
use crate::templates;
use crate::utils::extraer_token_sesion;

pub struct AdminHandler;

#[derive(Serialize)]
struct ReservaVista {
    pub id: i64,
    pub profe_nombre: String,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub motivo: String,
    pub equipos: Vec<String>,
}

impl AdminHandler {
    /// Valida que el usuario tenga sesión activa y sea Administrador
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
            // Buscar nombre del profesor
            let profe_nombre = match UsuarioRepository::buscar_por_id(conn, r.id_usuario) {
                Ok(Some(u)) => format!("{} {}", u.nombre, u.apellido),
                _ => "Usuario Desconocido".to_string(),
            };

            // Buscar los instrumentos de esta reserva
            let mut stmt = conn
                .prepare(
                    "SELECT m.nombre_modelo, e.patrimonio 
                 FROM reserva_ejemplar re
                 JOIN ejemplares e ON re.ejemplar_id = e.id
                 JOIN modelos m ON e.modelo_id = m.id
                 WHERE re.reserva_id = ?1",
                )
                .unwrap();

            let equipos_iter = stmt
                .query_map([r.id], |row| {
                    let nombre: String = row.get(0)?;
                    let patrimonio: String = row.get(1)?;
                    Ok(format!("{} (Pat: {})", nombre, patrimonio))
                })
                .unwrap();

            let mut equipos = Vec::new();
            for texto in equipos_iter.flatten() {
                equipos.push(texto);
            }

            reservas_vista.push(ReservaVista {
                id: r.id,
                profe_nombre,
                fecha_inicio: r.fecha_inicio,
                fecha_fin: r.fecha_fin,
                motivo: r
                    .motivo
                    .unwrap_or_else(|| "Sin motivo especificado".to_string()),
                equipos,
            });
        }
        reservas_vista
    }

    pub fn mostrar_dashboard(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let mut ctx = Context::new();
        let reservas = Self::obtener_reservas_detalladas(conn);
        let profes = UsuarioRepository::listar_profesores_pendientes(conn).unwrap_or_default();

        ctx.insert("reservas", &reservas);
        ctx.insert("profesores", &profes);
        templates::response_html(templates::render("admin_dashboard.html", &ctx))
    }

    pub fn recargar_tablas_htmx(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        let mut ctx = Context::new();
        let reservas = Self::obtener_reservas_detalladas(conn);
        let profes = UsuarioRepository::listar_profesores_pendientes(conn).unwrap_or_default();

        ctx.insert("reservas", &reservas);
        ctx.insert("profesores", &profes);
        templates::response_html(templates::render("partials/admin_tablas.html", &ctx))
    }

    pub fn aprobar_reserva(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        if let Ok(Some(reserva)) = ReservaRepository::buscar_por_id(conn, id)
            && let Ok(Some(u)) = UsuarioRepository::buscar_por_id(conn, reserva.id_usuario)
        {
            let _ = ReservaRepository::cambiar_estado(conn, id, "activa");

            let nombre_completo = format!("{} {}", u.nombre, u.apellido);
            let motivo = reserva
                .motivo
                .unwrap_or_else(|| "Uso de instrumental".to_string());
            let _ = MailService::enviar_notificacion_reserva_aprobada(
                &u.email,
                &nombre_completo,
                &id.to_string(),
                &motivo,
            );
        }

        Response::html("")
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

    // IDEA: Endpoint para enviar un comunicado general por mail a todos los usuarios, a un grupo específico o uno solo
    // Para implementarlo en el front ver bien cómo recibe los parámetros
    pub fn enviar_notificacion_admin(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }

        // Lee el cuerpo en crudo del formulario enviado por HTMX
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        // Parseamos las variables comunes usando un mapeo simple
        let datos_form = Self::parsear_formulario(&body);
        let asunto = datos_form.get("asunto").cloned().unwrap_or_default();
        let mensaje = datos_form.get("mensaje").cloned().unwrap_or_default();

        if asunto.is_empty() || mensaje.is_empty() {
            return templates::response_mensaje_error(
                "Campos obligatorios",
                "El asunto y el mensaje del comunicado son obligatorios.",
            );
        }

        let mut ids_seleccionados = Vec::new();
        for par in body.split('&') {
            if let Some(id_str) = par.strip_prefix("usuarios_ids=")
                && let Ok(id) = id_str.parse::<i64>()
            {
                ids_seleccionados.push(id);
            }
        }

        if ids_seleccionados.is_empty() {
            return templates::response_mensaje_error(
                "No se pudo enviar",
                "Debe seleccionar al menos un usuario para enviar el comunicado.",
            );
        }

        let mut lote_destinatarios = Vec::new();
        for id in &ids_seleccionados {
            if let Ok(Some(u)) = UsuarioRepository::buscar_por_id(conn, *id) {
                let nombre_completo = format!("{} {}", u.nombre, u.apellido);
                lote_destinatarios.push((nombre_completo, u.email));
            }
        }

        let total_intentos = lote_destinatarios.len();
        let mut cantidad_enviados = 0;

        if !lote_destinatarios.is_empty() {
            match MailService::enviar_comunicado_lote(&lote_destinatarios, &asunto, &mensaje) {
                Ok(exitos) => {
                    cantidad_enviados = exitos;
                }
                Err(e) => {
                    return templates::response_mensaje_error(
                        "Error de correo",
                        &format!("Ocurrió un problema con el servidor SMTP: {}", e),
                    );
                }
            }
        }

        if cantidad_enviados == 0 {
            return templates::response_mensaje_error(
                "Envío fallido",
                "No se pudo entregar el mensaje a ninguno de los usuarios seleccionados.",
            );
        }

        let fallidos = total_intentos - cantidad_enviados;

        let texto_resultado = if fallidos > 0 {
            format!(
                "El mensaje se envió correctamente a {} usuarios. Sin embargo, hubo un error con {} destinatario/s.",
                cantidad_enviados, fallidos
            )
        } else {
            format!(
                "El mensaje se despachó correctamente a los {} usuarios seleccionados.",
                cantidad_enviados
            )
        };

        templates::response_mensaje_exito("Comunicado procesado", &texto_resultado)
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
}
