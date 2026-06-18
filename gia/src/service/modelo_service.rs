use crate::models::modelo::Modelo;
use crate::repository::image_repository::ImageRepository;
use crate::repository::modelo_repository::ModeloRepository;
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

    /// Crea un vector de modelosCardDTO que tiene la info esencial para mostrar un modelo.
    pub fn listar_cards(conn: &Connection) -> Result<Vec<ModeloCardDTO>, String> {
        let modelos = ModeloRepository::listar_todos(conn)
            .map_err(|e| format!("Error al listar los modelos: {}", e))?;

        let mut cards = Vec::with_capacity(modelos.len());
        for modelo in modelos {
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

            cards.push(ModeloCardDTO {
                id: modelo.id,
                nombre_modelo: modelo.nombre_modelo,
                categoria: modelo.categoria,
                imagen,
            });
        }

        Ok(cards)
    }
}
