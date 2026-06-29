use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::constants::{
    FIRMA_INSTITUCIONAL, MAIL_EMISOR, MAILTRAP_PASSWORD, MAILTRAP_USER, MOCK_MAILS, PIE_AUTOMATICO,
};

pub trait MailProvider {
    fn enviar(&self, email: &str, nombre: &str, asunto: &str, cuerpo: &str) -> Result<(), String>;

    fn enviar_con_pdf_adjunto(
        &self,
        email: &str,
        nombre: &str,
        asunto: &str,
        cuerpo: &str,
        _pdf_bytes: &[u8],
        _nombre_archivo: &str,
    ) -> Result<(), String> {
        self.enviar(email, nombre, asunto, cuerpo)
    }
}

// MAILTRAP SMTP
pub struct MailtrapProvider;

impl MailtrapProvider {
    fn autenticar_smtp() -> Result<SmtpTransport, String> {
        let creds = Credentials::new(MAILTRAP_USER.to_string(), MAILTRAP_PASSWORD.to_string());
        let transport = SmtpTransport::starttls_relay("sandbox.smtp.mailtrap.io")
            .map_err(|e| format!("Error en host SMTP: {}", e))?
            .port(2525)
            .credentials(creds)
            .build();
        Ok(transport)
    }
}

impl MailProvider for MailtrapProvider {
    fn enviar(&self, email: &str, _nombre: &str, asunto: &str, cuerpo: &str) -> Result<(), String> {
        let transport = Self::autenticar_smtp()?;

        let email_msg = Message::builder()
            .from(MAIL_EMISOR.parse().unwrap())
            .to(email
                .parse()
                .map_err(|_| format!("Email inválido: {}", email))?)
            .subject(asunto)
            .body(cuerpo.to_string())
            .map_err(|e| format!("Error construyendo email: {}", e))?;

        transport
            .send(&email_msg)
            .map_err(|e| format!("Error SMTP: {}", e))?;
        Ok(())
    }

    fn enviar_con_pdf_adjunto(
        &self,
        email: &str,
        _nombre: &str,
        asunto: &str,
        cuerpo: &str,
        pdf_bytes: &[u8],
        nombre_archivo: &str,
    ) -> Result<(), String> {
        let transport = Self::autenticar_smtp()?;

        let texto_part = SinglePart::plain(cuerpo.to_string());
        let adjunto_part = Attachment::new(nombre_archivo.to_string())
            .body(pdf_bytes.to_vec(), "application/pdf".parse().unwrap());

        let email_body = MultiPart::mixed()
            .singlepart(texto_part)
            .singlepart(adjunto_part);

        let email_msg = Message::builder()
            .from(MAIL_EMISOR.parse().unwrap())
            .to(email
                .parse()
                .map_err(|_| format!("Email inválido: {}", email))?)
            .subject(asunto)
            .multipart(email_body)
            .map_err(|e| format!("Error construyendo email con adjunto: {}", e))?;

        transport
            .send(&email_msg)
            .map_err(|e| format!("Error SMTP: {}", e))?;
        Ok(())
    }
}

// MOCKING EN CONSOLA
pub struct ConsolaProvider;

impl MailProvider for ConsolaProvider {
    fn enviar(&self, email: &str, nombre: &str, asunto: &str, cuerpo: &str) -> Result<(), String> {
        println!("\n=================== [MOCK MAIL SERVICE] ===================");
        println!("Desde: {}", MAIL_EMISOR);
        println!("Para: {} <{}>", nombre, email);
        println!("Asunto: {}", asunto);
        println!("-----------------------------------------------------------");
        println!("{}", cuerpo);
        println!("===========================================================\n");
        Ok(())
    }

    fn enviar_con_pdf_adjunto(
        &self,
        email: &str,
        nombre: &str,
        asunto: &str,
        cuerpo: &str,
        pdf_bytes: &[u8],
        nombre_archivo: &str,
    ) -> Result<(), String> {
        self.enviar(email, nombre, asunto, cuerpo)?;
        println!(
            "[MOCK ADJUNTO]: Se acopló el archivo '{}' (Tamaño: {} bytes)",
            nombre_archivo,
            pdf_bytes.len()
        );
        println!("===========================================================\n");
        Ok(())
    }
}

