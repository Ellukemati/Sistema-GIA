use crate::models::imagen_modelo::ImagenModelo;
use crate::models::modelo::Modelo;
use crate::repository::modelo_repository::ModeloRepository;
use rusqlite::Connection;

pub struct ModeloService;

pub struct CrearModeloData {
    pub marca: String,
    pub nombre_modelo: String,
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
}

pub struct ModeloCardDTO {
    pub id: i64,
    pub nombre_modelo: String,
    pub categoria: Option<String>,
    pub imagen: Option<ImagenModelo>
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
}
