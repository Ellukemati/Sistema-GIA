use crate::constants::MANUALES_MAX_SIZE;
use crate::errors::ManualStorageError;

/// Valida que el archivo sea un PDF legítimo y no exceda los 16 MB, luego lo procesa para almacenamiento
pub fn validar_y_procesar_manual(bytes: &[u8]) -> Result<(Vec<u8>, String), ManualStorageError> {
    if bytes.len() > MANUALES_MAX_SIZE {
        return Err(ManualStorageError::InvalidManual(format!(
            "El archivo excede el limite de tamaño de {} MB.",
            MANUALES_MAX_SIZE / 1024 / 1024
        )));
    }

    // Validación de Magic Numbers (%PDF-)
    if bytes.len() < 4 || bytes[0..4] != [0x25, 0x50, 0x44, 0x46] {
        return Err(ManualStorageError::InvalidManual(
            "El archivo no es un documento PDF valido.".to_string(),
        ));
    }

    Ok((bytes.to_vec(), "application/pdf".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MANUALES_MAX_SIZE;

    #[test]
    fn test_validar_pdf_correcto_retorna_ok() {
        // Un PDF legitimo arranca con los bytes hex: 25 50 44 46 (%PDF)
        let mut bytes_validos = vec![0x25, 0x50, 0x44, 0x46];
        bytes_validos.extend_from_slice(b"hola soy un pdf valido");

        let resultado = validar_y_procesar_manual(&bytes_validos);

        assert!(resultado.is_ok());
        let (data, mime) = resultado.unwrap();
        assert_eq!(mime, "application/pdf");
        assert_eq!(data, bytes_validos);
    }

    #[test]
    fn test_validar_archivo_vacio_o_sin_magic_numbers_falla() {
        let bytes_invalidos = b"hola soy un pdf invalido".to_vec();
        let resultado = validar_y_procesar_manual(&bytes_invalidos);

        assert!(resultado.is_err());
        if let Err(ManualStorageError::InvalidManual(msg)) = resultado {
            assert!(msg.contains("no es un documento PDF valido"));
        } else {
            panic!("Se esperaba un error del tipo InvalidManual");
        }
    }

    #[test]
    fn test_validar_archivo_cuando_excede_tamanio_maximo_falla() {
        let bytes_gigantes = vec![0; MANUALES_MAX_SIZE + 1];
        let resultado = validar_y_procesar_manual(&bytes_gigantes);

        assert!(resultado.is_err());
        if let Err(ManualStorageError::InvalidManual(msg)) = resultado {
            assert!(msg.contains("excede el limite de tamaño"));
        } else {
            panic!("Se esperaba un error de limite de tamaño");
        }
    }
}
