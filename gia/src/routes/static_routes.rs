use crate::constants::STATIC_DIR;
use crate::handlers::static_handler::StaticHandler;

use mime_guess::from_path;
use rouille::{Request, Response, router};
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub fn router(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,

        (GET) (/creditos) => {
            let conn = conn.lock().unwrap();
            StaticHandler::mostrar_creditos(request, &conn)
        },

        _ => {
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
                        let mime = from_path(&disk_path)
                            .first_or_octet_stream()
                            .to_string();

                        let mut response = Response::from_data(mime, bytes);

                        if path == "sw.js" {
                            response = response.with_additional_header(
                                "Service-Worker-Allowed",
                                "/",
                            );
                        }

                        response
                    }

                    Err(_) => Response::empty_404(),
                };
            }

            Response::empty_404()
        }
    )
}
