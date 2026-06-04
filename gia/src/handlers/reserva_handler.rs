use crate::service::reserva_service::ReservaService;
use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;

pub struct ReservaHandler;

impl ReservaHandler {
    pub fn mostrar_formulario_reserva() -> Response {
        let html = include_str!("../../templates/reserva.html");

        Response::html(html)
    }

    pub fn procesar_reserva(request: &Request, conn: &Connection) -> Response {
        let mut body = String::new();

        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos = Self::parsear_formulario(&body);

        let usuario_id = match datos.get("usuario_id").and_then(|v| v.parse::<i64>().ok()) {
            Some(id) => id,
            None => {
                return Response::text("Usuario inválido").with_status_code(400);
            }
        };

        let ejemplar_id = match datos.get("ejemplar_id").and_then(|v| v.parse::<i64>().ok()) {
            Some(id) => id,
            None => {
                return Response::text("Ejemplar inválido").with_status_code(400);
            }
        };

        let fecha_inicio = datos.get("fecha_inicio").cloned().unwrap_or_default();

        let fecha_fin = datos.get("fecha_fin").cloned().unwrap_or_default();

        let motivo = datos.get("motivo").cloned();

        match ReservaService::crear_reserva(
            conn,
            usuario_id,
            fecha_inicio,
            fecha_fin,
            motivo,
            vec![ejemplar_id],
        ) {
            Ok(_) => Response::html(
                "<div style='color:green;'>
                        Reserva creada correctamente
                    </div>",
            ),

            Err(e) => {
                let html = format!(
                    "<div style='color:red;'>
                        {}
                    </div>",
                    e
                );

                Response::html(html)
            }
        }
    }

    fn parsear_formulario(body: &str) -> HashMap<String, String> {
        let mut mapa = HashMap::new();

        for par in body.split('&') {
            let mut partes = par.split('=');

            if let (Some(clave), Some(valor)) = (partes.next(), partes.next()) {
                let valor_decodificado = valor.replace("%40", "@").replace("+", " ");

                mapa.insert(clave.to_string(), valor_decodificado);
            }
        }

        mapa
    }
}
