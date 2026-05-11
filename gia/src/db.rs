//! Modulo para manejar la base de datos SQLite
use crate::errors::DbError;
use rusqlite::{Connection, Result as SqlResult};

/// Inicializa la base de datos y crea las tablas necesarias
pub fn init_db(db_path: &str) -> SqlResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Crear tabla cuentas (Usuarios del sistema)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cuentas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            segundo_nombre TEXT,
            apellido TEXT NOT NULL,
            segundo_apellido TEXT,
            email TEXT UNIQUE NOT NULL,
            legajo INTEGER UNIQUE NOT NULL,
            tipo TEXT NOT NULL CHECK (tipo IN ('alumno', 'profesor', 'admin')),
            password_hash TEXT NOT NULL,
            momento_creacion TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Crear tabla instrumentos (Inventario del departamento)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS instrumentos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            categoria TEXT,
            descripcion TEXT,
            stock INTEGER NOT NULL DEFAULT 0,
            estado TEXT DEFAULT 'disponible',
            manual_url TEXT, -- Ruta al PDF del manual
            imagen_principal_url TEXT -- Ruta a la primera imagen del equipo
        )",
        [],
    )?;

    // Crear tabla para las imagenes de los instrumentos
    conn.execute(
        "CREATE TABLE IF NOT EXISTS instrumento_imagen (
            instrumento_id INTEGER NOT NULL,
            orden INTEGER NOT NULL, -- Orden de la imagen (0 = principal, 1, 2, ...)
            imagen_url TEXT NOT NULL,
            PRIMARY KEY (instrumento_id, orden),
            FOREIGN KEY (instrumento_id) REFERENCES instrumentos(id) ON DELETE CASCADE
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
            FOREIGN KEY (id_usuario) REFERENCES cuentas(id)
        )",
        [],
    )?;

    // Crear tabla intermedia para relacionar reservas con instrumentos
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reserva_instrumentos (
            reserva_id INTEGER NOT NULL,
            instrumento_id INTEGER NOT NULL,
            cantidad INTEGER NOT NULL DEFAULT 1 CHECK (cantidad > 0),
            PRIMARY KEY (reserva_id, instrumento_id),
            FOREIGN KEY (reserva_id) REFERENCES reservas(id) ON DELETE CASCADE,
            FOREIGN KEY (instrumento_id) REFERENCES instrumentos(id)
        )",
        [],
    )?;
    Ok(conn)
}

/// Inserta un nuevo usuario en la tabla cuentas
pub fn insert_cuenta(
    conn: &Connection,
    nombre: &str,
    segundo_nombre: &str,
    apellido: &str,
    segundo_apellido: &str,
    email: &str,
    legajo: i32,
    tipo: &str,
    password_hash: &str,
) -> SqlResult<usize> {
    conn.execute(
        "INSERT INTO cuentas (nombre, segundo_nombre, apellido, segundo_apellido, email, legajo, tipo, password_hash)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![nombre, segundo_nombre, apellido, segundo_apellido, email, legajo, tipo, password_hash],
    )
}

/// Inserta un instrumento y sus imagenes asociadas de forma atomica
/// Las imagenes son opcionales. Si se proporcionan, la primera se usa como principal
pub fn insert_instrumento(
    conn: &mut Connection,
    nombre: &str,
    descripcion: &str,
    stock: i32,
    categoria: &str,
    manual_url: &str,
    imagenes: &[String], // Puede estar vacio. Si hay imagenes, la primera se asigna a imagen_principal_url
) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    let imagen_principal = imagenes.first().map(|s| s.as_str());

    // Insertar el instrumento en la tabla instrumentos
    tx.execute(
        "INSERT INTO instrumentos (nombre, descripcion, stock, categoria, manual_url, imagen_principal_url)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![nombre, descripcion, stock, categoria, manual_url, imagen_principal],
    )?;

    let instrumento_id = tx.last_insert_rowid();

    // Insertar las imagenes en orden en la tabla instrumento_imagen
    for (i, ruta) in imagenes.iter().enumerate() {
        tx.execute(
            "INSERT INTO instrumento_imagen (instrumento_id, imagen_url, orden)
             VALUES (?, ?, ?)",
            rusqlite::params![instrumento_id, ruta, i as i32,],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Actualiza un instrumento y reemplaza su galeria de imagenes
pub fn update_instrumento(
    conn: &mut Connection,
    id: i64,
    nombre: &str,
    descripcion: &str,
    stock: i32,
    categoria: &str,
    manual_url: &str,
    nuevas_imagenes: &[String],
) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    let imagen_principal = nuevas_imagenes.first().map(|s| s.as_str());

    tx.execute(
        "UPDATE instrumentos
         SET nombre = ?, descripcion = ?, stock = ?, categoria = ?, manual_url = ?, imagen_principal_url = ?
         WHERE id = ?",
        rusqlite::params![
            nombre,
            descripcion,
            stock,
            categoria,
            manual_url,
            imagen_principal,
            id,
        ],
    )?;

    tx.execute(
        "DELETE FROM instrumento_imagen WHERE instrumento_id = ?",
        [id],
    )?;

    for (orden, ruta) in nuevas_imagenes.iter().enumerate() {
        tx.execute(
            "INSERT INTO instrumento_imagen (instrumento_id, orden, imagen_url)
             VALUES (?, ?, ?)",
            rusqlite::params![id, orden as i32, ruta],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Inserta una nueva reserva y devuelve su id
pub fn insert_reserva(
    conn: &mut Connection,
    id_usuario: i32,
    fecha_inicio: &str,
    fecha_fin: &str,
    estado: &str,
    motivo: &str,
) -> SqlResult<i64> {
    let transaction = conn.transaction()?;

    transaction.execute(
        "INSERT INTO reservas (id_usuario, fecha_inicio, fecha_fin, estado, motivo)
         VALUES (?, ?, ?, ?, ?)",
        rusqlite::params![id_usuario, fecha_inicio, fecha_fin, estado, motivo],
    )?;

    let reserva_id =
        transaction.query_row("SELECT last_insert_rowid()", [], |row| row.get::<_, i64>(0))?;

    transaction.commit()?;

    Ok(reserva_id)
}

/// Asocia varios instrumentos a una reserva existente
pub fn add_instrumentos_to_reserva(
    conn: &mut Connection,
    reserva_id: i64,
    instrumentos: &[(i32, i32)],
) -> SqlResult<()> {
    let transaction = conn.transaction()?;

    for (instrumento_id, cantidad) in instrumentos {
        transaction.execute(
            "INSERT INTO reserva_instrumentos (reserva_id, instrumento_id, cantidad)
             VALUES (?, ?, ?)",
            rusqlite::params![reserva_id, instrumento_id, cantidad],
        )?;
    }

    transaction.commit()?;
    Ok(())
}
