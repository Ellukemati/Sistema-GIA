/// BDD
pub const DB_PATH: &str = "gia.db";

/// Servidor
pub const ADDRESS: &str = "0.0.0.0:8080";

/// Rutas y optimizacion de imagenes
pub const STATIC_DIR: &str = "static";
pub const INSTRUMENTOS_MAX_DIMENSION: u32 = 1200;
pub const AVATARES_MAX_DIMENSION: u32 = 256;

/// Limites de almacenamiento (en bytes)
pub const MANUALES_MAX_SIZE: usize = 16_777_216; // 16 MB

/// Tipos de usuario
pub const TIPO_ADMIN: &str = "A";
pub const TIPO_PROFESOR: &str = "P";

/// Seguridad y Tiempos de Expiración (Tokens de acceso y restablecimiento)
pub const EXPIRACION_RECUPERACION_SEGUNDOS: i64 = 900; // 15 minutos
pub const EXPIRACION_INVITACION_SEGUNDOS: i64 = 86400; // 24 horas
pub const BCRYPT_COST_FACTOR: u32 = 4;

/// Configuración de Mailtrap (Testing SMTP). Cuenta con plan gratuito de Matias Dundic con 1 mail permitido cada 10 segundos y máximo de 50 mensuales.
pub const MAILTRAP_USER: &str = "ca4badc18b73e0";
pub const MAILTRAP_PASSWORD: &str = "c56ae035ae868b";
// Si MOCK_MAILS es true, se imprime el contenido del mail en la consola en lugar de enviarlo realmente,
// y se simula que todos los envíos fueron exitosos. Para desarrollar sin agotar la cuota de Mailtrap.
pub const MOCK_MAILS: bool = true;
