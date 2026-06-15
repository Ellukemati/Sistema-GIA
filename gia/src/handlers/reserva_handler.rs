use crate::{
    repository::{
        ejemplar_repository::EjemplarRepository, modelo_repository::ModeloRepository,
        sesion_repository::SesionRepository,
    },
    service::reserva_service::ReservaService,
    utils::extraer_token_sesion,
};

use chrono::{Duration, Local};
use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;

pub struct ReservaHandler;

impl ReservaHandler {
    pub fn mostrar_formulario_reserva(request: &Request, conn: &Connection) -> Response {
        if let Err(response) = Self::obtener_usuario_sesion(request, conn) {
            return response;
        }

        let modelos = match ModeloRepository::listar_todos(conn) {
            Ok(m) => m,

            Err(e) => {
                return Response::text(format!("Error cargando modelos: {}", e))
                    .with_status_code(500);
            }
        };

        let mut contenido = String::new();

        for modelo in modelos {
            let marca = &modelo.marca;

            let categoria = modelo
                .categoria
                .clone()
                .unwrap_or("Sin categoría".to_string());

            let descripcion = modelo
                .descripcion
                .clone()
                .unwrap_or("Sin descripción".to_string());

            contenido.push_str(&format!(
                r#"
                <div
                    style="
                        border:1px solid #ccc;
                        padding:15px;
                        margin-bottom:15px;
                    ">

                    <h3>{}</h3>

                    <p>
                        <b>Marca:</b> {}
                    </p>

                    <p>
                        <b>Categoría:</b> {}
                    </p>

                    <p>
                        {}
                    </p>

                    <a href="/reservas/modelo/{}">
                        Ver ejemplares
                    </a>

                </div>
                "#,
                modelo.nombre_modelo, marca, categoria, descripcion, modelo.id
            ));
        }

        let html = include_str!("../../templates/reserva_modelos.html");

        let html = html.replace("{{modelos}}", &contenido);

        Response::html(html)
    }
    pub fn mostrar_ejemplares_modelo(conn: &Connection, modelo_id: i64) -> Response {
        let modelo = match ModeloRepository::buscar_por_id(conn, modelo_id) {
            Ok(Some(m)) => m,

            Ok(None) => {
                return Response::text("Modelo inexistente").with_status_code(404);
            }

            Err(e) => {
                return Response::text(format!("Error cargando modelo: {}", e))
                    .with_status_code(500);
            }
        };

        let ejemplares = match EjemplarRepository::listar_por_modelo(conn, modelo_id) {
            Ok(e) => e,

            Err(err) => {
                return Response::text(format!("Error cargando ejemplares: {}", err))
                    .with_status_code(500);
            }
        };

        let mut opciones = String::new();

        for ejemplar in ejemplares {
            let serie = ejemplar
                .numero_serie
                .clone()
                .unwrap_or("Sin serie".to_string());

            let patrimonio = ejemplar
                .patrimonio
                .clone()
                .unwrap_or("Sin patrimonio".to_string());

            let ubicacion = ejemplar
                .ubicacion
                .clone()
                .unwrap_or("Sin ubicación".to_string());

            opciones.push_str(&format!(
                r#"
                <div
                    style="
                        border:1px solid #ccc;
                        padding:10px;
                        margin-bottom:10px;
                    ">

                    <input
                        type="checkbox"
                        name="ejemplar_id"
                        value="{}">

                    <b>Serie:</b> {}<br>
                    <b>Patrimonio:</b> {}<br>
                    <b>Ubicación:</b> {}<br>

                </div>
                "#,
                ejemplar.id, serie, patrimonio, ubicacion
            ));
        }

        let fecha_minima = (Local::now().date_naive() + Duration::days(5))
            .format("%Y-%m-%d")
            .to_string();

        let fecha_maxima = (Local::now().date_naive() + Duration::days(180))
            .format("%Y-%m-%d")
            .to_string();

        let html = include_str!("../../templates/reserva_ejemplares.html");

        let html = html.replace("{{nombre_modelo}}", &modelo.nombre_modelo);

        let html = html.replace("{{ejemplares}}", &opciones);

        let html = html.replace("{{fecha_minima}}", &fecha_minima);

        let html = html.replace("{{fecha_maxima}}", &fecha_maxima);

        Response::html(html)
    }

    pub fn procesar_reserva(request: &Request, conn: &Connection) -> Response {
        let usuario_id = match Self::obtener_usuario_sesion(request, conn) {
            Ok(id) => id,

            Err(response) => {
                return response;
            }
        };

        let mut body = String::new();

        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos = Self::parsear_formulario(&body);

        let fecha_inicio = datos.get("fecha_inicio").cloned().unwrap_or_default();

        let fecha_fin = datos.get("fecha_fin").cloned().unwrap_or_default();

        let motivo = datos.get("motivo").cloned();

        let ejemplares = Self::obtener_ejemplares(&body);

        match ReservaService::crear_reserva(
            conn,
            usuario_id,
            fecha_inicio,
            fecha_fin,
            motivo,
            ejemplares,
        ) {
            Ok(_) => Response::html(
                "<div style='color:green'>
                    Reserva creada
                </div>",
            ),

            Err(e) => Response::html(format!(
                "<div style='color:red'>
                        {}
                    </div>",
                e
            )),
        }
    }

    fn obtener_ejemplares(body: &str) -> Vec<i64> {
        let mut ids = Vec::new();
        for par in body.split('&') {
            if let Some(id) = par
                .strip_prefix("ejemplar_id=")
                .and_then(|v| v.parse::<i64>().ok())
            {
                ids.push(id);
            }
        }
        ids
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
    fn obtener_usuario_sesion(request: &Request, conn: &Connection) -> Result<i64, Response> {
        let token = match extraer_token_sesion(request) {
            Some(t) => t,

            None => {
                return Err(Response::html(
                    "<div style='color:red'>
                            Debe iniciar sesión
                        </div>",
                )
                .with_status_code(401));
            }
        };

        let sesion = match SesionRepository::buscar_por_token(conn, &token) {
            Ok(Some(s)) => s,

            _ => {
                return Err(Response::html(
                    "<div style='color:red'>
                            Sesión inválida
                        </div>",
                )
                .with_status_code(401));
            }
        };

        Ok(sesion.usuario_id)
    }
}
