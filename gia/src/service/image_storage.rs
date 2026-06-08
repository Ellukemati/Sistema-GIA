use crate::constants::{
    AVATARES_MAX_DIMENSION, AVATARES_PUBLIC_PREFIX, AVATARES_UPLOAD_DIR, EJEMPLARES_PUBLIC_PREFIX,
    EJEMPLARES_UPLOAD_DIR, INSTRUMENTOS_MAX_DIMENSION, MODELOS_PUBLIC_PREFIX, MODELOS_UPLOAD_DIR,
    STATIC_DIR, UPLOADS_DIR,
};
use crate::errors::ImageStorageError;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageEncoder};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum ImagenDestino {
    Modelo,
    Avatar,
    Ejemplar,
}

impl ImagenDestino {
    fn directorio(self) -> &'static str {
        match self {
            ImagenDestino::Modelo => MODELOS_UPLOAD_DIR,
            ImagenDestino::Avatar => AVATARES_UPLOAD_DIR,
            ImagenDestino::Ejemplar => EJEMPLARES_UPLOAD_DIR,
        }
    }

    fn prefijo_publico(self) -> &'static str {
        match self {
            ImagenDestino::Modelo => MODELOS_PUBLIC_PREFIX,
            ImagenDestino::Avatar => AVATARES_PUBLIC_PREFIX,
            ImagenDestino::Ejemplar => EJEMPLARES_PUBLIC_PREFIX,
        }
    }

    fn max_dimension(self) -> u32 {
        match self {
            ImagenDestino::Modelo => INSTRUMENTOS_MAX_DIMENSION,
            ImagenDestino::Ejemplar => INSTRUMENTOS_MAX_DIMENSION,
            ImagenDestino::Avatar => AVATARES_MAX_DIMENSION,
        }
    }
}

/// Se asegura que existan las carpetas necesarias para almacenar las imagenes. Se llama al iniciar el servidor
pub fn ensure_storage_directories() -> Result<(), std::io::Error> {
    fs::create_dir_all(STATIC_DIR)?;
    fs::create_dir_all(UPLOADS_DIR)?;
    fs::create_dir_all(MODELOS_UPLOAD_DIR)?;
    fs::create_dir_all(EJEMPLARES_UPLOAD_DIR)?;
    fs::create_dir_all(AVATARES_UPLOAD_DIR)?;
    Ok(())
}

/// Elimina un archivo previamente guardado usando su direccion publica.
pub fn eliminar_imagen_por_direccion(dir_publica: &str) -> Result<(), ImageStorageError> {
    let ruta = ruta_desde_direccion_publica(dir_publica)?;

    match fs::remove_file(&ruta) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(ImageStorageError::Io(err)),
    }
}

pub fn guardar_avatar_con_legajo(legajo: i64, bytes: &[u8]) -> Result<String, ImageStorageError> {
    guardar_imagen_con_metadata(bytes, ImagenDestino::Avatar, legajo)
}

pub fn guardar_imagen_modelo(modelo_id: i64, bytes: &[u8]) -> Result<String, ImageStorageError> {
    guardar_imagen_con_metadata(bytes, ImagenDestino::Modelo, modelo_id)
}

pub fn guardar_imagen_ejemplar(
    ejemplar_id: i64,
    bytes: &[u8],
) -> Result<String, ImageStorageError> {
    guardar_imagen_con_metadata(bytes, ImagenDestino::Ejemplar, ejemplar_id)
}

/// Guarda una imagen perteneciente a un avatar o a un modelo. Devuelve la URL publica relativa.
fn guardar_imagen_con_metadata(
    bytes: &[u8],
    destino: ImagenDestino,
    entidad_id: i64,
) -> Result<String, ImageStorageError> {
    let base = Path::new(destino.directorio());

    if bytes.is_empty() {
        return Err(ImageStorageError::InvalidImage(
            "La imagen esta vacia".to_string(),
        ));
    }

    // Se usa un unico directorio por tipo (modelos/, ejemplares/ y avatares/) y se codifica la entidad en el nombre del archivo
    let target_dir = base.to_path_buf();
    fs::create_dir_all(&target_dir)?;

    let imagen = image::load_from_memory(bytes)?;
    let imagen = redimensionar_si_es_necesario(imagen, destino.max_dimension());
    let tiene_alpha = imagen.color().has_alpha();
    let nombre = generar_nombre_archivo(tiene_alpha, entidad_id);
    let ruta = target_dir.join(&nombre);

    guardar_imagen_segun_alpha(&imagen, tiene_alpha, &ruta)?;

    let public_url = format!("{}/{}", destino.prefijo_publico(), nombre);

    Ok(public_url)
}

