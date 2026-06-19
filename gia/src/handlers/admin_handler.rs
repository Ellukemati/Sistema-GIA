use crate::repository::{
    reserva_repository::ReservaRepository, sesion_repository::SesionRepository,
    usuario_repository::UsuarioRepository,
};
use crate::templates;
use crate::utils::extraer_token_sesion;
use rouille::{Request, Response};
use rusqlite::Connection;
use serde::Serialize;
use tera::Context;

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
        let _ = ReservaRepository::cambiar_estado(conn, id, "activa");
        // ACÁ: disparar mail con el comprobante al profesor
        Response::html("")
    }

    pub fn rechazar_reserva(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }
        let _ = ReservaRepository::cambiar_estado(conn, id, "cancelada");
        // ACÁ: disparar mail de rechazo
        Response::html("")
    }

    pub fn aprobar_profesor(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }
        let _ = UsuarioRepository::aprobar_profesor(conn, id);
        // ACÁ: disparar mail de bienvenida y habilitación
        Response::html("")
    }

    pub fn rechazar_profesor(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) {
            return resp;
        }
        // Eliminar el registro para que no ocupe lugar en la BDD
        let _ = UsuarioRepository::eliminar(conn, id);
        Response::html("")
    }
}
