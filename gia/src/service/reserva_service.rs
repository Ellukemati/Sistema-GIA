use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::sync::mpsc::SyncSender;

use crate::constants::{ESTADO_ACTIVA, MOCK_MAILS, PDF_TESTING};
use crate::errors::ErrorComprobante;
use crate::utils::{a_zona_arg, ahora_utc_string, formatear_rango_fechas};
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

#[derive(Serialize)]
pub struct CarritoItemDTO {
    pub ejemplar_id: i64,
    pub modelo_id: i64,
    pub categoria: String,
    pub modelo_nombre: String,
    pub numero_qr: String,
    pub numero_serie: String,
    pub patrimonio: String,
    pub ubicacion: String,
}

impl ReservaService {
    pub fn crear_reserva(
        conn: &Connection,
        id_user: i64,
        inicio: String,
        fin: String,
        motivo: Option<String>,
        ejemplares: Vec<i64>,
    ) -> Result<(), String> {
        Self::validar_parametros(&ejemplares, &inicio, &fin, &motivo)?;
        Self::verificar_disponibilidad(conn, &ejemplares, &inicio, &fin)?;

        let id_res = Self::insertar_reserva(conn, id_user, inicio, fin, motivo)?;

        for ejemplar_id in ejemplares {
            ReservaInstrumentoRepository::crear(conn, id_res, ejemplar_id)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn validar_parametros(
        ejms: &[i64],
        i: &str,
        f: &str,
        mot: &Option<String>,
    ) -> Result<(), String> {
        Self::validar_ejemplares(ejms)?;
        Self::validar_fechas(i, f)?;
        Self::validar_motivo(mot)
    }

    fn verificar_disponibilidad(
        conn: &Connection,
        ejms: &[i64],
        ini: &str,
        fin: &str,
    ) -> Result<(), String> {
        for &id in ejms {
            let disponible = ReservaRepository::ejemplar_disponible(conn, id, ini, fin)
                .map_err(|e| e.to_string())?;
            if !disponible {
                return Err(format!("El ejemplar {} ya está reservado", id));
            }
        }
        Ok(())
    }

    fn insertar_reserva(
        conn: &Connection,
        id_u: i64,
        i: String,
        f: String,
        m: Option<String>,
    ) -> Result<i64, String> {
        let reserva = Reserva {
            id: 0,
            id_usuario: id_u,
            fecha_inicio: i,
            fecha_fin: f,
            estado: "pendiente".into(),
            motivo: m,
            momento_creacion: ahora_utc_string(),
            momento_confirmacion: None,
            id_admin_aprobador: None,
        };
        let id = ReservaRepository::crear(conn, &reserva).map_err(|e| e.to_string())?;
        crate::logger::info(&format!(
            "Reserva (ID: {}) creada por usuario ID: {}",
            id, id_u
        ));
        Ok(id)
    }

    pub fn aprobar_reserva(
        conn: &Connection,
        id: i64,
        id_adm: i64,
        tx: &SyncSender<PdfRequest>,
    ) -> Result<Vec<u8>, String> {
        let ahora = ahora_utc_string();

        ReservaRepository::confirmar_aprobacion(conn, id, ESTADO_ACTIVA, id_adm, &ahora)
            .map_err(|e| format!("Error en BDD: {}", e))?;

        crate::logger::info(&format!(
            "Reserva (ID: {}) aprobada por admin ID: {}",
            id, id_adm
        ));

        let pdf = Self::generar_pdf_aprobacion(conn, id, tx, &ahora)?;

        if PDF_TESTING {
            let _ = std::fs::write(format!("comprobante_{}.pdf", id), &pdf);
        }

        Self::despachar_alertas(conn, id, &pdf);

        Ok(pdf)
    }

    pub fn sincronizar_si_necesario(conn: &Connection) {
        let hoy = crate::utils::a_zona_arg(Utc::now())
            .format("%Y-%m-%d")
            .to_string();

        let ultima = ReservaRepository::obtener_ultima_sincronizacion(conn)
            .unwrap_or_else(|_| "2000-01-01".into());

        if ultima != hoy {
            let _ = ReservaRepository::concluir_reservas_expiradas(conn, &hoy);
            let _ = ReservaRepository::actualizar_fecha_sinc(conn, &hoy);
        }
    }

    fn generar_pdf_aprobacion(
        conn: &Connection,
        id: i64,
        tx: &SyncSender<PdfRequest>,
        momento_generacion: &str,
    ) -> Result<Vec<u8>, String> {
        let mut data = Self::preparar_datos_comprobante(conn, id).map_err(|e| e.to_string())?;
        data.fecha_hora_actual = momento_generacion.to_string();

        let (tx_resp, rx_resp) = oneshot::channel();

        tx.send(PdfRequest {
            data,
            responder: tx_resp,
        })
        .map_err(|_| "Generador no disponible")?;
        rx_resp
            .recv()
            .map_err(|_| "Canal cerrado")?
            .map_err(|e| format!("Error PDF: {}", e))
    }

    fn despachar_alertas(conn: &Connection, id: i64, pdf: &[u8]) {
        if let Ok(datos) = ReservaRepository::obtener_datos_notificacion(conn, id)
            && let Ok(Some(res)) = ReservaRepository::buscar_por_id(conn, id)
        {
            let admins = UsuarioRepository::listar_administradores(conn).unwrap_or_default();
            let pdf_clon = pdf.to_vec();
            std::thread::spawn(move || Self::enviar_mails_hilo(datos, res, admins, pdf_clon));
        }
    }

    fn enviar_mails_hilo(
        d: (String, String, String, String, String, String),
        r: Reserva,
        admins: Vec<crate::models::usuario::Usuario>,
        pdf: Vec<u8>,
    ) {
        let rango = formatear_rango_fechas(&r.fecha_inicio, &r.fecha_fin);
        let _ = MailService::enviar_notificacion_reserva_aprobada_con_comprobante(
            &d.0,
            &d.1,
            &r.id.to_string(),
            &d.2,
            &rango,
            &pdf,
        );

        if !MOCK_MAILS {
            std::thread::sleep(std::time::Duration::from_millis(10500));
        }

        if !admins.is_empty() {
            let _ = MailService::enviar_notificacion_reserva_aprobada_admins_con_comprobante(
                admins,
                &r.id.to_string(),
                &d.1,
                &d.2,
                &rango,
                &pdf,
            );
        }
    }

    pub fn rechazar_reserva(conn: &Connection, id: i64) -> Result<(), String> {
        let (em, doc, mot, _, _, _) = ReservaRepository::obtener_datos_notificacion(conn, id)
            .map_err(|e| format!("Error datos: {}", e))?;

        ReservaRepository::cambiar_estado(conn, id, crate::constants::ESTADO_CANCELADA)
            .map_err(|e| format!("Error actualizando: {}", e))?;

        crate::logger::info(&format!("Reserva {} rechazada.", id));
        let id_str = id.to_string();
        std::thread::spawn(move || {
            let _ = MailService::enviar_notificacion_reserva_rechazada(&em, &doc, &id_str, &mot);
        });
        Ok(())
    }

    pub fn cancelar_reserva(conn: &Connection, id: i64, u_id: i64) -> Result<(), String> {
        match ReservaRepository::cancelar_por_usuario(conn, id, u_id) {
            Ok(0) => Err("No existe o ya cancelada".into()),
            Ok(_) => {
                crate::logger::info(&format!("Reserva {} cancelada por usuario {}", id, u_id));
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn preparar_datos_comprobante(
        conn: &Connection,
        id: i64,
    ) -> Result<ComprobanteData, ErrorComprobante> {
        let r =
            ReservaRepository::buscar_por_id(conn, id)?.ok_or(ErrorComprobante::NoEncontrada)?;
        if r.estado != "activa" && r.estado != "concluida" {
            return Err(ErrorComprobante::NoConfirmada);
        }
        let d = ReservaRepository::obtener_datos_notificacion(conn, id)?;
        Self::armar_comprobante(conn, id, r, d, false)
    }

    pub fn preparar_datos_previsualizacion(
        conn: &Connection,
        id: i64,
    ) -> Result<ComprobanteData, ErrorComprobante> {
        let r =
            ReservaRepository::buscar_por_id(conn, id)?.ok_or(ErrorComprobante::NoEncontrada)?;
        let d = ReservaRepository::obtener_datos_notificacion(conn, id)?;
        Self::armar_comprobante(conn, id, r, d, true)
    }

    fn armar_comprobante(
        conn: &Connection,
        id: i64,
        r: Reserva,
        d: (String, String, String, String, String, String),
        prev: bool,
    ) -> Result<ComprobanteData, ErrorComprobante> {
        let (em, doc, mot, _, conf, adm) = d;
        let p_res = formatear_rango_fechas(&r.fecha_inicio, &r.fecha_fin);
        let items = Self::mapear_equipos_comprobante(conn, id)?;
        let ahora = Self::formatear_fecha_hora_firma(a_zona_arg(chrono::Utc::now()));

        let f_firma = chrono::NaiveDateTime::parse_from_str(&conf, "%Y-%m-%d %H:%M:%S")
            .map(|dt| {
                let arg = a_zona_arg(chrono::Utc.from_utc_datetime(&dt));
                Self::formatear_fecha_hora_firma(arg)
            })
            .unwrap_or(conf);

        let id_a = if prev {
            0
        } else {
            ReservaRepository::obtener_id_admin_aprobador(conn, id)
                .unwrap_or(None)
                .unwrap_or(0)
        };
        let n_adm = if prev {
            "PREVISUALIZACIÓN".into()
        } else {
            adm
        };

        Ok(ComprobanteData {
            docente_email: em,
            docente: doc,
            fecha_hora_actual: ahora,
            motivo: mot,
            fecha_inicio: r.fecha_inicio,
            fecha_fin: r.fecha_fin,
            admin_nombre: n_adm,
            admin_id: id_a,
            fecha_hora_confirmacion: f_firma,
            periodo_reserva: p_res,
            items,
        })
    }

    fn mapear_equipos_comprobante(
        conn: &Connection,
        id: i64,
    ) -> Result<Vec<DetalleEjemplarComprobante>, ErrorComprobante> {
        let raw = ReservaRepository::obtener_equipos_por_reserva(conn, id)?;
        Ok(raw
            .into_iter()
            .map(|e| DetalleEjemplarComprobante {
                id_interno: e.ejemplar_id,
                marca: e.marca,
                nombre_modelo: e.nombre_modelo,
                categoria: e.categoria.unwrap_or_else(|| "Sin categoría".into()),
                numero_serie: e.numero_serie,
                codigo_qr: e.codigo_qr,
                patrimonio: e.patrimonio,
                observaciones: e.observaciones,
                accesorios: e.accesorios,
                imagenes_b64: vec![],
                imagenes_bytes: ReservaRepository::obtener_imagenes_ejemplar(conn, e.ejemplar_id)
                    .unwrap_or_default(),
            })
            .collect())
    }

    pub fn listar_carrito_detalle(
        conn: &Connection,
        ids: &[i64],
    ) -> Result<Vec<CarritoItemDTO>, String> {
        let mut items = Vec::with_capacity(ids.len());
        for &id in ids {
            if let Ok(Some(item)) = Self::obtener_item_carrito(conn, id) {
                items.push(item);
            }
        }
        Ok(items)
    }

    fn obtener_item_carrito(conn: &Connection, id: i64) -> Result<Option<CarritoItemDTO>, String> {
        let e = match EjemplarRepository::buscar_por_id(conn, id).map_err(|x| x.to_string())? {
            Some(ej) => ej,
            None => return Ok(None),
        };
        let m = ModeloRepository::buscar_por_id(conn, e.modelo_id).map_err(|x| x.to_string())?;
        let (n_mod, cat) = m
            .map(|x| {
                (
                    x.nombre_modelo,
                    x.categoria.unwrap_or_else(|| "Sin categoría".into()),
                )
            })
            .unwrap_or_else(|| ("Modelo".into(), "Sin categoría".into()));

        Ok(Some(CarritoItemDTO {
            ejemplar_id: e.id,
            modelo_id: e.modelo_id,
            categoria: cat,
            modelo_nombre: n_mod,
            numero_qr: e.codigo_qr.unwrap_or_else(|| "Sin QR".into()),
            numero_serie: e.numero_serie.unwrap_or_else(|| "Sin serie".into()),
            patrimonio: e.patrimonio.unwrap_or_else(|| "Sin patrimonio".into()),
            ubicacion: e.ubicacion.unwrap_or_else(|| "Sin ubicación".into()),
        }))
    }

    pub fn obtener_reservas_usuario(c: &Connection, u: i64) -> Result<Vec<Reserva>, String> {
        ReservaRepository::listar_por_usuario(c, u).map_err(|e| e.to_string())
    }

    pub fn obtener_todas(c: &Connection) -> Result<Vec<Reserva>, String> {
        ReservaRepository::listar_todas(c).map_err(|e| e.to_string())
    }

    pub fn formatear_fecha_hora_firma<Tz: TimeZone>(momento: DateTime<Tz>) -> String {
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

    fn validar_ejemplares(ejemplares: &[i64]) -> Result<(), String> {
        if ejemplares.is_empty() {
            return Err("Debe seleccionar al menos un ejemplar".into());
        }
        Ok(())
    }

    fn validar_motivo(motivo: &Option<String>) -> Result<(), String> {
        match motivo {
            Some(m) if !m.trim().is_empty() => Ok(()),
            Some(_) => Err("El motivo debe tener al menos 1 caracter".into()),
            None => Err("El motivo de la reserva es obligatorio".into()),
        }
    }

    fn validar_fechas(i: &str, f: &str) -> Result<(), String> {
        let ini = NaiveDate::parse_from_str(i, "%Y-%m-%d")
            .map_err(|_| "Fecha inicio inválida".to_string())?;
        let fin = NaiveDate::parse_from_str(f, "%Y-%m-%d")
            .map_err(|_| "Fecha fin inválida".to_string())?;
        let hoy = Local::now().date_naive();
        let min = hoy + Duration::days(5);
        let max = hoy
            .checked_add_months(chrono::Months::new(4))
            .ok_or("Límite máximo error")?;

        if ini < min {
            return Err("La reserva debe comenzar al menos 5 días después de hoy".into());
        }
        if ini > max {
            return Err("No se puede reservar con más de 4 meses de anticipación".into());
        }

        if fin < ini {
            return Err("La fecha de fin no puede ser anterior a la de inicio".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ejemplar::Ejemplar, modelo::Modelo};
    use rusqlite::Connection;

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE modelos (id INTEGER PRIMARY KEY AUTOINCREMENT, marca TEXT NOT NULL, nombre_modelo TEXT NOT NULL, categoria TEXT, descripcion TEXT, manual_blob BLOB, manual_mime TEXT, eliminado BOOLEAN NOT NULL DEFAULT 0)", []).unwrap();
        conn.execute("CREATE TABLE ejemplares (id INTEGER PRIMARY KEY AUTOINCREMENT, modelo_id INTEGER NOT NULL, numero_serie TEXT UNIQUE, codigo_qr TEXT UNIQUE, patrimonio TEXT UNIQUE, observaciones TEXT, accesorios TEXT, esta_disponible BOOLEAN DEFAULT TRUE, ubicacion TEXT, eliminado BOOLEAN NOT NULL DEFAULT 0)", []).unwrap();
        conn
    }

    fn insertar_modelo(conn: &Connection, nombre: &str) -> i64 {
        ModeloRepository::crear(
            conn,
            &Modelo {
                id: 0,
                marca: "Marca".into(),
                nombre_modelo: nombre.into(),
                categoria: None,
                descripcion: None,
                eliminado: false,
            },
        )
        .unwrap()
    }

    fn insertar_ejemplar(
        c: &Connection,
        m_id: i64,
        n_s: Option<&str>,
        pat: Option<&str>,
        ubi: Option<&str>,
        qr: Option<&str>,
    ) -> i64 {
        EjemplarRepository::crear(
            c,
            &Ejemplar {
                id: 0,
                modelo_id: m_id,
                numero_serie: n_s.map(String::from),
                codigo_qr: qr.map(String::from),
                patrimonio: pat.map(String::from),
                observaciones: None,
                accesorios: None,
                esta_disponible: true,
                ubicacion: ubi.map(String::from),
                eliminado: false,
            },
        )
        .unwrap()
    }

    #[test]
    fn validar_ejemplares_lista_vacia() {
        assert!(ReservaService::validar_ejemplares(&[]).is_err());
    }

    #[test]
    fn validar_ejemplares_lista_no_vacia() {
        assert!(ReservaService::validar_ejemplares(&[1]).is_ok());
    }

    #[test]
    fn validar_fechas_inicio_muy_cercano() {
        let hoy = Local::now().date_naive();
        assert!(
            ReservaService::validar_fechas(
                &(hoy + Duration::days(2)).format("%Y-%m-%d").to_string(),
                &(hoy + Duration::days(10)).format("%Y-%m-%d").to_string()
            )
            .is_err()
        );
    }

    #[test]
    fn validar_fechas_inicio_muy_lejano() {
        let hoy = Local::now().date_naive();
        assert!(
            ReservaService::validar_fechas(
                &(hoy + Duration::days(200)).format("%Y-%m-%d").to_string(),
                &(hoy + Duration::days(210)).format("%Y-%m-%d").to_string()
            )
            .is_err()
        );
    }

    #[test]
    fn validar_fechas_fin_antes_inicio() {
        let hoy = Local::now().date_naive();
        assert!(
            ReservaService::validar_fechas(
                &(hoy + Duration::days(10)).format("%Y-%m-%d").to_string(),
                &(hoy + Duration::days(9)).format("%Y-%m-%d").to_string()
            )
            .is_err()
        );
    }

    #[test]
    fn validar_fechas_mismo_dia() {
        let hoy = Local::now().date_naive();
        let f = (hoy + Duration::days(10)).format("%Y-%m-%d").to_string();
        assert!(ReservaService::validar_fechas(&f, &f).is_ok());
    }

    #[test]
    fn validar_motivo_vacio_es_error() {
        assert!(ReservaService::validar_motivo(&None).is_err());
    }

    #[test]
    fn validar_motivo_solo_espacios() {
        assert!(ReservaService::validar_motivo(&Some("   ".into())).is_err());
    }

    #[test]
    fn validar_motivo_valido() {
        assert!(ReservaService::validar_motivo(&Some("Práctica".into())).is_ok());
    }

    #[test]
    fn listar_carrito_vacio() {
        assert!(
            ReservaService::listar_carrito_detalle(&crear_db_test(), &[])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn listar_carrito_detalle_retorna_datos_completos() {
        let conn = crear_db_test();
        let m_id = insertar_modelo(&conn, "Violín");
        let e_id = insertar_ejemplar(
            &conn,
            m_id,
            Some("SN-1"),
            Some("PAT-1"),
            Some("Depo"),
            Some("QR-1"),
        );
        assert_eq!(
            ReservaService::listar_carrito_detalle(&conn, &[e_id]).unwrap()[0].modelo_nombre,
            "Violín"
        );
    }
}
