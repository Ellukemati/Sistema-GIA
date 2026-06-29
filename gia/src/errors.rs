//! Modulo de errores

use std::fmt;

/// Errores generales relacionados con la base de datos y el almacenamiento de archivos
#[derive(Debug)]
pub enum DbError {
    SqlError(rusqlite::Error),
    ImageStorage(ImageStorageError),
    ManualStorage(ManualStorageError),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::SqlError(err) => write!(f, "Error de base de datos: {}", err),
            DbError::ImageStorage(err) => write!(f, "Error de almacenamiento de imagen: {}", err),
            DbError::ManualStorage(err) => write!(f, "Error de almacenamiento de manual: {}", err),
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

impl From<ManualStorageError> for DbError {
    fn from(err: ManualStorageError) -> Self {
        DbError::ManualStorage(err)
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
            ImageStorageError::InvalidImage(err) => write!(f, "Imagen invalida: {}", err),
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

/// Errores especificos del manejo de manuales de instrumental (PDFs)
#[derive(Debug)]
pub enum ManualStorageError {
    Io(std::io::Error),
    InvalidManual(String),
}

impl fmt::Display for ManualStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManualStorageError::Io(err) => {
                write!(f, "Error de Entrada/Salida en el documento: {}", err)
            }
            ManualStorageError::InvalidManual(err) => write!(f, "Documento invalido: {}", err),
        }
    }
}

impl std::error::Error for ManualStorageError {}

impl From<std::io::Error> for ManualStorageError {
    fn from(err: std::io::Error) -> Self {
        ManualStorageError::Io(err)
    }
}

/// Errores específicos de la generación y obtención de comprobantes de reserva
#[derive(Debug)]
pub enum ErrorComprobante {
    NoEncontrada,
    NoConfirmada,
    ErrorBD(rusqlite::Error),
    ErrorPdf(String),
}

impl fmt::Display for ErrorComprobante {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorComprobante::NoEncontrada => write!(f, "Reserva no encontrada"),
            ErrorComprobante::NoConfirmada => write!(
                f,
                "El comprobante solo está disponible para reservas confirmadas"
            ),
            ErrorComprobante::ErrorBD(e) => write!(f, "Error de base de datos: {}", e),
            ErrorComprobante::ErrorPdf(e) => write!(f, "Error generando PDF: {}", e),
        }
    }
}

impl std::error::Error for ErrorComprobante {}

impl From<rusqlite::Error> for ErrorComprobante {
    fn from(err: rusqlite::Error) -> Self {
        ErrorComprobante::ErrorBD(err)
    }
}
