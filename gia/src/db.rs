//! Modulo para manejar la base de datos SQLite
use crate::constants::{TIPO_ADMIN, TIPO_ALUMNO, TIPO_PROFESOR};
use rusqlite::{Connection, Result as SqlResult};

/// Inicializa la base de datos y crea las tablas necesarias
pub fn init_db(db_path: &str) -> SqlResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    let tipos_usuario = format!("'{}', '{}', '{}'", TIPO_ALUMNO, TIPO_PROFESOR, TIPO_ADMIN);

    // Crear tabla cuentas (Usuarios del sistema)
    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS usuarios (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            apellido TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL,
            legajo INTEGER UNIQUE NOT NULL,
            tipo TEXT NOT NULL CHECK (tipo IN ({})),
            password_hash TEXT NOT NULL,
            momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
            direccion_avatar TEXT
        )",
            tipos_usuario
        ),
        [],
    )?;

    // Crear tabla modelos (Catalogo de modelos)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS modelos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            marca TEXT NOT NULL,
            nombre_modelo TEXT NOT NULL,
            categoria TEXT,
            descripcion TEXT,
            manual_url TEXT,
            direccion_imagen_principal TEXT
        )",
        [],
    )?;

    // Crear tabla para las imagenes asociadas a un modelo
    conn.execute(
        "CREATE TABLE IF NOT EXISTS modelo_imagen (
            modelo_id INTEGER NOT NULL,
            orden INTEGER NOT NULL, -- Orden de la imagen (0 = principal, 1, 2, ...)
            direccion_imagen TEXT NOT NULL,
            PRIMARY KEY (modelo_id, orden),
            FOREIGN KEY (modelo_id) REFERENCES modelos(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Crear tabla ejemplares (Cada entrada unica del inventario)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ejemplares (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            modelo_id INTEGER NOT NULL,
            numero_serie TEXT UNIQUE,
            codigo_qr TEXT UNIQUE,
            patrimonio TEXT UNIQUE,
            observaciones TEXT,
            accesorios TEXT,
            esta_disponible BOOLEAN DEFAULT TRUE,
            ubicacion TEXT,
            direccion_imagen_principal TEXT,
            FOREIGN KEY (modelo_id) REFERENCES modelos(id) ON DELETE RESTRICT
        )",
        [],
    )?;

    // Crear tabla para las imagenes asociadas a un ejemplar
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ejemplar_imagen (
            ejemplar_id INTEGER NOT NULL,
            orden INTEGER NOT NULL, -- Orden de la imagen (0 = principal, 1, 2, ...)
            direccion_imagen TEXT NOT NULL,
            PRIMARY KEY (ejemplar_id, orden),
            FOREIGN KEY (ejemplar_id) REFERENCES ejemplares(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Crear tabla reservas (Prestamos de instrumentos)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reservas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            id_usuario INTEGER NOT NULL,
            fecha_inicio TEXT NOT NULL,
            fecha_fin TEXT NOT NULL,
            estado TEXT NOT NULL CHECK (estado IN ('pendiente', 'activa', 'concluida', 'cancelada')),
            motivo TEXT,
            momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (id_usuario) REFERENCES usuarios(id)
        )",
        [],
    )?;

    // Crear tabla intermedia para relacionar reservas con ejemplares (sin cantidades)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reserva_ejemplar (
            reserva_id INTEGER NOT NULL,
            ejemplar_id INTEGER NOT NULL,
            PRIMARY KEY (reserva_id, ejemplar_id),
            FOREIGN KEY (reserva_id) REFERENCES reservas(id) ON DELETE CASCADE,
            FOREIGN KEY (ejemplar_id) REFERENCES ejemplares(id)
        )",
        [],
    )?;
    Ok(conn)
}
