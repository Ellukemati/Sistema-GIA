//! Modulo para manejar la base de datos SQLite
use crate::constants::{BCRYPT_COST_FACTOR, TIPO_ADMIN, TIPO_PROFESOR};
use rusqlite::{Connection, Result as SqlResult};

/// Inicializa la base de datos y crea las tablas necesarias
pub fn init_db(db_path: &str) -> SqlResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    crear_config_y_usuarios(&conn)?;
    crear_modelos_y_ejemplares(&conn)?;
    crear_imagenes(&conn)?;
    crear_reservas(&conn)?;
    crear_tokens(&conn)?;
    crear_admin_maestro(&conn)?;

    Ok(conn)
}

fn crear_config_y_usuarios(conn: &Connection) -> SqlResult<()> {
    // Configuracion guarda la fecha de la ultima sincronizacion de las reservas
    conn.execute(
        "CREATE TABLE IF NOT EXISTS configuracion (clave TEXT PRIMARY KEY, valor TEXT)",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO configuracion (clave, valor) VALUES ('ultima_sincronizacion', '2000-01-01')",
        [],
    )?;

    let tipos = format!("'{}', '{}'", TIPO_PROFESOR, TIPO_ADMIN);
    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS usuarios (
            id INTEGER PRIMARY KEY AUTOINCREMENT, nombre TEXT NOT NULL, apellido TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL, legajo INTEGER UNIQUE NOT NULL,
            tipo TEXT NOT NULL CHECK (tipo IN ({})), password_hash TEXT NOT NULL,
            aprobado BOOLEAN DEFAULT 0, momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
            avatar_blob BLOB, avatar_mime TEXT
        )",
            tipos
        ),
        [],
    )?;
    Ok(())
}

fn crear_modelos_y_ejemplares(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS modelos (
            id INTEGER PRIMARY KEY AUTOINCREMENT, marca TEXT NOT NULL, nombre_modelo TEXT NOT NULL,
            categoria TEXT, descripcion TEXT, manual_blob BLOB, manual_mime TEXT,
            eliminado BOOLEAN NOT NULL DEFAULT 0
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ejemplares (
            id INTEGER PRIMARY KEY AUTOINCREMENT, modelo_id INTEGER NOT NULL,
            numero_serie TEXT UNIQUE, codigo_qr TEXT UNIQUE, patrimonio TEXT UNIQUE,
            observaciones TEXT, accesorios TEXT, esta_disponible BOOLEAN DEFAULT TRUE,
            ubicacion TEXT, eliminado BOOLEAN NOT NULL DEFAULT 0,
            FOREIGN KEY (modelo_id) REFERENCES modelos(id) ON DELETE RESTRICT
        )",
        [],
    )?;
    Ok(())
}

fn crear_imagenes(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS modelo_imagen (
            modelo_id INTEGER NOT NULL, orden INTEGER NOT NULL,
            imagen_blob BLOB NOT NULL, imagen_mime TEXT NOT NULL,
            PRIMARY KEY (modelo_id, orden),
            FOREIGN KEY (modelo_id) REFERENCES modelos(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ejemplar_imagen (
            ejemplar_id INTEGER NOT NULL, orden INTEGER NOT NULL,
            imagen_blob BLOB NOT NULL, imagen_mime TEXT NOT NULL,
            PRIMARY KEY (ejemplar_id, orden),
            FOREIGN KEY (ejemplar_id) REFERENCES ejemplares(id) ON DELETE CASCADE
        )",
        [],
    )?;
    Ok(())
}

fn crear_reservas(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reservas (
            id INTEGER PRIMARY KEY AUTOINCREMENT, id_usuario INTEGER NOT NULL,
            fecha_inicio TEXT NOT NULL, fecha_fin TEXT NOT NULL,
            estado TEXT NOT NULL CHECK (estado IN ('pendiente', 'activa', 'concluida', 'cancelada')),
            motivo TEXT, momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
            momento_confirmacion TEXT, id_admin_aprobador INTEGER,
            FOREIGN KEY (id_usuario) REFERENCES usuarios(id),
            FOREIGN KEY (id_admin_aprobador) REFERENCES usuarios(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reserva_ejemplar (
            reserva_id INTEGER NOT NULL, ejemplar_id INTEGER NOT NULL,
            PRIMARY KEY (reserva_id, ejemplar_id),
            FOREIGN KEY (reserva_id) REFERENCES reservas(id) ON DELETE CASCADE,
            FOREIGN KEY (ejemplar_id) REFERENCES ejemplares(id)
        )",
        [],
    )?;
    Ok(())
}

fn crear_tokens(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sesiones (
            token TEXT PRIMARY KEY, id_usuario INTEGER NOT NULL,
            momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (id_usuario) REFERENCES usuarios(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tokens_restablecimiento_contrasena (
            id_usuario INTEGER NOT NULL, token TEXT NOT NULL UNIQUE, expira_en INTEGER NOT NULL,
            PRIMARY KEY (id_usuario), FOREIGN KEY (id_usuario) REFERENCES usuarios(id) ON DELETE CASCADE
        )",
        [],
    )?;
    /*
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tokens_invitacion (
            email TEXT NOT NULL, token TEXT NOT NULL UNIQUE, tipo TEXT NOT NULL,
            expira_en INTEGER NOT NULL, PRIMARY KEY (email)
        )",
        [],
    )?;
    */
    Ok(())
}

fn crear_admin_maestro(conn: &Connection) -> SqlResult<()> {
    let admin_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM usuarios WHERE tipo = ?1",
        [TIPO_ADMIN],
        |row| row.get(0),
    )?;

    if admin_count == 0 {
        let email =
            std::env::var("ADMIN_INICIAL_EMAIL").unwrap_or_else(|_| "admin@fi.uba.ar".to_string());
        let pass =
            std::env::var("ADMIN_INICIAL_PASSWORD").unwrap_or_else(|_| "admin123".to_string());
        let hash = bcrypt::hash(&pass, BCRYPT_COST_FACTOR).unwrap_or_default();

        conn.execute(
            "INSERT INTO usuarios (nombre, apellido, email, legajo, tipo, password_hash, aprobado)
             VALUES ('Admin', 'Maestro', ?1, 0, ?2, ?3, 1)",
            rusqlite::params![email, TIPO_ADMIN, hash],
        )?;

        crate::logger::info(&format!(
            "Admin maestro creado exitosamente (Email: {} | Clave: {})",
            email, pass
        ));
    }
    Ok(())
}
