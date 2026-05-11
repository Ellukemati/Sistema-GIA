use std::str::FromStr;

/// Representa un modelo de instrumento en el sistema GIA.
/// 
/// - **id**: Entero (i32)
/// - **nombre**: Texto
/// - **descripcion**: Texto (No puede omitirse el campo, aunque puede estar vacío)
/// - **stock**: Entero (i32)
/// - **categoria**: Texto
/// - **disponible**: "true" para disponible, "false" para no disponible
pub struct Instrumento {
    pub id: i32,
    pub nombre: String,
    pub descripcion: String,
    pub stock: i32,
    pub categoria: String,
    //pub manual: String,
    pub disponible: bool, // La idea seria que un instrumento no cambie de estado. Ya que la idea es agrupar instrumentos por tipo y disponibilidad para que tengan el mismo comportamiento.
}

/// ### Formato de entrada esperado para FromStr:
/// Se espera una cadena de texto con 6 campos separados por comas (CSV):
/// `id,nombre,descripcion,stock,categoria,disponible`
impl FromStr for Instrumento {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Separar por comas
        let parts: Vec<&str> = s.split(',').collect();

        if parts.len() != 6 {
            return Err(format!(
                "Error de formato: Se esperaban 6 campos, se recibieron {}. Formato: id,nombre,descripcion,stock,categoria,disponible", 
                parts.len()
            ));
        }

        // Parsear campo por campo con manejo de errores manual
        let id = parts[0].trim().parse::<i32>()
            .map_err(|_| "ID inválido: debe ser un número entero".to_string())?;

        let stock = parts[3].trim().parse::<i32>()
            .map_err(|_| "Stock inválido: debe ser un número entero".to_string())?;

        let disponible = match parts[5].trim() {
            "true" => true,
            "false" => false,
            _ => return Err("Disponibilidad inválida: use 'true' o 'false'".to_string()),
        };

        Ok(Instrumento {
            id,
            nombre: parts[1].trim().to_string(),
            descripcion: parts[2].trim().to_string(), // Si está vacío quedara -> ""
            stock,
            categoria: parts[4].trim().to_string(),
            disponible,
        })
    }
}