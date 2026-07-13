use crate::models::modelo::Modelo;
use crate::repository::ejemplar_repository::EjemplarRepository;
use crate::repository::image_repository::ImageRepository;
use crate::repository::modelo_repository::ModeloRepository;
use crate::repository::reserva_repository::ReservaRepository;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
pub struct ModeloService;

pub struct CrearModeloData {
    pub marca: String,
    pub nombre_modelo: String,
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
}
#[derive(Serialize)]
pub struct ModeloCardDTO {
    pub id: i64,
    pub nombre_modelo: String,
    pub categoria: Option<String>,
    pub imagen: Option<String>,
}

#[derive(Serialize)]
pub struct GrupoCategoriaDTO {
    pub categoria: String,
    pub modelos: Vec<ModeloCardDTO>,
}

impl ModeloService {
    pub fn actualizar_modelo(
        conn: &Connection,
        id: i64,
        data: CrearModeloData,
    ) -> Result<Modelo, String> {
        if data.nombre_modelo.trim().is_empty() {
            return Err("El nombre del modelo no puede estar vacio.".to_string());
        }

        match ModeloRepository::buscar_por_id(conn, id) {
            Ok(None) => return Err("Modelo no encontrado.".to_string()),
            Ok(Some(_)) => {}
            Err(e) => {
                return Err(format!("Error al buscar el modelo: {}", e));
            }
        }

        match ModeloRepository::actualizar(
            conn,
            id,
            &data.marca,
            &data.nombre_modelo,
            data.categoria.as_deref(),
            data.descripcion.as_deref(),
        ) {
            Ok(()) => Ok(Modelo {
                id,
                marca: data.marca,
                nombre_modelo: data.nombre_modelo,
                categoria: data.categoria,
                descripcion: data.descripcion,
                eliminado: false,
            }),
            Err(e) => Err(format!(
                "Error en la base de datos al actualizar modelo: {}",
                e
            )),
        }
    }

    pub fn crear_modelo(conn: &Connection, data: CrearModeloData) -> Result<Modelo, String> {
        if data.nombre_modelo.trim().is_empty() {
            return Err("El nombre del modelo no puede estar vacio.".to_string());
        }

        let modelo_temporal = Modelo {
            id: 0,
            marca: data.marca,
            nombre_modelo: data.nombre_modelo,
            categoria: data.categoria,
            descripcion: data.descripcion,
            eliminado: false,
        };

        match ModeloRepository::crear(conn, &modelo_temporal) {
            Ok(id_real) => Ok(Modelo {
                id: id_real,
                ..modelo_temporal
            }),
            Err(e) => Err(format!("Error en la base de datos al crear modelo: {}", e)),
        }
    }

    /// Elimina un modelo marcandolo como `eliminado`.
    /// Falla si el modelo no existe, si ya fue eliminado, o si tiene ejemplares
    /// vinculados que no fueron eliminados.
    pub fn eliminar_modelo(conn: &Connection, id: i64) -> Result<(), String> {
        let modelo = ModeloRepository::buscar_por_id(conn, id)
            .map_err(|e| format!("Error al buscar el modelo: {}", e))?
            .ok_or_else(|| "El modelo no existe.".to_string())?;

        if modelo.eliminado {
            return Err("El modelo ya fue eliminado.".to_string());
        }

        if EjemplarRepository::tiene_ejemplares_activos(conn, id)
            .map_err(|e| format!("Error al verificar ejemplares: {}", e))?
        {
            return Err("No se puede eliminar: el modelo tiene ejemplares vinculados.".to_string());
        }

        ModeloRepository::marcar_eliminado(conn, id)
            .map_err(|e| format!("Error en la base de datos al eliminar modelo: {}", e))?;

        Ok(())
    }

