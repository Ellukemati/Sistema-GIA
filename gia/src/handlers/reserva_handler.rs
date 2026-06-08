use crate::{
    repository::{
        ejemplar_repository::EjemplarRepository,
        modelo_repository::ModeloRepository,
    },
    service::reserva_service::ReservaService,
};

use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;

pub struct ReservaHandler;

impl ReservaHandler {

    pub fn mostrar_formulario_reserva(
    conn: &Connection,
) -> Response {

    let modelos =
        match ModeloRepository::listar_todos(conn) {

            Ok(m) => m,

            Err(e) => {
                return Response::text(
                    format!(
                        "Error cargando modelos: {}",
                        e
                    )
                )
                .with_status_code(500);
            }
        };

    let mut contenido = String::new();

    for modelo in modelos {

        let marca =
            modelo
                .marca
                .clone()
                .unwrap_or(
                    "Sin marca".to_string()
                );

        let categoria =
            modelo
                .categoria
                .clone()
                .unwrap_or(
                    "Sin categoría".to_string()
                );

        let descripcion =
            modelo
                .descripcion
                .clone()
                .unwrap_or(
                    "Sin descripción".to_string()
                );

        contenido.push_str(
            &format!(
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
                modelo.modelo,
                marca,
                categoria,
                descripcion,
                modelo.id
            )
        );
    }

    let html =
        include_str!(
            "../../templates/reserva_modelos.html"
        );

    let html =
        html.replace(
            "{{modelos}}",
            &contenido,
        );

    Response::html(html)
}
pub fn mostrar_ejemplares_modelo(
    conn: &Connection,
    modelo_id: i64,
) -> Response {

    let modelo =
        match ModeloRepository::buscar_por_id(
            conn,
            modelo_id,
        ) {

            Ok(Some(m)) => m,

            Ok(None) => {
                return Response::text(
                    "Modelo inexistente"
                )
                .with_status_code(404);
            }

            Err(e) => {
                return Response::text(
                    format!(
                        "Error cargando modelo: {}",
                        e
                    )
                )
                .with_status_code(500);
            }
        };

    let ejemplares =
        match EjemplarRepository::listar_por_modelo(
            conn,
            modelo_id,
        ) {

            Ok(e) => e,

            Err(err) => {
                return Response::text(
                    format!(
                        "Error cargando ejemplares: {}",
                        err
                    )
                )
                .with_status_code(500);
            }
        };

    let mut opciones = String::new();

    for ejemplar in ejemplares {

        let serie =
            ejemplar
                .numero_serie
                .clone()
                .unwrap_or(
                    "Sin serie".to_string()
                );

        let patrimonio =
            ejemplar
                .patrimonio
                .clone()
                .unwrap_or(
                    "Sin patrimonio".to_string()
                );

        let ubicacion =
            ejemplar
                .ubicacion
                .clone()
                .unwrap_or(
                    "Sin ubicación".to_string()
                );

        opciones.push_str(
            &format!(
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
                ejemplar.id,
                serie,
                patrimonio,
                ubicacion
            )
        );
    }

    let html =
        include_str!(
            "../../templates/reserva_ejemplares.html"
        );

    let html =
        html.replace(
            "{{nombre_modelo}}",
            &modelo.modelo,
        );

    let html =
        html.replace(
            "{{ejemplares}}",
            &opciones,
        );

    Response::html(html)
}

    pub fn procesar_reserva(
        request: &Request,
        conn: &Connection,
    ) -> Response {

        let mut body = String::new();

        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos =
            Self::parsear_formulario(&body);

        let usuario_id =
            match datos.get("usuario_id")
                .and_then(|v| v.parse::<i64>().ok())
            {
                Some(id) => id,

                None => {
                    return Response::text(
                        "Usuario inválido"
                    )
                    .with_status_code(400);
                }
            };

        let fecha_inicio =
            datos.get("fecha_inicio")
                .cloned()
                .unwrap_or_default();

        let fecha_fin =
            datos.get("fecha_fin")
                .cloned()
                .unwrap_or_default();

        let motivo =
            datos.get("motivo")
                .cloned();

        let ejemplares =
            Self::obtener_ejemplares(&body);

        match ReservaService::crear_reserva(
            conn,
            usuario_id,
            fecha_inicio,
            fecha_fin,
            motivo,
            ejemplares,
        ) {

            Ok(_) => {

                Response::html(
                    "<div style='color:green'>
                        Reserva creada
                    </div>"
                )
            }

            Err(e) => {

                Response::html(
                    format!(
                        "<div style='color:red'>
                            {}
                        </div>",
                        e
                    )
                )
            }
        }
    }

    fn obtener_ejemplares(
        body: &str,
    ) -> Vec<i64> {

        let mut ids = Vec::new();

        for par in body.split('&') {

            if let Some(valor) =
                par.strip_prefix("ejemplar_id=")
            {
                if let Ok(id) =
                    valor.parse::<i64>()
                {
                    ids.push(id);
                }
            }
        }

        ids
    }

    fn parsear_formulario(
        body: &str,
    ) -> HashMap<String, String> {

        let mut mapa = HashMap::new();

        for par in body.split('&') {

            let mut partes =
                par.split('=');

            if let (
                Some(clave),
                Some(valor),
            ) = (
                partes.next(),
                partes.next(),
            ) {

                mapa.insert(
                    clave.to_string(),
                    valor
                        .replace("%40", "@")
                        .replace("+", " "),
                );
            }
        }

        mapa
    }
}