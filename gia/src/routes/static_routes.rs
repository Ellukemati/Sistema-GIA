use crate::constants::STATIC_DIR;
use mime_guess::from_path;
use rouille::{Request, Response};
use std::fs;
use std::path::Path;

pub fn router(request: &Request) -> Response {
    if request.method() == "GET" && request.url().starts_with("/static/") {
        let path = request.url().replacen("/static/", "", 1);

        if path.contains("..") {
            return Response::empty_404();
        }

        let disk_path = Path::new(STATIC_DIR).join(&path);

        if !disk_path.is_file() {
            return Response::empty_404();
        }

        return match fs::read(&disk_path) {
            Ok(bytes) => {
                let mime = from_path(&disk_path).first_or_octet_stream().to_string();
                Response::from_data(mime, bytes)
            }
            Err(_) => Response::empty_404(),
        };
    }

    Response::empty_404()
}
