/// Rutas y optimizacion de imagenes
pub const STATIC_DIR: &str = "static";
pub const INSTRUMENTOS_MAX_DIMENSION: u32 = 1200;
pub const AVATARES_MAX_DIMENSION: u32 = 256;

/// Rutas estáticas de logos
pub const PATH_LOGO_FIUBA: &str = "static/img/logo_fiuba.jpeg";
pub const PATH_LOGO_FIUBA_TRANSPARENTE: &str = "static/img/logo_fiuba_transparente.png";
pub const PATH_LOGO_AGRIMENSURA: &str = "static/img/logo_depto_agrimensura.jpeg";
pub const PATH_LOGO_AGRIMENSURA_TRANSPARENTE: &str =
    "static/img/logo_depto_agrimensura_transparente.png";
pub const PATH_LOGO_GIA: &str = "static/img/logo_gia.jpeg";
pub const PATH_LOGO_GIA_TRANSPARENTE: &str = "static/img/logo_gia_transparente.png";

/// Limites de almacenamiento (en bytes)
pub const MANUALES_MAX_SIZE: usize = 16_777_216; // 16 MB

/// Tipos de usuario
pub const TIPO_ADMIN: &str = "A";
pub const TIPO_PROFESOR: &str = "P";

/// Estados de reserva
pub const ESTADO_PENDIENTE: &str = "pendiente";
pub const ESTADO_ACTIVA: &str = "activa";
pub const ESTADO_CANCELADA: &str = "cancelada";
pub const ESTADO_CONCLUIDA: &str = "concluida";

/// Seguridad y Tiempos de Expiración (Tokens de acceso y restablecimiento)
pub const EXPIRACION_SESION: i64 = 7200; // Sesiones de 2 horas de duración
pub const EXPIRACION_RESTABLECIMIENTO_PASSWORD_SEGUNDOS: i64 = 900; // 15 minutos
pub const EXPIRACION_INVITACION_SEGUNDOS: i64 = 86400; // 24 horas
pub const BCRYPT_COST_FACTOR: u32 = 4;

/// Textos de correo electrónico
pub const MAIL_EMISOR: &str = "No-Responder <no-responder@gia.fi.uba.ar>";
pub const FIRMA_INSTITUCIONAL: &str = "Atentamente,\nDepartamento de Agrimensura - FIUBA";
pub const PIE_AUTOMATICO: &str = "---\nEste es un mensaje automático enviado por el sistema GIA. Por favor, no responda a este correo.";

/// Configuración de constantes para testing
// Si MOCK_MAILS es true, se imprime el contenido del mail en la consola en lugar de enviarlo realmente,
// y se simula que todos los envíos fueron exitosos. Para desarrollar sin agotar la cuota de Mailtrap.
pub const MOCK_MAILS: bool = true;
// Si PDF_TESTING es true, se generan los PDFs y se guardan en disco.
pub const PDF_TESTING: bool = true;