    /// Construye un `ModeloCardDTO` a partir de un `Modelo`, resolviendo la URL
    /// de su imagen principal (orden 0) si existe.
    fn card_de_modelo(conn: &Connection, modelo: Modelo) -> Result<ModeloCardDTO, String> {
        let tiene_imagen = ImageRepository::existe_imagen_principal_modelo(conn, modelo.id)
            .map_err(|e| {
                format!(
                    "Error al consultar la imagen del modelo {}: {}",
                    modelo.id, e
                )
            })?;

        let imagen = if tiene_imagen {
            Some(format!("/imagenes/modelos/{}/0", modelo.id))
        } else {
            None
        };

        Ok(ModeloCardDTO {
            id: modelo.id,
            nombre_modelo: modelo.nombre_modelo,
            categoria: modelo.categoria,
            imagen,
        })
    }

    /// Clave de comparacion: minusculas y sin diacriticos (tildes, dieresis, etc.).
    fn clave_categoria(s: &str) -> String {
        s.to_lowercase()
            .chars()
            .map(|c| match c {
                'á' | 'à' | 'ä' | 'â' => 'a',
                'é' | 'è' | 'ë' | 'ê' => 'e',
                'í' | 'ì' | 'ï' | 'î' => 'i',
                'ó' | 'ò' | 'ö' | 'ô' => 'o',
                'ú' | 'ù' | 'ü' | 'û' => 'u',
                'ñ' => 'n',
                other => other,
            })
            .collect()
    }

    /// Agrupa las tarjetas por categoria, conservando el orden de aparicion.
    /// La comparacion ignora mayusculas/minusculas y tildes; se conserva el
    /// nombre de la primera aparicion. Los modelos sin categoria quedan como
    /// "Sin categoria".
    fn agrupar_por_categoria(cards: Vec<ModeloCardDTO>) -> Vec<GrupoCategoriaDTO> {
        let mut grupos: Vec<GrupoCategoriaDTO> = Vec::new();
        let mut indice: HashMap<String, usize> = HashMap::new();

        for card in cards {
            let categoria = card
                .categoria
                .clone()
                .filter(|c| !c.trim().is_empty())
                .unwrap_or_else(|| "Sin categoria".to_string());

            let clave = Self::clave_categoria(&categoria);
            match indice.get(&clave) {
                Some(&i) => grupos[i].modelos.push(card),
                None => {
                    indice.insert(clave, grupos.len());
                    grupos.push(GrupoCategoriaDTO {
                        categoria,
                        modelos: vec![card],
                    });
                }
            }
        }

        grupos
    }

    /// Crea un vector de modelosCardDTO que tiene la info esencial para mostrar un modelo.
    pub fn listar_cards(conn: &Connection) -> Result<Vec<ModeloCardDTO>, String> {
        let modelos = ModeloRepository::listar_todos(conn)
            .map_err(|e| format!("Error al listar los modelos: {}", e))?;

        let mut cards = Vec::with_capacity(modelos.len());
        for modelo in modelos {
            cards.push(Self::card_de_modelo(conn, modelo)?);
        }

        Ok(cards)
    }

    /// Igual que `listar_cards`, pero agrupando las tarjetas por categoria.
    /// Los modelos sin categoria quedan como "Sin categoria".
    pub fn listar_cards_agrupadas(conn: &Connection) -> Result<Vec<GrupoCategoriaDTO>, String> {
        let cards = Self::listar_cards(conn)?;
        Ok(Self::agrupar_por_categoria(cards))
    }

