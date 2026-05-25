/// BDD
pub const DB_PATH: &str = "gia.db";

/// Servidor
pub const ADDRESS: &str = "0.0.0.0:8080";

/// Rutas y almacenamiento de imagenes
pub const STATIC_DIR: &str = "static";
pub const UPLOADS_DIR: &str = "static/uploads";
pub const MODELOS_UPLOAD_DIR: &str = "static/uploads/modelos";
pub const AVATARES_UPLOAD_DIR: &str = "static/uploads/avatares";
pub const MODELOS_PUBLIC_PREFIX: &str = "/static/uploads/modelos";
pub const AVATARES_PUBLIC_PREFIX: &str = "/static/uploads/avatares";
pub const MODELOS_MAX_DIMENSION: u32 = 1600;
pub const AVATARES_MAX_DIMENSION: u32 = 512;

/// Tipos de usuario
pub const TIPO_ADMIN: &str = "A";
pub const TIPO_PROFESOR: &str = "P";
pub const TIPO_ALUMNO: &str = "S";
