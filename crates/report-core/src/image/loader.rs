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

    #[error("failed to decode SVG image '{}': {message}", path.display())]
    Svg { path: PathBuf, message: String },
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

    load_image_bytes(&bytes, path)
}

/// Decodes PNG/JPEG bytes obtained from a database BLOB.
pub fn load_image_bytes(
    bytes: &[u8],
    source_name: impl AsRef<Path>,
) -> Result<LoadedImage, ImageLoadError> {
    let source_name = source_name.as_ref();
    let decoded = match image::load_from_memory(bytes) {
        Ok(decoded) => decoded,
        Err(_source)
            if bytes.starts_with(b"<svg") || bytes.windows(4).any(|part| part == b"<svg") =>
        {
            let options = resvg::usvg::Options::default();
            let tree = resvg::usvg::Tree::from_data(bytes, &options).map_err(|error| {
                ImageLoadError::Svg {
                    path: source_name.to_path_buf(),
                    message: error.to_string(),
                }
            })?;
            let size = tree.size().to_int_size();
            let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
                .ok_or_else(|| ImageLoadError::Svg {
                    path: source_name.to_path_buf(),
                    message: "invalid SVG dimensions".to_string(),
                })?;
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::default(),
                &mut pixmap.as_mut(),
            );
            let png = pixmap.encode_png().map_err(|error| ImageLoadError::Svg {
                path: source_name.to_path_buf(),
                message: error.to_string(),
            })?;
            image::load_from_memory(&png).map_err(|source| ImageLoadError::Decode {
                path: source_name.to_path_buf(),
                source,
            })?
        }
        Err(source) => {
            return Err(ImageLoadError::Decode {
                path: source_name.to_path_buf(),
                source,
            });
        }
    };
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
    fn load_png_blob_to_rgba() {
        let source = RgbaImage::from_pixel(2, 1, Rgba([10, 20, 30, 40]));
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(source)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();

        let loaded = load_image_bytes(&bytes.into_inner(), "database-image").unwrap();

        assert_eq!(loaded.width, 2);
        assert_eq!(loaded.height, 1);
        assert_eq!(loaded.rgba.len(), 8);
    }

    #[test]
    fn load_svg_blob_to_rgba() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="3"><rect width="2" height="3" fill="red"/></svg>"#;

        let loaded = load_image_bytes(svg, "database-svg").unwrap();

        assert_eq!(loaded.width, 2);
        assert_eq!(loaded.height, 3);
        assert_eq!(loaded.rgba.len(), 2 * 3 * 4);
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