    /// Lista, agrupados por categoria, solo los modelos que tienen al menos un
    /// ejemplar disponible en el rango de fechas indicado.
    pub fn listar_cards_disponibles_agrupadas(
        conn: &Connection,
        fecha_inicio: &str,
        fecha_fin: &str,
        buscar: &str,
    ) -> Result<Vec<GrupoCategoriaDTO>, String> {
        let modelos = if buscar.trim().is_empty() {
            ModeloRepository::listar_todos(conn)
        } else {
            ModeloRepository::buscar_por_nombre(conn, buscar)
        }
        .map_err(|e| format!("Error al listar los modelos: {}", e))?;

        let mut cards = Vec::new();
        for modelo in modelos {
            let ejemplares =
                EjemplarRepository::listar_por_modelo(conn, modelo.id).map_err(|e| {
                    format!("Error al listar ejemplares del modelo {}: {}", modelo.id, e)
                })?;

            let mut hay_disponible = false;
            for ejemplar in &ejemplares {
                if ReservaRepository::ejemplar_disponible(
                    conn,
                    ejemplar.id,
                    fecha_inicio,
                    fecha_fin,
                )
                .map_err(|e| e.to_string())?
                {
                    hay_disponible = true;
                    break;
                }
            }

            if hay_disponible {
                cards.push(Self::card_de_modelo(conn, modelo)?);
            }
        }

        Ok(Self::agrupar_por_categoria(cards))
    }

    pub fn listar_cards_filtradas(
        conn: &Connection,
        texto: &str,
    ) -> Result<Vec<GrupoCategoriaDTO>, String> {
        let modelos = ModeloRepository::buscar_por_nombre(conn, texto)
            .map_err(|e| format!("Error al buscar modelos: {}", e))?;

        let mut cards = Vec::with_capacity(modelos.len());

        for modelo in modelos {
            cards.push(Self::card_de_modelo(conn, modelo)?);
        }

        Ok(Self::agrupar_por_categoria(cards))
    }

    fn aplicar_categoria_y_orden(
        mut grupos: Vec<GrupoCategoriaDTO>,
        cat: &str,
        orden: &str,
    ) -> Vec<GrupoCategoriaDTO> {
        if !cat.trim().is_empty() {
            grupos.retain(|g| g.categoria.eq_ignore_ascii_case(cat));
        }

        match orden {
            "cat_desc" => {
                grupos.sort_by(|a, b| b.categoria.to_lowercase().cmp(&a.categoria.to_lowercase()));
            }
            _ => {
                grupos.sort_by(|a, b| a.categoria.to_lowercase().cmp(&b.categoria.to_lowercase()));
            }
        }

        grupos
    }

    pub fn filtrar_y_ordenar_cards(
        conn: &Connection,
        buscar: &str,
        cat: &str,
        orden: &str,
    ) -> Result<Vec<GrupoCategoriaDTO>, String> {
        let grupos = if buscar.trim().is_empty() {
            Self::listar_cards_agrupadas(conn)?
        } else {
            Self::listar_cards_filtradas(conn, buscar)?
        };

        Ok(Self::aplicar_categoria_y_orden(grupos, cat, orden))
    }

    pub fn filtrar_y_ordenar_cards_disponibles(
        conn: &Connection,
        fecha_inicio: &str,
        fecha_fin: &str,
        buscar: &str,
        cat: &str,
        orden: &str,
    ) -> Result<Vec<GrupoCategoriaDTO>, String> {
        let grupos =
            Self::listar_cards_disponibles_agrupadas(conn, fecha_inicio, fecha_fin, buscar)?;

        Ok(Self::aplicar_categoria_y_orden(grupos, cat, orden))
    }

    pub fn obtener_lista_categorias(conn: &Connection) -> Vec<String> {
        let mut lista = ModeloRepository::listar_todos(conn)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| m.categoria)
            .filter(|c| !c.trim().is_empty())
            .fold(Vec::new(), |mut acc, c| {
                if !acc
                    .iter()
                    .any(|x: &String| x.to_lowercase() == c.to_lowercase())
                {
                    acc.push(c);
                }
                acc
            });

        lista.sort_by_key(|a| a.to_lowercase());

        lista
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::modelo::Modelo;
    use crate::repository::image_repository::ImageRepository;
    use crate::repository::modelo_repository::ModeloRepository;
    use rusqlite::Connection;

    fn card(id: i64, nombre: &str, categoria: Option<&str>) -> ModeloCardDTO {
        ModeloCardDTO {
            id,
            nombre_modelo: nombre.to_string(),
            categoria: categoria.map(String::from),
            imagen: None,
        }
    }

