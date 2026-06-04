//! Modulo para manejar la base de datos SQLite
use crate::constants::{TIPO_ADMIN, TIPO_ALUMNO, TIPO_PROFESOR};
use crate::errors::DbError;
use crate::service::image_storage::eliminar_imagen_por_direccion;
use rusqlite::{Connection, Result as SqlResult};
use std::collections::HashSet;

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

    // Crear tabla modelos_instrumentos (Catalogo de modelos)
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

/// Inserta un nuevo usuario en la tabla usuarios
pub fn insert_cuenta(
    conn: &Connection,
    nombre: &str,
    apellido: &str,
    email: &str,
    legajo: i32,
    tipo: &str,
    password_hash: &str,
) -> SqlResult<usize> {
    debug_assert!(
        [TIPO_ALUMNO, TIPO_PROFESOR, TIPO_ADMIN].contains(&tipo),
        "tipo de usuario invalido"
    );
    conn.execute(
        "INSERT INTO usuarios (nombre, apellido, email, legajo, tipo, password_hash)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![nombre, apellido, email, legajo, tipo, password_hash],
    )
}

/// Inserta un instrumento y sus imagenes asociadas de forma atomica
/// Las imagenes son opcionales. Si se proporcionan, la primera se usa como principal
pub fn insert_modelo(
    conn: &mut Connection,
    marca: &str,
    nombre_modelo: &str,
    descripcion: &str,
    categoria: &str,
    manual_url: &str,
    imagenes: &[String], // Puede estar vacio. Si hay imagenes, la primera se asigna a imagen_principal_url
) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    let imagen_principal = imagenes.first().map(|s| s.as_str());

    // Insertar el modelo en la tabla modelos
    tx.execute(
        "INSERT INTO modelos (marca, modelo, descripcion, categoria, manual_url, direccion_imagen_principal)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![marca, nombre_modelo, descripcion, categoria, manual_url, imagen_principal],
    )?;

    let modelo_id = tx.last_insert_rowid();

    // Insertar las imagenes en orden en la tabla modelo_imagen
    for (i, ruta) in imagenes.iter().enumerate() {
        tx.execute(
            "INSERT INTO modelo_imagen (modelo_id, orden, direccion_imagen)
             VALUES (?, ?, ?)",
            rusqlite::params![modelo_id, i as i32, ruta],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Actualiza un instrumento y reemplaza su galeria de imagenes
pub struct ModeloUpdate<'a> {
    pub id: i64,
    pub marca: &'a str,
    pub nombre_modelo: &'a str,
    pub descripcion: &'a str,
    pub categoria: &'a str,
    pub manual_url: &'a str,
    pub nuevas_imagenes: &'a [String],
}

pub fn update_modelo(conn: &mut Connection, data: ModeloUpdate<'_>) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    let imagen_principal = data.nuevas_imagenes.first().map(|s| s.as_str());

    tx.execute(
        "UPDATE modelos
         SET marca = ?, modelo = ?, descripcion = ?, categoria = ?, manual_url = ?, direccion_imagen_principal = ?
         WHERE id = ?",
        rusqlite::params![
            data.marca,
            data.nombre_modelo,
            data.descripcion,
            data.categoria,
            data.manual_url,
            imagen_principal,
            data.id,
        ],
    )?;

    tx.execute("DELETE FROM modelo_imagen WHERE modelo_id = ?", [data.id])?;

    for (orden, ruta) in data.nuevas_imagenes.iter().enumerate() {
        tx.execute(
            "INSERT INTO modelo_imagen (modelo_id, orden, direccion_imagen)
                 VALUES (?, ?, ?)",
            rusqlite::params![data.id, orden as i32, ruta],
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

/// Datos para insertar un ejemplar junto con sus imagenes de galeria.
pub struct EjemplarInsert<'a> {
    pub modelo_id: i64,
    pub numero_serie: Option<&'a str>,
    pub codigo_qr: Option<&'a str>,
    pub patrimonio: Option<&'a str>,
    pub observaciones: Option<&'a str>,
    pub accesorios: Option<&'a str>,
    pub esta_disponible: bool,
    pub ubicacion: Option<&'a str>,
    pub imagenes: &'a [String],
}

/// Inserta un ejemplar y sus imagenes de forma atomica.
pub fn insert_ejemplar(conn: &mut Connection, data: EjemplarInsert<'_>) -> Result<i64, DbError> {
    let tx = conn.transaction()?;
    let imagen_principal = data.imagenes.first().map(|s| s.as_str());

    tx.execute(
        "INSERT INTO ejemplares (
            modelo_id, numero_serie, codigo_qr, patrimonio, observaciones,
            accesorios, esta_disponible, ubicacion, direccion_imagen_principal
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            data.modelo_id,
            data.numero_serie,
            data.codigo_qr,
            data.patrimonio,
            data.observaciones,
            data.accesorios,
            data.esta_disponible,
            data.ubicacion,
            imagen_principal,
        ],
    )?;

    let ejemplar_id = tx.last_insert_rowid();

    for (orden, ruta) in data.imagenes.iter().enumerate() {
        tx.execute(
            "INSERT INTO imagen_ejemplar (ejemplar_id, orden, imagen_direccion)
             VALUES (?, ?, ?)",
            rusqlite::params![ejemplar_id, orden as i32, ruta],
        )?;
    }

    tx.commit()?;
    Ok(ejemplar_id)
}

/// Datos para actualizar un ejemplar y reemplazar su galeria de imagenes.
pub struct EjemplarUpdate<'a> {
    pub id: i64,
    pub modelo_id: i64,
    pub numero_serie: Option<&'a str>,
    pub codigo_qr: Option<&'a str>,
    pub patrimonio: Option<&'a str>,
    pub observaciones: Option<&'a str>,
    pub accesorios: Option<&'a str>,
    pub esta_disponible: bool,
    pub ubicacion: Option<&'a str>,
    pub imagenes: &'a [String],
}

/// Actualiza un ejemplar y reemplaza sus imagenes asociadas.
pub fn update_ejemplar(conn: &mut Connection, data: EjemplarUpdate<'_>) -> Result<(), DbError> {
    let tx = conn.transaction()?;

    let imagenes_previas = {
        let mut stmt =
            tx.prepare("SELECT imagen_direccion FROM imagen_ejemplar WHERE ejemplar_id = ?1")?;
        stmt.query_map([data.id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };

    tx.execute(
        "DELETE FROM imagen_ejemplar WHERE ejemplar_id = ?",
        [data.id],
    )?;

    let imagen_principal = data.imagenes.first().map(|s| s.as_str());

    tx.execute(
        "UPDATE ejemplares
         SET modelo_id = ?, numero_serie = ?, codigo_qr = ?, patrimonio = ?, observaciones = ?,
             accesorios = ?, esta_disponible = ?, ubicacion = ?, direccion_imagen_principal = ?
         WHERE id = ?",
        rusqlite::params![
            data.modelo_id,
            data.numero_serie,
            data.codigo_qr,
            data.patrimonio,
            data.observaciones,
            data.accesorios,
            data.esta_disponible,
            data.ubicacion,
            imagen_principal,
            data.id,
        ],
    )?;

    for (orden, ruta) in data.imagenes.iter().enumerate() {
        tx.execute(
            "INSERT INTO imagen_ejemplar (ejemplar_id, orden, imagen_direccion)
             VALUES (?, ?, ?)",
            rusqlite::params![data.id, orden as i32, ruta],
        )?;
    }

    tx.commit()?;

    let nuevas_direcciones: HashSet<&str> =
        data.imagenes.iter().map(|ruta| ruta.as_str()).collect();
    for direccion in imagenes_previas {
        if !nuevas_direcciones.contains(direccion.as_str()) {
            let _ = eliminar_imagen_por_direccion(&direccion);
        }
    }

    Ok(())
}

/// Asocia varios instrumentos a una reserva existente
pub fn add_instrumentos_to_reserva(
    conn: &mut Connection,
    reserva_id: i64,
    ejemplares: &[i32],
) -> SqlResult<()> {
    let transaction = conn.transaction()?;

    for ejemplar_id in ejemplares {
        transaction.execute(
            "INSERT INTO reserva_instrumentos (reserva_id, ejemplar_id)
             VALUES (?, ?)",
            rusqlite::params![reserva_id, ejemplar_id],
        )?;
    }

    transaction.commit()?;
    Ok(())
}
