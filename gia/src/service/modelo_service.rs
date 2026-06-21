use crate::models::modelo::Modelo;
use crate::repository::ejemplar_repository::EjemplarRepository;
use crate::repository::image_repository::ImageRepository;
use crate::repository::modelo_repository::ModeloRepository;
use crate::repository::reserva_repository::ReservaRepository;
use rusqlite::Connection;
use serde::Serialize;
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
    pub imagen: Option<String>
}

#[derive(Serialize)]
pub struct GrupoCategoriaDTO {
    pub categoria: String,
    pub modelos: Vec<ModeloCardDTO>,
}

impl ModeloService {
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
        };

        match ModeloRepository::crear(conn, &modelo_temporal) {
            Ok(id_real) => Ok(Modelo {
                id: id_real,
                ..modelo_temporal
            }),
            Err(e) => Err(format!("Error en la base de datos al crear modelo: {}", e)),
        }
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

    /// Agrupa las tarjetas por categoria, conservando el orden de aparicion.
    /// Los modelos sin categoria quedan como "Sin categoria".
    fn agrupar_por_categoria(cards: Vec<ModeloCardDTO>) -> Vec<GrupoCategoriaDTO> {
        let mut grupos: Vec<GrupoCategoriaDTO> = Vec::new();
        for card in cards {
            let categoria = card
                .categoria
                .clone()
                .filter(|c| !c.trim().is_empty())
                .unwrap_or_else(|| "Sin categoria".to_string());

            match grupos.iter_mut().find(|g| g.categoria == categoria) {
                Some(grupo) => grupo.modelos.push(card),
                None => grupos.push(GrupoCategoriaDTO {
                    categoria,
                    modelos: vec![card],
                }),
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
        inicio: &str,
        fin: &str,
    ) -> Result<Vec<GrupoCategoriaDTO>, String> {
        let modelos = ModeloRepository::listar_todos(conn)
            .map_err(|e| format!("Error al listar los modelos: {}", e))?;

        let mut cards = Vec::new();
        for modelo in modelos {
            let ejemplares = EjemplarRepository::listar_por_modelo(conn, modelo.id)
                .map_err(|e| {
                    format!(
                        "Error al listar ejemplares del modelo {}: {}",
                        modelo.id, e
                    )
                })?;

            let mut hay_disponible = false;
            for ejemplar in &ejemplares {
                if ReservaRepository::ejemplar_disponible(conn, ejemplar.id, inicio, fin)
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
}