// SERVICE
pub struct MailService;

impl MailService {
    fn obtener_provider() -> Box<dyn MailProvider> {
        if MOCK_MAILS {
            Box::new(ConsolaProvider)
        } else {
            Box::new(MailtrapProvider)
        }
    }

    pub fn enviar_comunicado_lote(
        destinatarios: &[(String, String)],
        asunto: &str,
        mensaje_cuerpo: &str,
    ) -> Result<usize, String> {
        let provider = Self::obtener_provider();
        let mut enviados_con_exito = 0;
        let total = destinatarios.len();

        for (indice, (nombre_completo, email)) in destinatarios.iter().enumerate() {
            let cuerpo = format!(
                "Hola {},\n\n{}\n\n{}\n\n{}",
                nombre_completo, mensaje_cuerpo, FIRMA_INSTITUCIONAL, PIE_AUTOMATICO
            );

            match provider.enviar(email, nombre_completo, asunto, &cuerpo) {
                Ok(_) => enviados_con_exito += 1,
                Err(e) => eprintln!("Falló el envío para {}: {}", email, e),
            }

            if !MOCK_MAILS && (indice + 1 < total) {
                println!("Esperando 10.5 segundos para respetar el límite gratuito de Mailtrap...");
                std::thread::sleep(std::time::Duration::from_millis(10500));
            }
        }

        Ok(enviados_con_exito)
    }

    pub fn enviar_notificacion_reserva_aprobada_con_comprobante(
        email: &str,
        docente: &str,
        id_reserva: &str,
        motivo: &str,
        rango_fechas: &str,
        pdf_bytes: &[u8],
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = format!("Reserva #{} Aprobada - Sistema GIA", id_reserva);
        let nombre_archivo = format!("comprobante_reserva_{}.pdf", id_reserva);

        let cuerpo = format!(
            "Hola {},\n\n\
            Le informamos que su solicitud de reserva con ID #{} y con el motivo '{}' fue APROBADA. La misma será {}.\n\n\
            Se adjunta el comprobante oficial original de retiro para el control del Salón de Instrumental.\n\n\
            {}\n\n\
            {}",
            docente, id_reserva, motivo, rango_fechas, FIRMA_INSTITUCIONAL, PIE_AUTOMATICO
        );

        provider
            .enviar_con_pdf_adjunto(email, docente, &asunto, &cuerpo, pdf_bytes, &nombre_archivo)
            .map_err(|e| format!("Error en el proveedor de correo al enviar adjunto: {}", e))
    }

    pub fn enviar_notificacion_reserva_aprobada_admins_con_comprobante(
        administradores: Vec<crate::models::usuario::Usuario>,
        id_reserva: &str,
        docente_nombre: &str,
        motivo: &str,
        rango_fechas: &str,
        pdf_bytes: &[u8],
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = format!("Reserva #{} Aprobada - Sistema GIA", id_reserva);
        let nombre_archivo = format!("comprobante_reserva_{}.pdf", id_reserva);

        let cuerpo_base = format!(
            "Hola Administradores,\n\n\
            Se les informa que la solicitud de reserva con ID #{} para el/la docente {} y con motivo '{}' fue APROBADA. La misma será {}.\n\n\
            Se adjunta una copia original del comprobante de retiro para el control de instrumental.\n\n\
            {}\n\n\
            {}",
            id_reserva, docente_nombre, motivo, rango_fechas, FIRMA_INSTITUCIONAL, PIE_AUTOMATICO
        );

        for (idx, admin) in administradores.iter().enumerate() {
            let nombre_completo = format!("{} {}", admin.nombre, admin.apellido);
            let _ = provider.enviar_con_pdf_adjunto(
                &admin.email,
                &nombre_completo,
                &asunto,
                &cuerpo_base,
                pdf_bytes,
                &nombre_archivo,
            );

            if !MOCK_MAILS && (idx + 1 < administradores.len()) {
                println!("Esperando 10.5 segundos para respetar el límite gratuito de Mailtrap...");
                std::thread::sleep(std::time::Duration::from_millis(10500));
            }
        }
        Ok(())
    }

