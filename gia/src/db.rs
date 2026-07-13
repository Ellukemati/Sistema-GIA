//! Modulo para manejar la base de datos SQLite
use crate::constants::{TIPO_ADMIN, TIPO_PROFESOR};
use rusqlite::{Connection, Result as SqlResult};

/// Inicializa la base de datos y crea las tablas necesarias
pub fn init_db(db_path: &str) -> SqlResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    let tipos_usuario = format!("'{}', '{}'", TIPO_PROFESOR, TIPO_ADMIN);

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
            aprobado BOOLEAN DEFAULT 0,
            momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
            avatar_blob BLOB,
            avatar_mime TEXT
        )",
            tipos_usuario
        ),
        [],
    )?;

    // Crear tabla modelos para el catalogo de modelos de instrumentos
    conn.execute(
        "CREATE TABLE IF NOT EXISTS modelos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            marca TEXT NOT NULL,
            nombre_modelo TEXT NOT NULL,
            categoria TEXT,
            descripcion TEXT,
            manual_blob BLOB,
            manual_mime TEXT,
            eliminado BOOLEAN NOT NULL DEFAULT 0
        )",
        [],
    )?;

    // Crear tabla para las imagenes asociadas a un modelo
    conn.execute(
        "CREATE TABLE IF NOT EXISTS modelo_imagen (
            modelo_id INTEGER NOT NULL,
            orden INTEGER NOT NULL, -- Orden de la imagen (0 = principal, 1, 2, ...)
            imagen_blob BLOB NOT NULL,
            imagen_mime TEXT NOT NULL,
            PRIMARY KEY (modelo_id, orden),
            FOREIGN KEY (modelo_id) REFERENCES modelos(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Crear tabla ejemplares para cada entrada unica del inventario de instrumentos
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
            eliminado BOOLEAN NOT NULL DEFAULT 0,
            FOREIGN KEY (modelo_id) REFERENCES modelos(id) ON DELETE RESTRICT
        )",
        [],
    )?;

    // Crear tabla para las imagenes asociadas a un ejemplar
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ejemplar_imagen (
            ejemplar_id INTEGER NOT NULL,
            orden INTEGER NOT NULL, -- Orden de la imagen (0 = principal, 1, 2, ...)
            imagen_blob BLOB NOT NULL,
            imagen_mime TEXT NOT NULL,
            PRIMARY KEY (ejemplar_id, orden),
            FOREIGN KEY (ejemplar_id) REFERENCES ejemplares(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Crear tabla reservas
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reservas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            id_usuario INTEGER NOT NULL,
            fecha_inicio TEXT NOT NULL,
            fecha_fin TEXT NOT NULL,
            estado TEXT NOT NULL CHECK (estado IN ('pendiente', 'activa', 'concluida', 'cancelada')),
            motivo TEXT,
            momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP, -- Timestamp exacto del momento de la creación de la solicitud de reserva (Auditoría inmutable)
            momento_confirmacion TEXT,  -- Timestamp exacto de la confirmación (Auditoría inmutable)
            id_admin_aprobador INTEGER, -- ID del administrador que confirme la reserva
            FOREIGN KEY (id_usuario) REFERENCES usuarios(id),
            FOREIGN KEY (id_admin_aprobador) REFERENCES usuarios(id)
        )",
        [],
    )?;

    // Crear tabla intermedia para relacionar reservas con ejemplares
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

    // Crear tabla sesiones
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sesiones (
            token TEXT PRIMARY KEY,
            id_usuario INTEGER NOT NULL,
            momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (id_usuario) REFERENCES usuarios(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Crear tabla tokens para restablecimiento de contraseñas
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tokens_restablecimiento_contrasena (
            id_usuario INTEGER NOT NULL,
            token TEXT NOT NULL UNIQUE,
            expira_en INTEGER NOT NULL, -- Timestamp Unix Epoch en segundos
            PRIMARY KEY (id_usuario),
            FOREIGN KEY (id_usuario) REFERENCES usuarios(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Crear tabla tokens para invitación de nuevos usuarios
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tokens_invitacion (
            email TEXT NOT NULL,
            token TEXT NOT NULL UNIQUE,
            tipo TEXT NOT NULL,         -- 'A' para Admin, 'P' para Profesor = Docente
            expira_en INTEGER NOT NULL, -- Timestamp Unix Epoch en segundos
            PRIMARY KEY (email)
        )",
        [],
    )?;

    // Creacion de un admin por defecto
    let admin_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM usuarios WHERE tipo = ?1",
        [TIPO_ADMIN],
        |row| row.get::<_, i32>(0),
    )?;

    if admin_count == 0 {
        let admin_email =
            std::env::var("ADMIN_INICIAL_EMAIL").unwrap_or_else(|_| "admin@fi.uba.ar".to_string());
        let admin_pass =
            std::env::var("ADMIN_INICIAL_PASSWORD").unwrap_or_else(|_| "admin123".to_string());

        let admin_hash = bcrypt::hash(&admin_pass, 4).unwrap_or_default();

        conn.execute(
            "INSERT INTO usuarios (nombre, apellido, email, legajo, tipo, password_hash, aprobado)
             VALUES ('Admin', 'Maestro', ?1, 0, ?2, ?3, 1)",
            rusqlite::params![admin_email, TIPO_ADMIN, admin_hash],
        )?;

        crate::logger::info(&format!(
            "Admin maestro creado exitosamente (Email: {} | Clave: {})",
            admin_email, admin_pass
        ));
    }

    Ok(conn)
}
