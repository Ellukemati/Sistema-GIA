#![cfg(test)]
use gia::service::mail_service::MailService;

// Pruebas de integración para el servicio de mail. Se ignoran por defecto en 'cargo test' para no requerir internet obligatoriamente
// y para no agotar el límite gratuito de Mailtrap rápido. Para correrlos, usar 'cargo test -- --ignored'.
// Contienen pausas de unos 10 segundos entre cada envío para respetar el límite de 1 mail cada 10 segundos del plan gratuito de Mailtrap.
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
        "Falló el envío de notificación de reserva aprobada"
    );

    // Pausa obligatoria para respetar el límite de Mailtrap
    println!("Esperando 10.5 segundos para el siguiente envío...");
    std::thread::sleep(std::time::Duration::from_millis(10500));

    // CASO 2: Notificación de Reserva Rechazada
    let resultado_reserva_rechazada = MailService::enviar_notificacion_reserva_rechazada(
        "docentest@fi.uba.ar",
        "Juan Pérez",
        "42",
        "Para dar clases de testing.",
    );
    assert!(
        resultado_reserva_rechazada.is_ok(),
        "Falló el envío de notificación de reserva rechazada"
    );

    std::thread::sleep(std::time::Duration::from_millis(10500));

    // CASO 3: Notificación de Docente Aprobado
    let resultado_profe_aprobado =
        MailService::enviar_notificacion_profesor_aprobado("docentest@fi.uba.ar", "Carlos Gómez");
    assert!(
        resultado_profe_aprobado.is_ok(),
        "Falló el envío de notificación de profesor aprobado"
    );

    std::thread::sleep(std::time::Duration::from_millis(10500));

    // CASO 4: Notificación de Profesor Rechazado
    let resultado_profe_rechazado =
        MailService::enviar_notificacion_profesor_rechazado("docentest@fi.uba.ar", "Aníbal López");
    assert!(
        resultado_profe_rechazado.is_ok(),
        "Falló el envío de notificación de profesor rechazado"
    );

    // Por si se corre un test de mail inmediatamente después
    println!("Esperando 10.5 segundos para estar seguros...");
    std::thread::sleep(std::time::Duration::from_millis(10500));
}

#[test]
#[ignore]
fn test_enviar_comunicado_en_lote() {
    let destinatarios = vec![
        ("Carlitos Test".to_string(), "ctest@fi.uba.ar".to_string()),
        ("Test López".to_string(), "tlopez@fi.uba.ar".to_string()),
    ];

    let asunto = "Test de Integración de envío de mails";
    let cuerpo = "Esto es un mail de prueba automático.";

    let resultado = MailService::enviar_comunicado_lote(&destinatarios, asunto, cuerpo);

    assert!(resultado.is_ok(), "El servicio SMTP falló");

    let cantidad_exitos = resultado.unwrap();
    assert_eq!(
        cantidad_exitos, 2,
        "Se esperaban 2 envíos exitosos, pero se procesaron: {}",
        cantidad_exitos
    );

    // Por si se corre un test de mail inmediatamente después
    println!("Esperando 10.5 segundos para estar seguros...");
    std::thread::sleep(std::time::Duration::from_millis(10500));
}

#[test]
fn test_extraccion_destinatarios_con_emails_invalidos() {
    let destinatarios_con_error = vec![("Matias".to_string(), "email-sin-arroba.com".to_string())];

    let resultado =
        MailService::enviar_comunicado_lote(&destinatarios_con_error, "Asunto", "Cuerpo");

    assert!(
        resultado.is_err(),
        "Debería haber fallado debido al formato de email inválido"
    );
}