    fn crear_db_test() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE modelos (
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
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE modelo_imagen (
                modelo_id INTEGER NOT NULL,
                orden INTEGER NOT NULL,
                imagen_blob BLOB NOT NULL,
                imagen_mime TEXT NOT NULL,
                PRIMARY KEY (modelo_id, orden)
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE ejemplares (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                modelo_id INTEGER NOT NULL,
                numero_serie TEXT UNIQUE,
                codigo_qr TEXT UNIQUE,
                patrimonio TEXT UNIQUE,
                observaciones TEXT,
                accesorios TEXT,
                esta_disponible BOOLEAN DEFAULT TRUE,
                ubicacion TEXT,
                eliminado BOOLEAN NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn insertar_ejemplar(conn: &Connection, modelo_id: i64, eliminado: bool) -> i64 {
        conn.execute(
            "INSERT INTO ejemplares (modelo_id, esta_disponible, eliminado)
             VALUES (?1, 1, ?2)",
            rusqlite::params![modelo_id, eliminado as i32],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insertar_modelo(conn: &Connection, nombre: &str, categoria: Option<&str>) -> i64 {
        let modelo = Modelo {
            id: 0,
            marca: "Marca".into(),
            nombre_modelo: nombre.into(),
            categoria: categoria.map(String::from),
            descripcion: None,
            eliminado: false,
        };
        ModeloRepository::crear(conn, &modelo).unwrap()
    }

    fn modelo_test(id: i64) -> Modelo {
        Modelo {
            id,
            marca: "Marca".into(),
            nombre_modelo: "Modelo X".into(),
            categoria: Some("Cuerdas".into()),
            descripcion: None,
            eliminado: false,
        }
    }

    #[test]
    fn agrupar_por_categoria_lista_vacia() {
        let grupos = ModeloService::agrupar_por_categoria(vec![]);
        assert!(grupos.is_empty());
    }

    #[test]
    fn agrupar_por_categoria_una_sola_tarjeta() {
        let grupos = ModeloService::agrupar_por_categoria(vec![card(1, "Violín", Some("Cuerdas"))]);

        assert_eq!(grupos.len(), 1);
        assert_eq!(grupos[0].categoria, "Cuerdas");
        assert_eq!(grupos[0].modelos.len(), 1);
        assert_eq!(grupos[0].modelos[0].id, 1);
    }

    #[test]
    fn agrupar_por_categoria_varias_misma_categoria() {
        let grupos = ModeloService::agrupar_por_categoria(vec![
            card(1, "Violín", Some("Cuerdas")),
            card(2, "Viola", Some("Cuerdas")),
        ]);

        assert_eq!(grupos.len(), 1);
        assert_eq!(grupos[0].categoria, "Cuerdas");
        assert_eq!(grupos[0].modelos.len(), 2);
        assert_eq!(grupos[0].modelos[0].id, 1);
        assert_eq!(grupos[0].modelos[1].id, 2);
    }

    #[test]
    fn agrupar_por_categoria_ignora_mayusculas_minusculas() {
        let grupos = ModeloService::agrupar_por_categoria(vec![
            card(1, "Violín", Some("Cuerdas")),
            card(2, "Viola", Some("cuerdas")),
            card(3, "Cello", Some("CUERDAS")),
        ]);

        assert_eq!(grupos.len(), 1);
        assert_eq!(grupos[0].categoria, "Cuerdas");
        assert_eq!(grupos[0].modelos.len(), 3);
        assert_eq!(grupos[0].modelos[0].id, 1);
        assert_eq!(grupos[0].modelos[1].id, 2);
        assert_eq!(grupos[0].modelos[2].id, 3);
    }

    #[test]
    fn agrupar_por_categoria_ignora_tildes() {
        let grupos = ModeloService::agrupar_por_categoria(vec![
            card(1, "Cámara", Some("Fotografía")),
            card(2, "Lente", Some("fotografia")),
            card(3, "Flash", Some("FOTOGRAFIA")),
        ]);

        assert_eq!(grupos.len(), 1);
        assert_eq!(grupos[0].categoria, "Fotografía");
        assert_eq!(grupos[0].modelos.len(), 3);
        assert_eq!(grupos[0].modelos[0].id, 1);
        assert_eq!(grupos[0].modelos[1].id, 2);
        assert_eq!(grupos[0].modelos[2].id, 3);
    }

    #[test]
    fn agrupar_por_categoria_varias_categorias_distintas() {
        let grupos = ModeloService::agrupar_por_categoria(vec![
            card(1, "Violín", Some("Cuerdas")),
            card(2, "Flauta", Some("Viento")),
            card(3, "Viola", Some("Cuerdas")),
        ]);

        assert_eq!(grupos.len(), 2);
        assert_eq!(grupos[0].categoria, "Cuerdas");
        assert_eq!(grupos[0].modelos.len(), 2);
        assert_eq!(grupos[1].categoria, "Viento");
        assert_eq!(grupos[1].modelos.len(), 1);
    }

    #[test]
    fn agrupar_por_categoria_sin_categoria_usa_sin_categoria() {
        let grupos = ModeloService::agrupar_por_categoria(vec![card(1, "Genérico", None)]);

        assert_eq!(grupos.len(), 1);
        assert_eq!(grupos[0].categoria, "Sin categoria");
    }

    #[test]
    fn agrupar_por_categoria_categoria_solo_espacios_usa_sin_categoria() {
        let grupos = ModeloService::agrupar_por_categoria(vec![card(1, "Genérico", Some("   "))]);

        assert_eq!(grupos.len(), 1);
        assert_eq!(grupos[0].categoria, "Sin categoria");
    }

    #[test]
    fn agrupar_por_categoria_conserva_orden_de_aparicion() {
        let grupos = ModeloService::agrupar_por_categoria(vec![
            card(1, "A", Some("Viento")),
            card(2, "B", Some("Cuerdas")),
            card(3, "C", Some("Viento")),
        ]);

        assert_eq!(grupos[0].categoria, "Viento");
        assert_eq!(grupos[0].modelos[0].id, 1);
        assert_eq!(grupos[0].modelos[1].id, 3);
        assert_eq!(grupos[1].categoria, "Cuerdas");
        assert_eq!(grupos[1].modelos[0].id, 2);
    }

    #[test]
    fn card_de_modelo_con_imagen_principal() {
        let conn = crear_db_test();
        ImageRepository::guardar_modelo(&conn, 1, 0, b"fake", "image/jpeg").unwrap();

        let card = ModeloService::card_de_modelo(&conn, modelo_test(1)).unwrap();

        assert_eq!(card.imagen, Some("/imagenes/modelos/1/0".into()));
        assert_eq!(card.nombre_modelo, "Modelo X");
    }

    #[test]
    fn card_de_modelo_sin_imagen() {
        let conn = crear_db_test();
        let card = ModeloService::card_de_modelo(&conn, modelo_test(1)).unwrap();
        assert!(card.imagen.is_none());
    }

    #[test]
    fn listar_cards_sin_modelos_retorna_vacio() {
        let conn = crear_db_test();
        let cards = ModeloService::listar_cards(&conn).unwrap();
        assert!(cards.is_empty());
    }

    #[test]
    fn listar_cards_retorna_cards_ordenadas_por_nombre() {
        let conn = crear_db_test();
        insertar_modelo(&conn, "Viola", Some("Cuerdas"));
        insertar_modelo(&conn, "Violín", Some("Cuerdas"));

        let cards = ModeloService::listar_cards(&conn).unwrap();

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].nombre_modelo, "Viola");
        assert_eq!(cards[0].categoria, Some("Cuerdas".into()));
        assert_eq!(cards[1].nombre_modelo, "Violín");
        assert!(cards[0].imagen.is_none());
        assert!(cards[1].imagen.is_none());
    }

    #[test]
    fn listar_cards_incluye_url_de_imagen_principal() {
        let conn = crear_db_test();
        let id = insertar_modelo(&conn, "Flauta", Some("Viento"));
        ImageRepository::guardar_modelo(&conn, id, 0, b"fake", "image/jpeg").unwrap();

        let cards = ModeloService::listar_cards(&conn).unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, id);
        assert_eq!(cards[0].imagen, Some(format!("/imagenes/modelos/{}/0", id)));
    }

    #[test]
    fn listar_cards_mezcla_modelos_con_y_sin_imagen() {
        let conn = crear_db_test();
        let id_con_imagen = insertar_modelo(&conn, "Clarinete", Some("Viento"));
        insertar_modelo(&conn, "Oboe", Some("Viento"));
        ImageRepository::guardar_modelo(&conn, id_con_imagen, 0, b"fake", "image/jpeg").unwrap();

        let cards = ModeloService::listar_cards(&conn).unwrap();

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].nombre_modelo, "Clarinete");
        assert_eq!(
            cards[0].imagen,
            Some(format!("/imagenes/modelos/{}/0", id_con_imagen))
        );
        assert_eq!(cards[1].nombre_modelo, "Oboe");
        assert!(cards[1].imagen.is_none());
    }

