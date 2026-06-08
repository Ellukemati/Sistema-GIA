use crate::models::modelo::Modelo;
use crate::repository::modelo_repository::ModeloRepository;
use rusqlite::Connection;

pub struct ModeloService;

impl ModeloService {
    pub fn crear_modelo(
        conn: &Connection, // Agrego conexión
        marca: Option<String>,
        nombre_modelo: String,
        categoria: Option<String>,
        descripcion: Option<String>,
        manual_url: Option<String>,
        direccion_imagen_principal: Option<String>,
    ) -> Result<Modelo, String> {
        //validar que el usuario sea admin

        let modelo = Modelo {
            id: 0, // se asigna en la db
            marca,
            nombre_modelo,
            categoria,
            descripcion,
            manual_url,
            direccion_imagen_principal,
        };

        match ModeloRepository::crear(conn, &modelo) {
            Ok(_) => Ok(modelo), //en el futuro, deberia buscar el modelo y retornarlo
            Err(e) => Err(format!("Error en la base de datos al crear modelo: {}", e)),
        }
    }
}
