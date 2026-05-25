use crate::constants::STATIC_DIR;
use mime_guess::from_path;
use rouille::{Request, Response};
use std::fs;
use std::path::Path;

pub fn serve(request: &Request) -> Option<Response> {
    if request.method() != "GET" {
        return None;
    }

    let raw_url = request.url();
    let path = raw_url.split('?').next().unwrap_or("");
    let path = path.strip_prefix("/static/")?;

    if path.contains("..") {
        return Some(Response::empty_404());
    }

    // Se arma la ruta fisica del archivo dentro de static/
    let disk_path = Path::new(STATIC_DIR).join(path);

    if !disk_path.is_file() {
        return Some(Response::empty_404());
    }

    // Se lee el archivo y se devuelve el MIME correcto para que el navegador lo interprete
    match fs::read(&disk_path) {
        Ok(bytes) => {
            let mime = from_path(&disk_path).first_or_octet_stream().to_string();
            Some(Response::from_data(mime, bytes))
        }
        Err(_) => Some(Response::empty_404()),
    }
}