    #[test]
    fn listar_cards_preserva_categoria_opcional() {
        let conn = crear_db_test();
        insertar_modelo(&conn, "Genérico", None);

        let cards = ModeloService::listar_cards(&conn).unwrap();

        assert_eq!(cards.len(), 1);
        assert!(cards[0].categoria.is_none());
    }

    #[test]
    fn eliminar_modelo_sin_ejemplares_lo_marca_eliminado() {
        let conn = crear_db_test();
        let id = insertar_modelo(&conn, "Violín", Some("Cuerdas"));

        ModeloService::eliminar_modelo(&conn, id).unwrap();

        let modelo = ModeloRepository::buscar_por_id(&conn, id).unwrap().unwrap();
        assert!(modelo.eliminado);
    }

    #[test]
    fn eliminar_modelo_inexistente_falla() {
        let conn = crear_db_test();

        assert!(ModeloService::eliminar_modelo(&conn, 999).is_err());
    }

    #[test]
    fn eliminar_modelo_ya_eliminado_falla() {
        let conn = crear_db_test();
        let id = insertar_modelo(&conn, "Violín", Some("Cuerdas"));
        ModeloService::eliminar_modelo(&conn, id).unwrap();

        assert!(ModeloService::eliminar_modelo(&conn, id).is_err());
    }

