use crate::constants::{AVATARES_MAX_DIMENSION, INSTRUMENTOS_MAX_DIMENSION};
use crate::errors::ImageStorageError;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageEncoder};
use std::io::Cursor;

#[derive(Debug, Clone, Copy)]
pub enum ImagenDestino {
    Modelo,
    Avatar,
    Ejemplar,
}

impl ImagenDestino {
    fn max_dimension(self) -> u32 {
        match self {
            ImagenDestino::Modelo => INSTRUMENTOS_MAX_DIMENSION,
            ImagenDestino::Ejemplar => INSTRUMENTOS_MAX_DIMENSION,
            ImagenDestino::Avatar => AVATARES_MAX_DIMENSION,
        }
    }
}

/// Procesa los bytes de una imagen (redimensión y compresión) puramente en memoria
/// Retorna una tupla con los bytes finales y el tipo MIME ("image/png" o "image/jpeg")
pub fn procesar_imagen_en_memoria(
    bytes: &[u8],
    destino: ImagenDestino,
) -> Result<(Vec<u8>, String), ImageStorageError> {
    if bytes.is_empty() {
        return Err(ImageStorageError::InvalidImage(
            "La imagen esta vacia".to_string(),
        ));
    }

    let imagen = image::load_from_memory(bytes)?;
    let imagen = redimensionar_si_es_necesario(imagen, destino.max_dimension());
    let tiene_alpha = imagen.color().has_alpha();

    let mut bytes_finales = Vec::new();
    let mime = if tiene_alpha {
        imagen.write_to(
            &mut Cursor::new(&mut bytes_finales),
            image::ImageFormat::Png,
        )?;
        "image/png".to_string()
    } else {
        guardar_como_jpeg_en_memoria(&imagen, &mut bytes_finales)?;
        "image/jpeg".to_string()
    };

    Ok((bytes_finales, mime))
}

pub fn procesar_avatar(bytes: &[u8]) -> Result<(Vec<u8>, String), ImageStorageError> {
    procesar_imagen_en_memoria(bytes, ImagenDestino::Avatar)
}

pub fn procesar_modelo(bytes: &[u8]) -> Result<(Vec<u8>, String), ImageStorageError> {
    procesar_imagen_en_memoria(bytes, ImagenDestino::Modelo)
}

pub fn procesar_ejemplar(bytes: &[u8]) -> Result<(Vec<u8>, String), ImageStorageError> {
    procesar_imagen_en_memoria(bytes, ImagenDestino::Ejemplar)
}

fn redimensionar_si_es_necesario(imagen: DynamicImage, max_dimension: u32) -> DynamicImage {
    let (ancho, alto) = imagen.dimensions();
    if ancho <= max_dimension && alto <= max_dimension {
        imagen
    } else {
        imagen.resize(max_dimension, max_dimension, FilterType::Lanczos3)
    }
}

fn guardar_como_jpeg_en_memoria(
    imagen: &DynamicImage,
    buffer: &mut Vec<u8>,
) -> Result<(), ImageStorageError> {
    let rgb = imagen.to_rgb8();
    let encoder = JpegEncoder::new_with_quality(buffer, 82);
    encoder.write_image(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guardar_imagen_vacia_retorna_error() {
        let bytes_vacios: [u8; 0] = [];
        let resultado = procesar_modelo(&bytes_vacios);
        assert!(resultado.is_err());
    }
}
