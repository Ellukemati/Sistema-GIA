#![cfg(test)]
use gia::constants::MOCK_MAILS;
use gia::service::mail_service::MailService;

// Pruebas de integración para el servicio de mail. Se ignoran por defecto en 'cargo test' para no requerir internet obligatoriamente.
// Se corren con 'cargo test -- --ignored'.
// Se adaptan automáticamente al estado de la constante `MOCK_MAILS`.
// Si el mock está en false, se conectan a la API de Mailtrap usando la red y aplicando las esperas.
// Si el mock está en true, corren al instante simulando el éxito sin llamadas de red ni demoras.
#[test]
#[ignore]
fn test_circuito_de_notificaciones() {
    // CASO 1: Notificación de Reserva Aprobada
    let resultado_reserva_aprobada = MailService::enviar_notificacion_reserva_aprobada(
        "docentest@fi.uba.ar",
        "Juan Pérez",
        "42",
        "Uso de Estación Total para testeo.",
    );
    assert!(
        resultado_reserva_aprobada.is_ok(),
        "Falló la notificación de reserva aprobada"
    );

    // Solo espera los 10.5 segundos si estamos pegándole a la API de Mailtrap
    if !MOCK_MAILS {
        println!("Esperando 10.5 segundos para el siguiente envío real en Mailtrap...");
        std::thread::sleep(std::time::Duration::from_millis(10500));
    }

    // CASO 2: Notificación de Reserva Rechazada
    let resultado_reserva_rechazada = MailService::enviar_notificacion_reserva_rechazada(
        "docentest@fi.uba.ar",
        "Juan Pérez",
        "42",
        "Para dar clases de testing.",
    );
    assert!(
        resultado_reserva_rechazada.is_ok(),
        "Falló la notificación de reserva rechazada"
    );

    if !MOCK_MAILS {
        std::thread::sleep(std::time::Duration::from_millis(10500));
    }

    // CASO 3: Notificación de Docente Aprobado
    let resultado_profe_aprobado =
        MailService::enviar_notificacion_profesor_aprobado("docentest@fi.uba.ar", "Carlos Gómez");
    assert!(
        resultado_profe_aprobado.is_ok(),
        "Falló la notificación de profesor aprobado"
    );

    if !MOCK_MAILS {
        std::thread::sleep(std::time::Duration::from_millis(10500));
    }

    // CASO 4: Notificación de Profesor Rechazado
    let resultado_profe_rechazado =
        MailService::enviar_notificacion_profesor_rechazado("docentest@fi.uba.ar", "Aníbal López");
    assert!(
        resultado_profe_rechazado.is_ok(),
        "Falló la notificación de profesor rechazado"
    );

    if !MOCK_MAILS {
        println!("Esperando 10.5 segundos finales para liberar el canal SMTP...");
        std::thread::sleep(std::time::Duration::from_millis(10500));
    }
}

#[test]
#[ignore]
fn test_enviar_comunicado_en_lote() {
    let destinatarios = vec![
        ("Carlitos Test".to_string(), "ctest@fi.uba.ar".to_string()),
        ("Test López".to_string(), "tlopez@fi.uba.ar".to_string()),
    ];

    let asunto = "Test de envío de comunicados en lote";
    let cuerpo = "Esto es un mail de prueba automático.";

    let resultado = MailService::enviar_comunicado_lote(&destinatarios, asunto, cuerpo);

    assert!(resultado.is_ok(), "El servicio SMTP falló");

    let cantidad_exitos = resultado.unwrap();

    // Si es mock, la simulación devuelve la cantidad total procesada (2)
    assert_eq!(
        cantidad_exitos, 2,
        "Se esperaban 2 envíos exitosos, pero se procesaron: {}",
        cantidad_exitos
    );

    if !MOCK_MAILS {
        println!("Esperando 10.5 segundos para liberar el canal SMTP...");
        std::thread::sleep(std::time::Duration::from_millis(10500));
    }
}

#[test]
fn test_extraccion_destinatarios_con_emails_invalidos() {
    let destinatarios_con_error = vec![("Matias".to_string(), "email-sin-arroba.com".to_string())];

    let resultado =
        MailService::enviar_comunicado_lote(&destinatarios_con_error, "Asunto", "Cuerpo");

    if MOCK_MAILS {
        // En modo mock, no hay validación estricta y el lote simula éxito devolviendo Ok(1)
        assert!(
            resultado.is_ok(),
            "En modo MOCK debería haber devuelto Ok con la simulación del lote"
        );
        assert_eq!(
            resultado.unwrap(),
            1,
            "Se esperaba que simule exitosamente 1 envío"
        );
    } else {
        // En modo real, Lettre parsea el string roto y devuelve Err
        assert!(
            resultado.is_err(),
            "En modo REAL debería haber fallado debido al formato de email inválido"
        );
    }
}