    #[test]
    fn eliminar_modelo_con_ejemplar_activo_falla() {
        let conn = crear_db_test();
        let id = insertar_modelo(&conn, "Violín", Some("Cuerdas"));
        insertar_ejemplar(&conn, id, false);

        assert!(ModeloService::eliminar_modelo(&conn, id).is_err());
    }

    #[test]
    fn eliminar_modelo_permite_si_ejemplares_ya_eliminados() {
        let conn = crear_db_test();
        let id = insertar_modelo(&conn, "Violín", Some("Cuerdas"));
        insertar_ejemplar(&conn, id, true);

        ModeloService::eliminar_modelo(&conn, id).unwrap();

        let modelo = ModeloRepository::buscar_por_id(&conn, id).unwrap().unwrap();
        assert!(modelo.eliminado);
    }

    #[test]
    fn listar_cards_no_incluye_modelos_eliminados() {
        let conn = crear_db_test();
        insertar_modelo(&conn, "Viola", Some("Cuerdas"));
        let id_eliminado = insertar_modelo(&conn, "Violín", Some("Cuerdas"));
        ModeloService::eliminar_modelo(&conn, id_eliminado).unwrap();

        let cards = ModeloService::listar_cards(&conn).unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].nombre_modelo, "Viola");
    }
}
