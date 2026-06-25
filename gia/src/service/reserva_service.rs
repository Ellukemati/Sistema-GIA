use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime};
use rusqlite::Connection;
use serde::Serialize;

use crate::service::mail_service::MailService;
use crate::{
    models::reserva::Reserva,
    repository::{
        ejemplar_repository::EjemplarRepository, modelo_repository::ModeloRepository,
        reserva_instrumento_repository::ReservaInstrumentoRepository,
        reserva_repository::ReservaRepository,
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
        reserva_id: i64,
        admin_id: i64,
    ) -> Result<(), String> {
        use crate::constants::ESTADO_ACTIVA;

        let ahora_confirmada = Local::now()
            .naive_local()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        ReservaRepository::confirmar_aprobacion(
            conn,
            reserva_id,
            ESTADO_ACTIVA,
            admin_id,
            &ahora_confirmada,
        )
        .map_err(|e| format!("Error al persistir la aprobación: {}", e))?;

        let (
            docente_email,
            docente_nombre,
            motivo,
            fecha_inicio,
            momento_confirmacion,
            admin_nombre,
        ) = ReservaRepository::obtener_datos_notificacion(conn, reserva_id)
            .map_err(|e| format!("Error consultando datos de auditoría: {}", e))?;

        let items_raw = ReservaRepository::obtener_equipos_por_reserva(conn, reserva_id)
            .map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for item in items_raw {
            let imagenes_bytes =
                ReservaRepository::obtener_imagenes_ejemplar(conn, item.ejemplar_id)
                    .unwrap_or_default();

            items.push(
                crate::service::comprobante_service::DetalleEjemplarComprobante {
                    id_interno: item.ejemplar_id,
                    marca: item.marca,
                    nombre_modelo: item.nombre_modelo,
                    categoria: item
                        .categoria
                        .unwrap_or_else(|| "Instrumentos Varios".into()),
                    numero_serie: item.numero_serie,
                    codigo_qr: item.codigo_qr,
                    patrimonio: item.patrimonio,
                    observaciones: item.observaciones,
                    accesorios: item.accesorios,
                    imagenes_bytes,
                },
            );
        }

        let rango_fechas_texto =
            Self::formatear_rango_fechas_institucional(&fecha_inicio, &fecha_inicio);

        let emision_reporte_legible = Local::now()
            .naive_local()
            .format("%d/%m/%Y %H:%M")
            .to_string();

        let confirmacion_pdf_legible =
            match NaiveDateTime::parse_from_str(&momento_confirmacion, "%Y-%m-%d %H:%M:%S") {
                Ok(dt) => dt.format("%d/%m/%Y %H:%M").to_string(),
                Err(_) => momento_confirmacion.clone(),
            };

        let data_comprobante = crate::service::comprobante_service::ComprobanteData {
            docente: docente_nombre.clone(),
            fecha_hora_actual: emision_reporte_legible,
            motivo: motivo.clone(),
            fecha_inicio: rango_fechas_texto.clone(),
            fecha_fin: String::new(), // Queda seguro y compatible
            admin_nombre,
            admin_id,
            momento_confirmacion: confirmacion_pdf_legible,
            items,
        };

        let pdf_bytes =
            crate::service::comprobante_service::ComprobanteService::generar_pdf_en_memoria(
                data_comprobante,
            )?;

        if crate::constants::PDF_TESTING {
            println!(
                "[GIA DEBUG]: Escribiendo comprobante_test.pdf en la raíz por constante PDF_TESTING..."
            );
            let _ = std::fs::write("comprobante_test.pdf", &pdf_bytes);
        }

        let id_reserva_str = reserva_id.to_string();
        std::thread::spawn(move || {
            let _ = MailService::enviar_notificacion_reserva_aprobada_con_comprobante(
                &docente_email,
                &docente_nombre,
                &id_reserva_str,
                &motivo,
                &rango_fechas_texto,
                &pdf_bytes,
            );
        });

        Ok(())
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

    fn formatear_rango_fechas_institucional(fecha_inicio_str: &str, fecha_fin_str: &str) -> String {
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

        let inicio = match NaiveDate::parse_from_str(fecha_inicio_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => return format!("desde el {} hasta el {}", fecha_inicio_str, fecha_fin_str),
        };

        let fin = match NaiveDate::parse_from_str(fecha_fin_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => return format!("desde el {} hasta el {}", fecha_inicio_str, fecha_fin_str),
        };

        let dia_ini = inicio.day();
        let mes_ini = meses[(inicio.month() as usize) - 1];
        let anio_ini = inicio.year();

        let dia_fin = fin.day();
        let mes_fin = meses[(fin.month() as usize) - 1];
        let anio_fin = fin.year();

        if inicio == fin {
            format!("para el próximo {} de {}", dia_ini, mes_ini)
        } else if anio_ini != anio_fin {
            format!(
                "desde el {} de {} de {} hasta el {} de {} de {}",
                dia_ini, mes_ini, anio_ini, dia_fin, mes_fin, anio_fin
            )
        } else {
            format!(
                "desde el {} de {} hasta el {} de {}",
                dia_ini, mes_ini, dia_fin, mes_fin
            )
        }
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

// MÓDULO DE TESTS ORIGINAL CONSERVADO INTACTO
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
                manual_mime TEXT
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
                ubicacion TEXT
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

    #[test]
    fn test_formatear_rango_fechas_mismo_dia() {
        let res = ReservaService::formatear_rango_fechas_institucional("2026-08-18", "2026-08-18");
        assert_eq!(res, "para el próximo 18 de agosto");
    }
}
