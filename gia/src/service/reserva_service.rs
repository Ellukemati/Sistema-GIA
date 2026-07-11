use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike};
use rusqlite::Connection;
use serde::Serialize;
use std::sync::mpsc::SyncSender;

use crate::constants::{ESTADO_ACTIVA, MOCK_MAILS, PDF_TESTING};
use crate::errors::ErrorComprobante;
use crate::{
    models::reserva::Reserva,
    repository::{
        ejemplar_repository::EjemplarRepository, modelo_repository::ModeloRepository,
        reserva_instrumento_repository::ReservaInstrumentoRepository,
        reserva_repository::ReservaRepository, usuario_repository::UsuarioRepository,
    },
    service::{
        comprobante_service::{ComprobanteData, DetalleEjemplarComprobante},
        mail_service::MailService,
        pdf_worker_service::PdfRequest,
    },
};

pub struct ReservaService;

/// Item del carrito listo para mostrar en el detalle, con el nombre del modelo
/// al que pertenece el ejemplar.
#[derive(Serialize)]
pub struct CarritoItemDTO {
    pub ejemplar_id: i64,
    pub modelo_id: i64,
    pub modelo_nombre: String,
    pub numero_serie: String,
    pub patrimonio: String,
    pub ubicacion: String,
}

impl ReservaService {
    pub fn crear_reserva(
        conn: &Connection,
        id_usuario: i64,
        fecha_inicio: String,
        fecha_fin: String,
        motivo: Option<String>,
        ejemplares: Vec<i64>,
    ) -> Result<(), String> {
        Self::validar_ejemplares(&ejemplares)?;
        Self::validar_fechas(&fecha_inicio, &fecha_fin)?;
        Self::validar_motivo(&motivo)?;

        for ejemplar_id in &ejemplares {
            let disponible = ReservaRepository::ejemplar_disponible(
                conn,
                *ejemplar_id,
                &fecha_inicio,
                &fecha_fin,
            )
            .map_err(|e| e.to_string())?;

            if !disponible {
                return Err(format!(
                    "El ejemplar {} ya está reservado para esas fechas",
                    ejemplar_id
                ));
            }
        }

        let ahora_string = Local::now()
            .naive_local()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let reserva = Reserva {
            id: 0,
            id_usuario,
            fecha_inicio,
            fecha_fin,
            estado: "pendiente".to_string(),
            motivo,
            momento_creacion: ahora_string,
            momento_confirmacion: None,
            id_admin_aprobador: None,
        };

        let reserva_id = ReservaRepository::crear(conn, &reserva).map_err(|e| e.to_string())?;

        for ejemplar_id in ejemplares {
            ReservaInstrumentoRepository::crear(conn, reserva_id, ejemplar_id)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn aprobar_reserva(
        conn: &Connection,
        id_reserva: i64,
        admin_id: i64,
        pdf_tx: &SyncSender<PdfRequest>,
    ) -> Result<Vec<u8>, String> {
        let ahora_raw = Local::now()
            .naive_local()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        ReservaRepository::confirmar_aprobacion(
            conn,
            id_reserva,
            ESTADO_ACTIVA,
            admin_id,
            &ahora_raw,
        )
        .map_err(|e| format!("Error al persistir la aprobacion en la BDD: {}", e))?;

        let data = Self::preparar_datos_comprobante(conn, id_reserva)
            .map_err(|e| format!("Error preparando comprobante: {}", e))?;

        let (tx_respuesta, rx_respuesta) = oneshot::channel();
        pdf_tx
            .send(PdfRequest {
                data,
                responder: tx_respuesta,
            })
            .map_err(|_| "El generador de PDF no está disponible")?;

        let pdf_bytes = rx_respuesta
            .recv()
            .map_err(|_| "Canal worker cerrado")?
            .map_err(|e| format!("Error en el motor wkhtmltopdf: {}", e))?;

        if PDF_TESTING {
            let _ = std::fs::write(format!("comprobante_test_{}.pdf", id_reserva), &pdf_bytes);
        }

        Self::despachar_alertas_aprobacion_reserva(conn, id_reserva, &pdf_bytes);

        Ok(pdf_bytes)
    }

    fn despachar_alertas_aprobacion_reserva(conn: &Connection, id_reserva: i64, pdf_bytes: &[u8]) {
        if let Ok((email, docente, motivo, _, _, _)) =
            ReservaRepository::obtener_datos_notificacion(conn, id_reserva)
        {
            let reserva_base = match ReservaRepository::buscar_por_id(conn, id_reserva) {
                Ok(Some(r)) => r,
                _ => {
                    eprintln!(
                        "No se pudo encontrar la reserva {} para despachar alertas",
                        id_reserva
                    );
                    return;
                }
            };

            let id_str = id_reserva.to_string();
            let pdf_bytes_clonado = pdf_bytes.to_vec();

            let rango = crate::utils::formatear_rango_fechas(
                &reserva_base.fecha_inicio,
                &reserva_base.fecha_fin,
            );

            let administradores =
                UsuarioRepository::listar_administradores(conn).unwrap_or_default();

            std::thread::spawn(move || {
                // Envio de mail para el docente
                let _ = MailService::enviar_notificacion_reserva_aprobada_con_comprobante(
                    &email,
                    &docente,
                    &id_str,
                    &motivo,
                    &rango,
                    &pdf_bytes_clonado,
                );

                if !MOCK_MAILS {
                    println!(
                        "Esperando 10.5 segundos para respetar el límite gratuito de Mailtrap..."
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10500));
                }

                // Envío de mail en lote para los administradores
                if !administradores.is_empty() {
                    let _ =
                        MailService::enviar_notificacion_reserva_aprobada_admins_con_comprobante(
                            administradores,
                            &id_str,
                            &docente,
                            &motivo,
                            &rango,
                            &pdf_bytes_clonado,
                        );
                }
            });
        }
    }

    pub fn formatear_fecha_hora_firma(momento: DateTime<Local>) -> String {
        let meses = [
            "enero",
            "febrero",
            "marzo",
            "abril",
            "mayo",
            "junio",
            "julio",
            "agosto",
            "septiembre",
            "octubre",
            "noviembre",
            "diciembre",
        ];
        format!(
            "{} de {} de {} - {:02}:{:02} hs",
            momento.day(),
            meses[(momento.month() as usize) - 1],
            momento.year(),
            momento.hour(),
            momento.minute()
        )
    }

    pub fn preparar_datos_comprobante(
        conn: &Connection,
        reserva_id: i64,
    ) -> Result<ComprobanteData, ErrorComprobante> {
        let reserva_base = ReservaRepository::buscar_por_id(conn, reserva_id)?
            .ok_or(ErrorComprobante::NoEncontrada)?;

        if reserva_base.estado != "activa" && reserva_base.estado != "concluida" {
            return Err(ErrorComprobante::NoConfirmada);
        }

        let (
            docente_email,
            docente_nombre,
            motivo,
            _fecha_inicio_raw,
            momento_confirmacion,
            admin_nombre,
        ) = ReservaRepository::obtener_datos_notificacion(conn, reserva_id)?;

        let periodo_reserva_formateado = crate::utils::formatear_rango_fechas(
            &reserva_base.fecha_inicio,
            &reserva_base.fecha_fin,
        );

        let fecha_firma_formateada =
            match chrono::NaiveDateTime::parse_from_str(&momento_confirmacion, "%Y-%m-%d %H:%M:%S")
            {
                Ok(dt) => {
                    let dt_local = chrono::Local.from_local_datetime(&dt).unwrap();
                    Self::formatear_fecha_hora_firma(dt_local)
                }
                Err(_) => momento_confirmacion.clone(),
            };

        let ahora = chrono::Local::now();
        let fecha_generacion_formateada = Self::formatear_fecha_hora_firma(ahora);

        let equipos_raw = ReservaRepository::obtener_equipos_por_reserva(conn, reserva_id)?;

        let items: Vec<DetalleEjemplarComprobante> = equipos_raw
            .into_iter()
            .map(|eq| {
                let imagenes_bytes =
                    ReservaRepository::obtener_imagenes_ejemplar(conn, eq.ejemplar_id)
                        .unwrap_or_else(|_| vec![]);

                DetalleEjemplarComprobante {
                    id_interno: eq.ejemplar_id,
                    marca: eq.marca,
                    nombre_modelo: eq.nombre_modelo,
                    categoria: eq.categoria.unwrap_or_else(|| "Sin categoría".to_string()),
                    numero_serie: eq.numero_serie,
                    codigo_qr: eq.codigo_qr,
                    patrimonio: eq.patrimonio,
                    observaciones: eq.observaciones,
                    accesorios: eq.accesorios,
                    imagenes_bytes,
                    imagenes_b64: vec![],
                }
            })
            .collect();

        let admin_id = ReservaRepository::obtener_id_admin_aprobador(conn, reserva_id)
            .unwrap_or(None)
            .unwrap_or(0);

        Ok(ComprobanteData {
            docente_email,
            docente: docente_nombre,
            fecha_hora_actual: fecha_generacion_formateada,
            motivo,
            fecha_inicio: reserva_base.fecha_inicio.clone(),
            fecha_fin: reserva_base.fecha_fin.clone(),
            admin_nombre,
            admin_id,
            fecha_hora_confirmacion: fecha_firma_formateada,
            periodo_reserva: periodo_reserva_formateado,
            items,
        })
    }

    pub fn preparar_datos_previsualizacion(
        conn: &Connection,
        reserva_id: i64,
    ) -> Result<ComprobanteData, ErrorComprobante> {
        let reserva_base = ReservaRepository::buscar_por_id(conn, reserva_id)?
            .ok_or(ErrorComprobante::NoEncontrada)?;

        let (
            docente_email,
            docente_nombre,
            motivo,
            _fecha_inicio_raw,
            _momento_confirmacion,
            _admin_nombre,
        ) = ReservaRepository::obtener_datos_notificacion(conn, reserva_id)?;

        let periodo_reserva_formateado = crate::utils::formatear_rango_fechas(
            &reserva_base.fecha_inicio,
            &reserva_base.fecha_fin,
        );

        let ahora = chrono::Local::now();
        let fecha_simulada_formateada = Self::formatear_fecha_hora_firma(ahora);

        let equipos_raw = ReservaRepository::obtener_equipos_por_reserva(conn, reserva_id)?;

        let items: Vec<DetalleEjemplarComprobante> = equipos_raw
            .into_iter()
            .map(|eq| {
                let imagenes_bytes =
                    ReservaRepository::obtener_imagenes_ejemplar(conn, eq.ejemplar_id)
                        .unwrap_or_else(|_| vec![]);

                DetalleEjemplarComprobante {
                    id_interno: eq.ejemplar_id,
                    marca: eq.marca,
                    nombre_modelo: eq.nombre_modelo,
                    categoria: eq.categoria.unwrap_or_else(|| "Sin categoría".to_string()),
                    numero_serie: eq.numero_serie,
                    codigo_qr: eq.codigo_qr,
                    patrimonio: eq.patrimonio,
                    observaciones: eq.observaciones,
                    accesorios: eq.accesorios,
                    imagenes_bytes,
                    imagenes_b64: vec![],
                }
            })
            .collect();

        let admin_id = 0;

        Ok(ComprobanteData {
            docente_email,
            docente: docente_nombre,
            fecha_hora_actual: fecha_simulada_formateada.clone(),
            motivo,
            fecha_inicio: reserva_base.fecha_inicio.clone(),
            fecha_fin: reserva_base.fecha_fin.clone(),
            admin_nombre: "PREVISUALIZACIÓN".to_string(),
            admin_id,
            fecha_hora_confirmacion: fecha_simulada_formateada,
            periodo_reserva: periodo_reserva_formateado,
            items,
        })
    }

    pub fn rechazar_reserva(conn: &Connection, reserva_id: i64) -> Result<(), String> {
        use crate::constants::ESTADO_CANCELADA;

        let (docente_email, docente_nombre, motivo, _, _, _) =
            ReservaRepository::obtener_datos_notificacion(conn, reserva_id)
                .map_err(|e| format!("Error al obtener datos de la reserva: {}", e))?;

        ReservaRepository::cambiar_estado(conn, reserva_id, ESTADO_CANCELADA)
            .map_err(|e| format!("Error al actualizar el estado: {}", e))?;

        let id_reserva_str = reserva_id.to_string();
        std::thread::spawn(move || {
            let _ = MailService::enviar_notificacion_reserva_rechazada(
                &docente_email,
                &docente_nombre,
                &id_reserva_str,
                &motivo,
            );
        });

        Ok(())
    }

    pub fn listar_carrito_detalle(
        conn: &Connection,
        ids: &[i64],
    ) -> Result<Vec<CarritoItemDTO>, String> {
        let mut items = Vec::with_capacity(ids.len());

        for id in ids {
            let ejemplar =
                match EjemplarRepository::buscar_por_id(conn, *id).map_err(|e| e.to_string())? {
                    Some(e) => e,
                    None => continue,
                };

            let modelo_nombre = ModeloRepository::buscar_por_id(conn, ejemplar.modelo_id)
                .map_err(|e| e.to_string())?
                .map(|m| m.nombre_modelo)
                .unwrap_or_else(|| "Modelo".to_string());

            items.push(CarritoItemDTO {
                ejemplar_id: ejemplar.id,
                modelo_id: ejemplar.modelo_id,
                modelo_nombre,
                numero_serie: ejemplar
                    .numero_serie
                    .unwrap_or_else(|| "Sin serie".to_string()),
                patrimonio: ejemplar
                    .patrimonio
                    .unwrap_or_else(|| "Sin patrimonio".to_string()),
                ubicacion: ejemplar
                    .ubicacion
                    .unwrap_or_else(|| "Sin ubicación".to_string()),
            });
        }

        Ok(items)
    }

    fn validar_ejemplares(ejemplares: &[i64]) -> Result<(), String> {
        if ejemplares.is_empty() {
            return Err("Debe seleccionar al menos un ejemplar".to_string());
        }
        Ok(())
    }

    fn validar_motivo(motivo: &Option<String>) -> Result<(), String> {
        match motivo {
            Some(m) if !m.trim().is_empty() => Ok(()),
            Some(_) => Err("El motivo debe tener al menos 1 caracter".to_string()),
            None => Err("El motivo de la reserva es obligatorio".to_string()),
        }
    }

    fn validar_fechas(fecha_inicio: &str, fecha_fin: &str) -> Result<(), String> {
        let inicio = NaiveDate::parse_from_str(fecha_inicio, "%Y-%m-%d")
            .map_err(|_| "Fecha de inicio inválida".to_string())?;

        let fin = NaiveDate::parse_from_str(fecha_fin, "%Y-%m-%d")
            .map_err(|_| "Fecha de fin inválida".to_string())?;

        let hoy = Local::now().date_naive();
        let minimo = hoy + Duration::days(5);
        let maximo = hoy + Duration::days(180);

        if inicio < minimo {
            return Err("La reserva debe comenzar al menos 5 días después de hoy".to_string());
        }
        if inicio > maximo {
            return Err("No se puede reservar con más de 6 meses de anticipación".to_string());
        }
        if fin <= inicio {
            return Err("La fecha de fin debe ser posterior a la fecha de inicio".to_string());
        }
        Ok(())
    }

    pub fn cancelar_reserva(
        conn: &Connection,
        reserva_id: i64,
        usuario_id: i64,
    ) -> Result<(), String> {
        let filas = ReservaRepository::cancelar_por_usuario(conn, reserva_id, usuario_id)
            .map_err(|e| e.to_string())?;

        if filas == 0 {
            return Err("La reserva no existe o ya fue cancelada".to_string());
        }
        Ok(())
    }

    pub fn obtener_reservas_usuario(
        conn: &Connection,
        id_usuario: i64,
    ) -> Result<Vec<Reserva>, String> {
        ReservaRepository::listar_por_usuario(conn, id_usuario).map_err(|e| e.to_string())
    }

    pub fn obtener_todas(conn: &Connection) -> Result<Vec<Reserva>, String> {
        ReservaRepository::listar_todas(conn).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ejemplar::Ejemplar;
    use crate::models::modelo::Modelo;
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE modelos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                marca TEXT NOT NULL,
                nombre_modelo TEXT NOT NULL,
                categoria TEXT,
                descripcion TEXT,
                manual_blob BLOB,
                manual_mime TEXT,
                eliminado BOOLEAN NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE ejemplares (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                modelo_id INTEGER NOT NULL,
                numero_serie TEXT UNIQUE,
                codigo_qr TEXT UNIQUE,
                patrimonio TEXT UNIQUE,
                observaciones TEXT,
                accesorios TEXT,
                esta_disponible BOOLEAN DEFAULT TRUE,
                ubicacion TEXT,
                eliminado BOOLEAN NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn insertar_modelo(conn: &Connection, nombre: &str) -> i64 {
        let modelo = Modelo {
            id: 0,
            marca: "Marca".into(),
            nombre_modelo: nombre.into(),
            categoria: None,
            descripcion: None,
            eliminado: false,
        };
        ModeloRepository::crear(conn, &modelo).unwrap()
    }

    fn insertar_ejemplar(
        conn: &Connection,
        modelo_id: i64,
        numero_serie: Option<&str>,
        patrimonio: Option<&str>,
        ubicacion: Option<&str>,
        codigo_qr: Option<&str>,
    ) -> i64 {
        let ejemplar = Ejemplar {
            id: 0,
            modelo_id,
            numero_serie: numero_serie.map(String::from),
            codigo_qr: codigo_qr.map(String::from),
            patrimonio: patrimonio.map(String::from),
            observaciones: None,
            accesorios: None,
            esta_disponible: true,
            ubicacion: ubicacion.map(String::from),
            eliminado: false,
        };
        EjemplarRepository::crear(conn, &ejemplar).unwrap()
    }

    #[test]
    fn validar_ejemplares_lista_vacia() {
        let resultado = ReservaService::validar_ejemplares(&[]);
        assert!(resultado.is_err());
    }

    #[test]
    fn validar_ejemplares_lista_no_vacia() {
        let resultado = ReservaService::validar_ejemplares(&[1]);
        assert!(resultado.is_ok());
    }

    #[test]
    fn validar_fechas_inicio_muy_cercano() {
        let hoy = Local::now().date_naive();
        let inicio = (hoy + Duration::days(2)).format("%Y-%m-%d").to_string();
        let fin = (hoy + Duration::days(10)).format("%Y-%m-%d").to_string();
        let resultado = ReservaService::validar_fechas(&inicio, &fin);
        assert!(resultado.is_err());
    }

    #[test]
    fn validar_fechas_inicio_muy_lejano() {
        let hoy = Local::now().date_naive();
        let inicio = (hoy + Duration::days(200)).format("%Y-%m-%d").to_string();
        let fin = (hoy + Duration::days(210)).format("%Y-%m-%d").to_string();
        let resultado = ReservaService::validar_fechas(&inicio, &fin);
        assert!(resultado.is_err());
    }

    #[test]
    fn validar_fechas_fin_antes_inicio() {
        let hoy = Local::now().date_naive();
        let inicio = (hoy + Duration::days(10)).format("%Y-%m-%d").to_string();
        let fin = (hoy + Duration::days(9)).format("%Y-%m-%d").to_string();
        let resultado = ReservaService::validar_fechas(&inicio, &fin);
        assert!(resultado.is_err());
    }

    #[test]
    fn validar_fechas_validas() {
        let hoy = Local::now().date_naive();
        let inicio = (hoy + Duration::days(10)).format("%Y-%m-%d").to_string();
        let fin = (hoy + Duration::days(20)).format("%Y-%m-%d").to_string();
        let resultado = ReservaService::validar_fechas(&inicio, &fin);
        assert!(resultado.is_ok());
    }

    #[test]
    fn validar_motivo_vacio_es_error() {
        let resultado = ReservaService::validar_motivo(&None);
        assert!(resultado.is_err());
    }

    #[test]
    fn validar_motivo_solo_espacios_es_error() {
        let resultado = ReservaService::validar_motivo(&Some("   ".into()));
        assert!(resultado.is_err());
    }

    #[test]
    fn validar_motivo_valido() {
        let resultado = ReservaService::validar_motivo(&Some("Práctica de campo".into()));
        assert!(resultado.is_ok());
    }

    #[test]
    fn listar_carrito_detalle_ids_vacio_retorna_vacio() {
        let conn = crear_db_test();
        let items = ReservaService::listar_carrito_detalle(&conn, &[]).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn listar_carrito_detalle_retorna_datos_completos() {
        let conn = crear_db_test();
        let modelo_id = insertar_modelo(&conn, "Violín");
        let ejemplar_id = insertar_ejemplar(
            &conn,
            modelo_id,
            Some("SN-001"),
            Some("PAT-001"),
            Some("Depósito"),
            Some("QR-001"),
        );
        let items = ReservaService::listar_carrito_detalle(&conn, &[ejemplar_id]).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].modelo_nombre, "Violín");
    }
}
