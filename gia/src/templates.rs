use std::sync::OnceLock;

use rouille::Response;
use tera::{Context, Tera};

static ENGINE: OnceLock<Tera> = OnceLock::new();

fn engine() -> &'static Tera {
    ENGINE.get_or_init(|| {
        Tera::new("templates/**/*.html").expect("No se pudieron cargar las plantillas")
    })
}

pub fn render(template: &str, context: &Context) -> Result<String, tera::Error> {
    let mut ctx_global = context.clone();

    ctx_global.insert("logo_path", crate::constants::PATH_LOGO_GIA_TRANSPARENTE);

    engine().render(template, &ctx_global)
}

pub fn render_mensaje_exito(titulo: &str, mensaje: &str) -> Result<String, tera::Error> {
    let mut ctx = Context::new();
    ctx.insert("titulo", titulo);
    ctx.insert("mensaje", mensaje);
    render("partials/mensaje_exito.html", &ctx)
}

pub fn render_mensaje_error(titulo: &str, mensaje: &str) -> Result<String, tera::Error> {
    let mut ctx = Context::new();
    ctx.insert("titulo", titulo);
    ctx.insert("mensaje", mensaje);
    render("partials/mensaje_error.html", &ctx)
}

pub fn response_html(result: Result<String, tera::Error>) -> Response {
    match result {
        Ok(html) => Response::html(html),
        Err(e) => {
            Response::text(format!("Error renderizando plantilla: {}", e)).with_status_code(500)
        }
    }
}

pub fn response_mensaje_exito(titulo: &str, mensaje: &str) -> Response {
    response_html(render_mensaje_exito(titulo, mensaje))
}

pub fn response_mensaje_error(titulo: &str, mensaje: &str) -> Response {
    response_html(render_mensaje_error(titulo, mensaje))
}

pub fn response_mensaje_error_con_status(titulo: &str, mensaje: &str, status: u16) -> Response {
    response_mensaje_error(titulo, mensaje).with_status_code(status)
}