/// Convierte una direccion publica de imagen a la ruta fisica en el disco
fn ruta_desde_direccion_publica(dir_publica: &str) -> Result<PathBuf, ImageStorageError> {
    let dir_relativa = if let Some(resto) = dir_publica.strip_prefix(MODELOS_PUBLIC_PREFIX) {
        Path::new(MODELOS_UPLOAD_DIR).join(resto.trim_start_matches('/'))
    } else if let Some(resto) = dir_publica.strip_prefix(AVATARES_PUBLIC_PREFIX) {
        Path::new(AVATARES_UPLOAD_DIR).join(resto.trim_start_matches('/'))
    } else if let Some(resto) = dir_publica.strip_prefix(EJEMPLARES_PUBLIC_PREFIX) {
        Path::new(EJEMPLARES_UPLOAD_DIR).join(resto.trim_start_matches('/'))
    } else {
        return Err(ImageStorageError::InvalidImage(
            "La direccion no pertenece a una imagen administrada por el servidor".to_string(),
        ));
    };

    Ok(dir_relativa)
}

/// Redimensiona la imagen si alguna de sus dimensiones excede el maximo permitido para su tipo
fn redimensionar_si_es_necesario(imagen: DynamicImage, max_dimension: u32) -> DynamicImage {
    let (ancho, alto) = imagen.dimensions();

    if ancho <= max_dimension && alto <= max_dimension {
        imagen
    } else {
        imagen.resize(max_dimension, max_dimension, FilterType::Lanczos3)
    }
}

/// Genera un nombre de archivo unico usando metadata de la entidad y un UUID.
/// Formato: {id}_{uuid}.{ext}
fn generar_nombre_archivo(tiene_alpha: bool, entidad_id: i64) -> String {
    let extension = if tiene_alpha { "png" } else { "jpg" };
    let uuid_part = format!("{}.{}", Uuid::new_v4(), extension);
    format!("{}_{}", entidad_id, uuid_part)
}

/// Guarda la imagen en PNG si tiene canal alpha, o como JPEG optimizado si no lo tiene
fn guardar_imagen_segun_alpha(
    imagen: &DynamicImage,
    tiene_alpha: bool,
    ruta: &Path,
) -> Result<(), ImageStorageError> {
    if tiene_alpha {
        imagen.save_with_format(ruta, image::ImageFormat::Png)?;
    } else {
        guardar_como_jpeg(imagen, ruta)?;
    }

    Ok(())
}

/// Guarda la imagen como JPEG comprimido
fn guardar_como_jpeg(imagen: &DynamicImage, ruta: &Path) -> Result<(), ImageStorageError> {
    let file = File::create(ruta)?;
    let mut writer = BufWriter::new(file);
    let rgb = imagen.to_rgb8();
    let encoder = JpegEncoder::new_with_quality(&mut writer, 82);
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
    fn test_ruta_desde_direccion_publica_valida() {
        // Testea que el parseo de strings funcione como lo esperado
        let dir_publica = "/static/uploads/modelos/1_un-uuid-falso.jpg";
        let resultado = ruta_desde_direccion_publica(dir_publica).unwrap();

        assert!(resultado.starts_with(MODELOS_UPLOAD_DIR));
        assert!(resultado.ends_with("1_un-uuid-falso.jpg"));
    }

    #[test]
    fn test_ruta_desde_direccion_publica_invalida() {
        let dir_invalida = "/static/uploads/hack/foto.jpg";
        let resultado = ruta_desde_direccion_publica(dir_invalida);

        assert!(resultado.is_err());
    }

    #[test]
    fn test_guardar_imagen_vacia_retorna_error() {
        let bytes_vacios: [u8; 0] = [];
        let resultado = guardar_imagen_modelo(42, &bytes_vacios);

        assert!(resultado.is_err());
        if let Err(ImageStorageError::InvalidImage(msg)) = resultado {
            assert_eq!(msg, "La imagen esta vacia");
        } else {
            panic!("Deberia haber devuelto un InvalidImage");
        }
    }

    #[test]
    fn test_generar_nombre_archivo() {
        let entidad_id = 123;

        // Probar el camino de JPEG
        let nombre_jpeg = generar_nombre_archivo(false, entidad_id);
        assert!(nombre_jpeg.starts_with("123_"));
        assert!(nombre_jpeg.ends_with(".jpg"));

        // Probar el camino de PNG
        let nombre_png = generar_nombre_archivo(true, entidad_id);
        assert!(nombre_png.starts_with("123_"));
        assert!(nombre_png.ends_with(".png"));
    }
}
