use crate::models::modelo_instrumento::ModeloInstrumento;
use crate::repository::modelo_instrumento_repository::ModeloInstrumentoRepository;
use rusqlite::Connection;

pub struct ModeloInstrumentoService;

impl ModeloInstrumentoService {
    pub fn crear_modelo_instrumento(
        conn: &Connection, // Agrego conexión
        marca: Option<String>,
        nombre_modelo: String,
        categoria: Option<String>,
        descripcion: Option<String>,
        manual_url: Option<String>,
        imagen_principal_url: Option<String>,
    ) -> Result<ModeloInstrumento, String> {
        //validar que el usuario sea admin

        let modelo = ModeloInstrumento {
            id: 0, // se asigna en la db
            marca,
            nombre_modelo,
            categoria,
            descripcion,
            manual_url,
            imagen_principal_url,
        };

        match ModeloInstrumentoRepository::crear(conn, &modelo) {
            Ok(_) => Ok(modelo), //en el futuro, deberia buscar el modelo y retornarlo
            Err(e) => Err(format!("Error en la base de datos al crear modelo: {}", e)),
        }
    }
}