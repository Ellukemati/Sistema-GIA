//! Modulo de errores
use std::fmt;

/// Errores especificos de operaciones en la base de datos
#[derive(Debug)]
pub enum DbError {
    /// Error de SQLite
    SqlError(rusqlite::Error),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::SqlError(err) => write!(f, "Error de base de datos: {}", err),
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        DbError::SqlError(err)
    }
}
