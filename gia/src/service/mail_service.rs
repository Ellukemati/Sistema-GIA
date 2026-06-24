use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::constants::{MAILTRAP_PASSWORD, MAILTRAP_USER, MOCK_MAILS};

// Abstracción para cambiar de servicio envío de correos electrónicos
// permitiendo cambiar entre proveedores reales (Inversión de dependencia) y simulados (mocking) sin afectar el resto del código
pub trait MailProvider {
    fn enviar(&self, email: &str, nombre: &str, asunto: &str, cuerpo: &str) -> Result<(), String>;
}

// MAILTRAP (SMTP)
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
            .from("No-Responder <no-responder@gia.fi.uba.ar>".parse().unwrap())
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
}

// SIMULACIÓN EN CONSOLA (MOCKING)
pub struct ConsolaProvider;

impl MailProvider for ConsolaProvider {
    fn enviar(&self, email: &str, nombre: &str, asunto: &str, cuerpo: &str) -> Result<(), String> {
        println!("\n--- [MOCK MAIL SERVICE] ---");
        println!("Para: {} <{}>", nombre, email);
        println!("Asunto: {}", asunto);
        println!("Cuerpo:\n{}", cuerpo);
        println!("---------------------------\n");
        Ok(())
    }
}

// SERVICE
pub struct MailService;

impl MailService {
    /// Inyección basada en tus constantes globales y entorno
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
                "Hola {},\n\n{}\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
                nombre_completo, mensaje_cuerpo
            );

            match provider.enviar(email, nombre_completo, asunto, &cuerpo) {
                Ok(_) => enviados_con_exito += 1,
                Err(e) => println!("Falló el envío para {}: {}", email, e),
            }

            // Solo espera los 10.5 segundos si estamos pegándole a la API de Mailtrap
            if !MOCK_MAILS && (indice + 1 < total) {
                println!("Esperando 10.5 segundos para respetar el límite gratuito de Mailtrap...");
                std::thread::sleep(std::time::Duration::from_millis(10500));
            }
        }

        Ok(enviados_con_exito)
    }

    pub fn enviar_notificacion_reserva_aprobada(
        email_destino: &str,
        profe_nombre: &str,
        id_reserva: &str,
        motivo: &str,
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = "Reserva de Instrumental Aprobada - Sistema GIA";
        let cuerpo = format!(
            "Hola {},\n\nSu solicitud de reserva con ID: '{}' y con el motivo '{}' ha sido APROBADA.\nPuede pasar a retirar el instrumental en el momento correspondiente.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
            profe_nombre, id_reserva, motivo
        );

        provider.enviar(email_destino, profe_nombre, asunto, &cuerpo)
    }

    pub fn enviar_notificacion_reserva_rechazada(
        email_destino: &str,
        profe_nombre: &str,
        id_reserva: &str,
        motivo: &str,
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = "Reserva de Instrumental Rechazada - Sistema GIA";
        let cuerpo = format!(
            "Hola {},\n\nSu solicitud de reserva con ID: '{}' y con el motivo '{}' fue RECHAZADA.\n\nPor favor, póngase en contacto con el departamento por cualquier consulta.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
            profe_nombre, id_reserva, motivo
        );

        provider.enviar(email_destino, profe_nombre, asunto, &cuerpo)
    }

    pub fn enviar_notificacion_profesor_aprobado(
        email_destino: &str,
        profe_nombre: &str,
    ) -> Result<(), String> {
        let provider = Self::obtener_provider();
        let asunto = "Cuenta Habilitada - Sistema GIA";
        let cuerpo = format!(
            "Bienvenido/a {},\n\nSu solicitud de alta como Docente en el sistema GIA ha sido APROBADA por la administración.\nA partir de este momento puede ingresar al sistema y realizar solicitudes de reserva de instrumental.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
            profe_nombre
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
            "Hola {},\n\nLe informamos que su solicitud de alta como Docente en el sistema GIA ha sido RECHAZADA por la administración.\n\nSi considera que esto se debe a un error, por favor póngase en contacto con el Departamento.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
            profe_nombre
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
            "Hola {},\n\nSe ha solicitado un enlace para restablecer la contraseña de su cuenta en el sistema GIA.\n\nPara continuar, haga clic en el siguiente enlace (Válido por 15 minutos):\n{}\n\nSi usted no realizó esta solicitud, puede ignorar este correo de forma segura.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA",
            nombre_usuario, link_restablecimiento
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
        } else if tipo_rol == "P" {
            "Docente"
        } else {
            "Rol Desconocido"
        };

        let cuerpo = format!(
            "Hola,\n\nSe ha generado una invitación institucional para darte de alta como {} en el Sistema GIA (Gestión de Instrumental de Agrimensura).\n\nPara configurar tu contraseña y activar tu acceso completo, ingresá al siguiente enlace (Válido por 24 horas):\n{}\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA",
            nombre_rol, link_invitacion
        );

        provider.enviar(email_destino, "Colega", asunto, &cuerpo)
    }
}
