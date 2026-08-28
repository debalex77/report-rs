use std::path::{Path, PathBuf};

use thiserror::Error;

/// An image decoded to renderer-independent RGBA8 pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Errors produced while reading or decoding an image source.
#[derive(Debug, Error)]
pub enum ImageLoadError {
    #[error("failed to read image '{}': {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to decode image '{}': {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
}

/// Reads a PNG or JPEG file and decodes it to RGBA8 pixels.
///
/// Path resolution is intentionally left to the caller so report-relative and
/// application-specific resource locations can be supported without coupling
/// the report model or layout engine to the filesystem.
pub fn load_image(path: impl AsRef<Path>) -> Result<LoadedImage, ImageLoadError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|source| ImageLoadError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    let decoded = image::load_from_memory(&bytes).map_err(|source| ImageLoadError::Decode {
        path: path.to_path_buf(),
        source,
    })?;
    let rgba = decoded.to_rgba8();

    Ok(LoadedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    #[test]
    fn load_png_to_rgba() {
        let source = RgbaImage::from_pixel(2, 1, Rgba([10, 20, 30, 40]));
        let mut bytes = Cursor::new(Vec::new());

        DynamicImage::ImageRgba8(source)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();

        let path = std::env::temp_dir().join(format!(
            "report-rs-image-loader-{}-{}.png",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(&path, bytes.into_inner()).unwrap();

        let loaded = load_image(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(loaded.width, 2);
        assert_eq!(loaded.height, 1);
        assert_eq!(loaded.rgba, vec![10, 20, 30, 40, 10, 20, 30, 40]);
    }

    #[test]
    fn load_jpeg_to_rgba() {
        let source = RgbaImage::from_pixel(3, 2, Rgba([10, 20, 30, 255]));
        let mut bytes = Cursor::new(Vec::new());

        DynamicImage::ImageRgba8(source)
            .write_to(&mut bytes, ImageFormat::Jpeg)
            .unwrap();

        let path = std::env::temp_dir().join(format!(
            "report-rs-image-loader-{}-{}.jpg",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(&path, bytes.into_inner()).unwrap();

        let loaded = load_image(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(loaded.width, 3);
        assert_eq!(loaded.height, 2);
        assert_eq!(loaded.rgba.len(), 3 * 2 * 4);
    }

    #[test]
    fn missing_file_returns_read_error() {
        let path = Path::new("this-image-does-not-exist.png");

        let error = load_image(path).unwrap_err();

        assert!(matches!(error, ImageLoadError::Read { .. }));
    }
}
