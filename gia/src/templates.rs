use std::sync::OnceLock;

use tera::{Context, Tera};

static ENGINE: OnceLock<Tera> = OnceLock::new();

fn engine() -> &'static Tera {
    ENGINE.get_or_init(|| {
        Tera::new("templates/**/*.html").expect("No se pudieron cargar las plantillas")
    })
}

pub fn render(template: &str, context: &Context) -> Result<String, tera::Error> {
    engine().render(template, context)
}
