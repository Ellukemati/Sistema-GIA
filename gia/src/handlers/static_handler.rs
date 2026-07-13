use rouille::{Request, Response};
use rusqlite::Connection;
use tera::Context;

use crate::templates;
use crate::utils::usuario_actual;

pub struct StaticHandler;

impl StaticHandler {
    pub fn mostrar_creditos(request: &Request, conn: &Connection) -> Response {
        let usuario_opt = usuario_actual(request, conn).ok();

        let mut ctx = Context::new();

        if let Some(ref usuario) = usuario_opt {
            ctx.insert("usuario_actual", usuario);
        }

        templates::response_html(templates::render("creditos.html", &ctx))
    }
}