    pub fn enviar_notificacion_reserva_rechazada(
        email_destino: &str,
        profe_nombre: &str,
        id_reserva: &str,
        motivo: &str,
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = format!("Reserva #{} Rechazada - Sistema GIA", id_reserva);

        let cuerpo = format!(
            "Hola {},\n\n\
            Le informamos que su solicitud de reserva con ID #{} y con el motivo '{}' ha sido RECHAZADA por la administración.\n\n\
            Por favor, póngase en contacto con el Departamento de Agrimensura ante cualquier duda o consulta.\n\n\
            {}\n\n\
            {}",
            profe_nombre, id_reserva, motivo, FIRMA_INSTITUCIONAL, PIE_AUTOMATICO
        );

        provider.enviar(email_destino, profe_nombre, &asunto, &cuerpo)
    }

    pub fn enviar_notificacion_profesor_aprobado(
        email_destino: &str,
        profe_nombre: &str,
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = "Cuenta Habilitada - Sistema GIA";

        let cuerpo = format!(
            "Bienvenido/a {},\n\n\
            Su solicitud de alta como Docente en el sistema GIA ha sido APROBADA por la administración.\n\n\
            A partir de este momento puede ingresar a la plataforma para realizar solicitudes de reserva de instrumental.\n\n\
            {}\n\n\
            {}",
            profe_nombre, FIRMA_INSTITUCIONAL, PIE_AUTOMATICO
        );

        provider.enviar(email_destino, profe_nombre, asunto, &cuerpo)
    }

    pub fn enviar_notificacion_profesor_rechazado(
        email_destino: &str,
        profe_nombre: &str,
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = "Solicitud de Registro Rechazada - Sistema GIA";

        let cuerpo = format!(
            "Hola {},\n\n\
            Le informamos que su solicitud de alta como Docente en el sistema GIA ha sido RECHAZADA por las autoridades.\n\n\
            Si considera que esto se debe a un error administrativo, por favor póngase en contacto con el Departamento.\n\n\
            {}\n\n\
            {}",
            profe_nombre, FIRMA_INSTITUCIONAL, PIE_AUTOMATICO
        );

        provider.enviar(email_destino, profe_nombre, asunto, &cuerpo)
    }

    pub fn enviar_link_restablecimiento_password(
        email_destino: &str,
        nombre_usuario: &str,
        link_restablecimiento: &str,
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = "Restablecer Contraseña - Sistema GIA";

        let cuerpo = format!(
            "Hola {},\n\n\
            Se ha solicitado un enlace para restablecer la contraseña de su cuenta en el sistema GIA.\n\n\
            Para continuar con el proceso, haga clic en el siguiente enlace (Válido por 15 minutos):\n\
            {}\n\n\
            Si usted no realizó esta solicitud, puede ignorar este correo de forma segura.\n\n\
            {}\n\n\
            {}",
            nombre_usuario, link_restablecimiento, FIRMA_INSTITUCIONAL, PIE_AUTOMATICO
        );

        provider.enviar(email_destino, nombre_usuario, asunto, &cuerpo)
    }

    pub fn enviar_link_invitacion(
        email_destino: &str,
        tipo_rol: &str,
        link_invitacion: &str,
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = "Invitación de Acceso - Sistema GIA";
        let nombre_rol = if tipo_rol == "A" {
            "Administrador"
        } else {
            "Docente"
        };

        let cuerpo = format!(
            "Hola,\n\n\
            Se ha generado una invitación institucional para darle de alta como {} en el Sistema GIA (Gestión de Instrumental de Agrimensura - FIUBA).\n\n\
            Para configurar su contraseña de acceso y habilitar su perfil, ingrese al siguiente enlace (Válido por 24 horas):\n\
            {}\n\n\
            {}\n\n\
            {}",
            nombre_rol, link_invitacion, FIRMA_INSTITUCIONAL, PIE_AUTOMATICO
        );

        provider.enviar(email_destino, "Colega", asunto, &cuerpo)
    }
}
