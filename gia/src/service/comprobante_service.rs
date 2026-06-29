use base64::{Engine as _, engine::general_purpose};
use serde::Serialize;
use std::io::Read;
use wkhtmltopdf::PdfApplication;

#[derive(Serialize)]
pub struct DetalleEjemplarComprobante {
    pub id_interno: i64,
    pub marca: String,
    pub nombre_modelo: String,
    pub categoria: String,
    pub numero_serie: Option<String>,
    pub codigo_qr: Option<String>,
    pub patrimonio: Option<String>,
    pub observaciones: Option<String>,
    pub accesorios: Option<String>,
    pub imagenes_bytes: Vec<Vec<u8>>,
    pub imagenes_b64: Vec<String>,
}

#[derive(Serialize)]
pub struct ComprobanteData {
    pub docente_email: String,
    pub docente: String,
    pub fecha_hora_actual: String,
    pub motivo: String,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub admin_nombre: String,
    pub admin_id: i64,
    pub items: Vec<DetalleEjemplarComprobante>,
    pub fecha_hora_confirmacion: String,
    pub periodo_reserva: String,
}

pub struct ComprobanteService;

impl ComprobanteService {
    pub fn generar_pdf_en_memoria(data: ComprobanteData) -> Result<Vec<u8>, String> {
        let app = wkhtmltopdf::PdfApplication::new()
            .map_err(|e| format!("Error inicializando wkhtmltopdf: {}", e))?;
        Self::generar_pdf_con_app(&app, data, "", "")
    }

    pub fn generar_pdf_con_app(
        app: &PdfApplication,
        mut data: ComprobanteData,
        logo_fiuba_b64: &str,
        logo_agrimensura_b64: &str,
    ) -> Result<Vec<u8>, String> {
        let tera = tera::Tera::new("templates/**/*")
            .map_err(|e| format!("Error inicializando Tera: {}", e))?;

        for item in data.items.iter_mut() {
            item.imagenes_b64 = item
                .imagenes_bytes
                .iter()
                .map(|bytes_foto| general_purpose::STANDARD.encode(bytes_foto))
                .collect();
        }

        let mut ctx = tera::Context::new();
        ctx.insert("docente", &data.docente);
        ctx.insert("fecha_hora_actual", &data.fecha_hora_actual);
        ctx.insert("motivo", &data.motivo);
        ctx.insert("fecha_inicio", &data.fecha_inicio);
        ctx.insert("fecha_fin", &data.fecha_fin);
        ctx.insert("admin_nombre", &data.admin_nombre);
        ctx.insert("admin_id", &data.admin_id);
        ctx.insert("fecha_hora_confirmacion", &data.fecha_hora_confirmacion);
        ctx.insert("periodo_reserva", &data.periodo_reserva);
        ctx.insert("items", &data.items);
        ctx.insert("logo_fiuba_b64", &logo_fiuba_b64);
        ctx.insert("logo_agrimensura_b64", &logo_agrimensura_b64);

        let cuerpo_html = tera
            .render("comprobante_pdf.html", &ctx)
            .map_err(|e| format!("Error en plantilla Tera al renderizar: {}", e))?;

        let mut builder = app.builder();

        builder
            .margin(wkhtmltopdf::Margin {
                top: wkhtmltopdf::Size::Millimeters(10),
                bottom: wkhtmltopdf::Size::Millimeters(10),
                left: wkhtmltopdf::Size::Millimeters(10),
                right: wkhtmltopdf::Size::Millimeters(10),
            })
            .image_quality(100);

        let mut pdf_output = builder
            .build_from_html(&cuerpo_html)
            .map_err(|e| format!("Error wkhtmltopdf: {}", e))?;

        let mut pdf_bytes = Vec::new();
        pdf_output
            .read_to_end(&mut pdf_bytes)
            .map_err(|e| format!("Error leyendo buffer de salida: {}", e))?;

        Ok(pdf_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        PATH_LOGO_AGRIMENSURA_TRANSPARENTE, PATH_LOGO_FIUBA_TRANSPARENTE, PDF_TESTING,
    };

    #[test]
    fn test_generar_pdf_vacio_y_con_items() {
        let pdf_app = PdfApplication::new().expect("No se pudo inicializar wkhtmltopdf en test");

        let data = ComprobanteData {
            docente_email: "jperez@fi.uba.ar".to_string(),
            docente: "Dr. Juan Pérez".to_string(),
            fecha_hora_actual: "2 de julio de 2026 - 05:38 hs".to_string(),
            motivo: "Práctica Taller Agrimensura".to_string(),
            fecha_inicio: "2026-08-18".to_string(),
            fecha_fin: "2026-08-22".to_string(),
            admin_nombre: "Admin GIA".to_string(),
            admin_id: 1,
            fecha_hora_confirmacion: "30 de junio de 2026 - 19:22 hs".to_string(),
            periodo_reserva: "desde el 18 de agosto hasta el 22 de agosto".to_string(),
            items: vec![DetalleEjemplarComprobante {
                id_interno: 1,
                marca: "Topcon".to_string(),
                nombre_modelo: "GTS-230".to_string(),
                categoria: "Estación Total".to_string(),
                numero_serie: Some("SN123456".to_string()),
                codigo_qr: Some("103".to_string()),
                patrimonio: Some("FIUBA-0042".to_string()),
                observaciones: Some("Calibrado recientemente".to_string()),
                accesorios: Some("Trípode y prisma".to_string()),
                imagenes_bytes: vec![],
                imagenes_b64: vec![],
            }],
        };

        let logo_fiuba_bytes =
            std::fs::read(PATH_LOGO_FIUBA_TRANSPARENTE).unwrap_or_else(|_| vec![0; 4]);
        let logo_agri_bytes =
            std::fs::read(PATH_LOGO_AGRIMENSURA_TRANSPARENTE).unwrap_or_else(|_| vec![0; 4]);

        let logo_fiuba_b64 = general_purpose::STANDARD.encode(logo_fiuba_bytes);
        let logo_agri_b64 = general_purpose::STANDARD.encode(logo_agri_bytes);

        let bytes = match ComprobanteService::generar_pdf_con_app(
            &pdf_app,
            data,
            &logo_fiuba_b64,
            &logo_agri_b64,
        ) {
            Ok(b) => b,
            Err(e) => panic!("El generador de PDF falló: {:?}", e),
        };

        if PDF_TESTING {
            let _ = std::fs::write("comprobante_test_desde_test.pdf", &bytes);
        }

        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..4], b"%PDF");
    }
}
