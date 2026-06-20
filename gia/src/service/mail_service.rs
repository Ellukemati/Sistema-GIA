use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::constants::{MAILTRAP_PASSWORD, MAILTRAP_USER};

pub struct MailService;

impl MailService {
    fn autenticar_smtp() -> Result<SmtpTransport, String> {
        let creds = Credentials::new(MAILTRAP_USER.to_string(), MAILTRAP_PASSWORD.to_string());

        let transport = SmtpTransport::starttls_relay("sandbox.smtp.mailtrap.io")
            .map_err(|e| format!("Error en host SMTP: {}", e))?
            .port(2525)
            .credentials(creds)
            .build();

        Ok(transport)
    }

    // IDEA:
    // Recibe una lista de destinatarios (nombre completo y email), un asunto común y un mensaje común, y envía el comunicado a todos
    // Puede usarse para enviar comunicados generales por mail a todos los usuarios, a un grupo específico o a un usuario solo.
    pub fn enviar_comunicado_lote(
        destinatarios: &[(String, String)],
        asunto: &str,
        mensaje_cuerpo: &str,
    ) -> Result<usize, String> {
        let transport = Self::autenticar_smtp()?;
        let mut enviados_con_exito = 0;
        let total = destinatarios.len();

        for (indice, (nombre_completo, email)) in destinatarios.iter().enumerate() {
            let cuerpo = format!(
                "Hola {},\n\n{}\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
                nombre_completo, mensaje_cuerpo
            );

            let email_msg = Message::builder()
                .from("No-Responder <no-responder@gia.fi.uba.ar>".parse().unwrap())
                .to(email
                    .parse()
                    .map_err(|_| format!("Email inválido: {}", email))?)
                .subject(asunto)
                .body(cuerpo)
                .map_err(|e| format!("Error construyendo email: {}", e))?;

            match transport.send(&email_msg) {
                Ok(_) => {
                    enviados_con_exito += 1;
                }
                Err(e) => {
                    println!("Fallo el envío para {}: {}", email, e);
                }
            }

            if indice + 1 < total {
                println!("Esperando 10 segundos para respetar el límite gratuito de Mailtrap...");
                std::thread::sleep(std::time::Duration::from_millis(10500));
            }
        }

        Ok(enviados_con_exito)
    }

    // WIP, aún queda adjuntarle el comprobante de reserva y actualizar el test acorde.
    pub fn enviar_notificacion_reserva_aprobada(
        email_destino: &str,
        profe_nombre: &str,
        id_reserva: &str,
        motivo: &str,
    ) -> Result<(), String> {
        let transport = Self::autenticar_smtp()?;

        let cuerpo = format!(
            "Hola {},\n\nSu solicitud de reserva con ID: '{}' y con el motivo '{}' ha sido APROBADA.\nPuede pasar a retirar el instrumental en el momento correspondiente.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
            profe_nombre, id_reserva, motivo
        );

        let email = Message::builder()
            .from("No-Responder <no-responder@gia.fi.uba.ar>".parse().unwrap())
            .to(email_destino
                .parse()
                .map_err(|_| "Email de destino inválido")?)
            .subject("Reserva de Instrumental Aprobada - Sistema de Gestión de Instrumental de Agrimensura")
            .body(cuerpo)
            .map_err(|e| format!("Error construyendo email: {}", e))?;

        transport
            .send(&email)
            .map_err(|e| format!("Error al enviar: {}", e))?;
        Ok(())
    }

    pub fn enviar_notificacion_reserva_rechazada(
        email_destino: &str,
        profe_nombre: &str,
        id_reserva: &str,
        motivo: &str,
    ) -> Result<(), String> {
        let transport = Self::autenticar_smtp()?;

        let cuerpo = format!(
            "Hola {},\n\nSu solicitud de reserva con ID: '{}' y con el motivo '{}' fue RECHAZADA.\n\nPor favor, póngase en contacto con el departamento por cualquier consulta.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
            profe_nombre, id_reserva, motivo
        );

        let email = Message::builder()
            .from("No-Responder <no-responder@gia.fi.uba.ar>".parse().unwrap())
            .to(email_destino
                .parse()
                .map_err(|_| "Email de destino inválido")?)
            .subject("Reserva de Instrumental Rechazada - Sistema de Gestión de Instrumental de Agrimensura")
            .body(cuerpo)
            .map_err(|e| format!("Error construyendo email: {}", e))?;

        transport
            .send(&email)
            .map_err(|e| format!("Error al enviar: {}", e))?;
        Ok(())
    }

    pub fn enviar_notificacion_profesor_aprobado(
        email_destino: &str,
        profe_nombre: &str,
    ) -> Result<(), String> {
        let transport = Self::autenticar_smtp()?;

        let cuerpo = format!(
            "Bienvenido/a {},\n\nSu solicitud de alta como Docente en el sistema GIA ha sido APROBADA por la administración.\nA partir de este momento puede ingresar al sistema y realizar solicitudes de reserva de instrumental.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
            profe_nombre
        );

        let email = Message::builder()
            .from("No-Responder <no-responder@gia.fi.uba.ar>".parse().unwrap())
            .to(email_destino
                .parse()
                .map_err(|_| "Email de destino inválido")?)
            .subject("Cuenta Habilitada - Sistema de Gestión de Instrumental de Agrimensura")
            .body(cuerpo)
            .map_err(|e| format!("Error construyendo email: {}", e))?;

        transport
            .send(&email)
            .map_err(|e| format!("Error al enviar: {}", e))?;
        Ok(())
    }

    pub fn enviar_notificacion_profesor_rechazado(
        email_destino: &str,
        profe_nombre: &str,
    ) -> Result<(), String> {
        let transport = Self::autenticar_smtp()?;

        let cuerpo = format!(
            "Hola {},\n\nLe informamos que su solicitud de alta como Docente en el sistema GIA ha sido RECHAZADA por la administración.\n\nSi considera que esto se debe a un error, por favor póngase en contacto con el Departamento.\n\nAtentamente,\nDepartamento de Agrimensura - FIUBA\n\n---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.",
            profe_nombre
        );

        let email = Message::builder()
            .from("No-Responder <no-responder@gia.fi.uba.ar>".parse().unwrap())
            .to(email_destino.parse().map_err(|_| "Email de destino inválido")?)
            .subject("Solicitud de Registro Rechazada - Sistema de Gestión de Instrumental de Agrimensura")
            .body(cuerpo)
            .map_err(|e| format!("Error construyendo email: {}", e))?;

        transport
            .send(&email)
            .map_err(|e| format!("Error al enviar: {}", e))?;
        Ok(())
    }
}
