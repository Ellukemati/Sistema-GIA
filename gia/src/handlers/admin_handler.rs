use crate::repository::{
    reserva_repository::ReservaRepository, sesion_repository::SesionRepository,
    usuario_repository::UsuarioRepository,
};
use crate::templates;
use crate::utils::extraer_token_sesion;
use rouille::{Request, Response};
use rusqlite::Connection;
use tera::Context;

pub struct AdminHandler;

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

    pub fn mostrar_dashboard(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) { return resp; }
        
        let mut ctx = Context::new();
        let reservas = ReservaRepository::listar_por_estado(conn, "pendiente").unwrap_or_default();
        let profes = UsuarioRepository::listar_profesores_pendientes(conn).unwrap_or_default();
        
        ctx.insert("reservas", &reservas);
        ctx.insert("profesores", &profes);
        templates::response_html(templates::render("admin_dashboard.html", &ctx))
    }

    pub fn recargar_tablas_htmx(request: &Request, conn: &Connection) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) { return resp; }
        
        let mut ctx = Context::new();
        let reservas = ReservaRepository::listar_por_estado(conn, "pendiente").unwrap_or_default();
        let profes = UsuarioRepository::listar_profesores_pendientes(conn).unwrap_or_default();
        
        ctx.insert("reservas", &reservas);
        ctx.insert("profesores", &profes);
        templates::response_html(templates::render("partials/admin_tablas.html", &ctx))
    }

    pub fn aprobar_reserva(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) { return resp; }
        let _ = ReservaRepository::cambiar_estado(conn, id, "activa");
        // ACÁ: disparar mail con el comprobante al profesor
        Response::empty_204()
    }

    pub fn rechazar_reserva(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) { return resp; }
        let _ = ReservaRepository::cambiar_estado(conn, id, "cancelada");
        // ACÁ: disparar mail de rechazo
        Response::empty_204()
    }

    pub fn aprobar_profesor(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) { return resp; }
        let _ = UsuarioRepository::aprobar_profesor(conn, id);
        // ACÁ: disparar mail de bienvenida y habilitación
        Response::empty_204()
    }

    pub fn rechazar_profesor(request: &Request, conn: &Connection, id: i64) -> Response {
        if let Err(resp) = Self::verificar_admin(request, conn) { return resp; }
        // Eliminar el registro para que no ocupe lugar en la BDD
        let _ = UsuarioRepository::eliminar(conn, id);
        Response::empty_204()
    }
}