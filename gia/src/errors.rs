//! Modulo de errores

use std::fmt;

/// Errores especificos de operaciones en la base de datos
#[allow(dead_code)]
#[derive(Debug)]
pub enum DbError {
    /// Error de SQLite
    SqlError(rusqlite::Error),
    /// Error al manejar archivos de imagen asociados a la base de datos
    ImageStorage(ImageStorageError),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::SqlError(err) => write!(f, "Error de base de datos: {}", err),
            DbError::ImageStorage(err) => write!(f, "Error de imagen: {}", err),
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        DbError::SqlError(err)
    }
}

impl From<ImageStorageError> for DbError {
    fn from(err: ImageStorageError) -> Self {
        DbError::ImageStorage(err)
    }
}

/// Errores especificos del manejo local de imagenes
#[derive(Debug)]
pub enum ImageStorageError {
    Io(std::io::Error),
    Decode(image::ImageError),
    InvalidImage(String),
}

impl fmt::Display for ImageStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageStorageError::Io(err) => write!(f, "No se pudo guardar la imagen: {}", err),
            ImageStorageError::Decode(err) => write!(f, "No se pudo procesar la imagen: {}", err),
            ImageStorageError::InvalidImage(msg) => write!(f, "Imagen invalida: {}", msg),
        }
    }
}

impl std::error::Error for ImageStorageError {}

impl From<std::io::Error> for ImageStorageError {
    fn from(err: std::io::Error) -> Self {
        ImageStorageError::Io(err)
    }
}

impl From<image::ImageError> for ImageStorageError {
    fn from(err: image::ImageError) -> Self {
        ImageStorageError::Decode(err)
    }
}
