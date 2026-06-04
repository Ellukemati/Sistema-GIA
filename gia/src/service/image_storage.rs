use std::fs;

/// Elimina un archivo de imagen dado su path. Ignora si no existe.
pub fn eliminar_imagen_por_direccion(direccion: &str) -> Result<(), std::io::Error> {
    match fs::remove_file(direccion) {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}
