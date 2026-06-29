#![cfg(test)]
use gia::constants::MOCK_MAILS;
use gia::models::usuario::Usuario;
use gia::service::mail_service::MailService; // 🌟 Importamos el modelo de usuario real para el test de admins

// Ignorado para no enviar mails reales a Mailtrap durante pruebas automáticas (salvo usando cargo test -- --include-ignored),
// pero se puede ejecutar manualmente para verificar el flujo completo. MOCK_MAILS = true para no enviar mails a Mailtrap.
#[test]
#[ignore]
fn test_circuito_de_notificaciones() {
    // CASO 1: Notificación de Reserva Confirmada (Docente)
    let resultado_reserva_aprobada =
        MailService::enviar_notificacion_reserva_aprobada_con_comprobante(
            "docentest@fi.uba.ar",
            "Juan Pérez",
            "42",
            "Uso de Estación Total para testeo.",
            "desde el 18 de agosto hasta el 2 de octubre",
            &[], // bytes del PDF simulados
        );

    assert!(
        resultado_reserva_aprobada.is_ok(),
        "Falló la notificación de reserva aprobada para el docente"
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

    // CASO 3: Notificación de cuenta de Docente habilitada
    let resultado_profe_aprobado =
        MailService::enviar_notificacion_profesor_aprobado("docentest@fi.uba.ar", "Carlos Gómez");
    assert!(
        resultado_profe_aprobado.is_ok(),
        "Falló la notificación de profesor aprobado"
    );

    if !MOCK_MAILS {
        std::thread::sleep(std::time::Duration::from_millis(10500));
    }

    // CASO 4: Notificación de Solicitud de Registro de Docente  Rechazada
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

// Ignorado para no enviar mails reales a Mailtrap durante pruebas automáticas (salvo usando cargo test -- --include-ignored),
// pero se puede ejecutar manualmente para verificar el flujo completo. MOCK_MAILS = true para no enviar mails a Mailtrap.
#[test]
#[ignore]
fn test_notificacion_lote_administradores_reserva_aprobada() {
    use gia::constants::TIPO_ADMIN;

    let administradores_mock = vec![
        Usuario {
            id: 1,
            nombre: "Admin".to_string(),
            apellido: "Maestro".to_string(),
            email: "adminmaestro@fi.uba.ar".to_string(),
            legajo: 99991,
            tipo: TIPO_ADMIN.to_string(),
            password_hash: "hash_mock_1".to_string(),
            aprobado: true,
            momento_creacion: "2026-06-29 07:00:00".to_string(),
            avatar_blob: None,
            avatar_mime: None,
        },
        Usuario {
            id: 2,
            nombre: "Gestion".to_string(),
            apellido: "Instrumental".to_string(),
            email: "gestion@fi.uba.ar".to_string(),
            legajo: 99992,
            tipo: TIPO_ADMIN.to_string(),
            password_hash: "hash_mock_2".to_string(),
            aprobado: true,
            momento_creacion: "2026-06-29 07:00:00".to_string(),
            avatar_blob: None,
            avatar_mime: None,
        },
    ];

    let resultado_lote_admins =
        MailService::enviar_notificacion_reserva_aprobada_admins_con_comprobante(
            administradores_mock,
            "42",
            "Juan Pérez",
            "Uso de Estación Total para testeo.",
            "desde el 18 de agosto hasta el 2 de octubre",
            &[], // buffer de bytes de PDF simulado vacío
        );

    assert!(
        resultado_lote_admins.is_ok(),
        "Falló la notificación de reserva aprobada enviada al lote de administradores"
    );

    if !MOCK_MAILS {
        println!("Esperando 10.5 segundos de enfriamiento para Mailtrap...");
        std::thread::sleep(std::time::Duration::from_millis(10500));
    }
}

// Ignorado para no enviar mails reales a Mailtrap durante pruebas automáticas (salvo usando cargo test -- --include-ignored),
// pero se puede ejecutar manualmente para verificar el flujo completo. MOCK_MAILS = true para no enviar mails a Mailtrap.
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
        // En modo real, Lettre falla al parsear el mail y el servicio lo maneja devolviendo Ok(0)
        assert!(
            resultado.is_ok(),
            "El servicio debería manejar el error internamente y devolver Ok"
        );
        assert_eq!(
            resultado.unwrap(),
            0,
            "Se esperaba que devuelva 0 envíos exitosos debido al formato de email inválido"
        );
    }
}
