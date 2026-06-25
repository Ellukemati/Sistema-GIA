use base64::{Engine as _, engine::general_purpose};
use serde::Serialize;
use std::io::Read;
use wkhtmltopdf::{PdfApplication, Size};

use crate::constants::{PATH_LOGO_AGRIMENSURA, PATH_LOGO_FIUBA};

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
}

#[derive(Serialize)]
pub struct ComprobanteData {
    pub docente: String,
    pub fecha_hora_actual: String,
    pub motivo: String,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub admin_nombre: String,
    pub admin_id: i64,
    pub momento_confirmacion: String,
    pub items: Vec<DetalleEjemplarComprobante>,
}

pub struct ComprobanteService;

impl ComprobanteService {
    pub fn generar_pdf_en_memoria(data: ComprobanteData) -> Result<Vec<u8>, String> {
        let tera = tera::Tera::new("templates/**/*")
            .map_err(|e| format!("Error inicializando Tera: {}", e))?;

        let logo_fiuba_bytes = std::fs::read(PATH_LOGO_FIUBA).unwrap_or_else(|_| vec![0; 4]);
        let logo_agrimensura_bytes =
            std::fs::read(PATH_LOGO_AGRIMENSURA).unwrap_or_else(|_| vec![0; 4]);

        let logo_fiuba_b64 = general_purpose::STANDARD.encode(logo_fiuba_bytes);
        let logo_agrimensura_b64 = general_purpose::STANDARD.encode(logo_agrimensura_bytes);

        let mut ctx = tera::Context::new();
        ctx.insert("docente", &data.docente);
        ctx.insert("fecha_hora_actual", &data.fecha_hora_actual);
        ctx.insert("motivo", &data.motivo);
        ctx.insert("fecha_inicio", &data.fecha_inicio);
        ctx.insert("fecha_fin", &data.fecha_fin);
        ctx.insert("admin_nombre", &data.admin_nombre);
        ctx.insert("admin_id", &data.admin_id);
        ctx.insert("momento_confirmacion", &data.momento_confirmacion);
        ctx.insert("items", &data.items);
        ctx.insert("logo_fiuba_b64", &logo_fiuba_b64);
        ctx.insert("logo_agrimensura_b64", &logo_agrimensura_b64);

        let cuerpo_html = tera
            .render("comprobante_pdf.html", &ctx)
            .map_err(|e| format!("Error en plantilla principal: {}", e))?;

        let pdf_app = PdfApplication::new().map_err(|e| e.to_string())?;
        let mut builder = pdf_app.builder();

        builder.margin(Size::Millimeters(20)).image_quality(100);

        let mut pdf_output = builder
            .build_from_html(&cuerpo_html)
            .map_err(|e| format!("Error en wkhtmltopdf: {}", e))?;

        let mut pdf_bytes = Vec::new();
        pdf_output
            .read_to_end(&mut pdf_bytes)
            .map_err(|e| format!("Error al leer buffer de PDF: {}", e))?;

        Ok(pdf_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PDF_TESTING;

    #[test]
    fn test_generar_pdf_vacio_y_con_items() {
        let data = ComprobanteData {
            docente: "Dr. Juan Pérez".to_string(),
            fecha_hora_actual: "25/06/2026 15:45:58".to_string(),
            motivo: "Práctica Taller Agrimensura".to_string(),
            fecha_inicio: "Del 18 de Agosto al 22 de Agosto".to_string(),
            fecha_fin: String::new(),
            admin_nombre: "Admin GIA".to_string(),
            admin_id: 1,
            momento_confirmacion: "2026/06/25 15:30:22".to_string(),
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
                imagenes_bytes: vec![vec![0, 1, 2, 3]],
            }],
        };

        let bytes = match ComprobanteService::generar_pdf_en_memoria(data) {
            Ok(b) => b,
            Err(e) => panic!("El generador de PDF falló con el siguiente error: {:?}", e),
        };

        // Si PDF_TESTING es true, se guarda el PDF generado en disco para testing
        if PDF_TESTING {
            let _ = std::fs::write("comprobante_test_desde_test.pdf", &bytes);
        }

        assert!(
            !bytes.is_empty(),
            "El PDF generado no debería retornar un vector de bytes vacío"
        );

        assert_eq!(
            &bytes[0..4],
            b"%PDF",
            "La firma del archivo generado no coincide con el estándar de formato PDF"
        );
    }
}
