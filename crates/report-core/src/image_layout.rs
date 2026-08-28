use crate::model::{ImageFit, Mm};

/// Final image rectangle inside the bounds declared by [`crate::model::ImageItem`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePlacement {
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,
}

/// Calculates renderer-independent image geometry.
pub fn calculate_image_placement(
    x: Mm,
    y: Mm,
    width: Mm,
    height: Mm,
    pixel_width: u32,
    pixel_height: u32,
    fit: ImageFit,
) -> ImagePlacement {
    let bounds = ImagePlacement {
        x,
        y,
        width,
        height,
    };

    if fit == ImageFit::Stretch
        || pixel_width == 0
        || pixel_height == 0
        || width.0 <= 0.0
        || height.0 <= 0.0
    {
        return bounds;
    }

    let scale = (width.0 / pixel_width as f32).min(height.0 / pixel_height as f32);
    let placed_width = pixel_width as f32 * scale;
    let placed_height = pixel_height as f32 * scale;

    ImagePlacement {
        x: Mm(x.0 + (width.0 - placed_width) / 2.0),
        y: Mm(y.0 + (height.0 - placed_height) / 2.0),
        width: Mm(placed_width),
        height: Mm(placed_height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretch_uses_complete_bounds() {
        let placement = calculate_image_placement(
            Mm(10.0),
            Mm(20.0),
            Mm(100.0),
            Mm(100.0),
            200,
            100,
            ImageFit::Stretch,
        );

        assert_eq!(placement.x, Mm(10.0));
        assert_eq!(placement.y, Mm(20.0));
        assert_eq!(placement.width, Mm(100.0));
        assert_eq!(placement.height, Mm(100.0));
    }

    #[test]
    fn contain_centers_landscape_image_vertically() {
        let placement = calculate_image_placement(
            Mm(10.0),
            Mm(20.0),
            Mm(100.0),
            Mm(100.0),
            200,
            100,
            ImageFit::Contain,
        );

        assert_eq!(placement.x, Mm(10.0));
        assert_eq!(placement.y, Mm(45.0));
        assert_eq!(placement.width, Mm(100.0));
        assert_eq!(placement.height, Mm(50.0));
    }

    #[test]
    fn contain_centers_portrait_image_horizontally() {
        let placement = calculate_image_placement(
            Mm(10.0),
            Mm(20.0),
            Mm(100.0),
            Mm(100.0),
            100,
            200,
            ImageFit::Contain,
        );

        assert_eq!(placement.x, Mm(35.0));
        assert_eq!(placement.y, Mm(20.0));
        assert_eq!(placement.width, Mm(50.0));
        assert_eq!(placement.height, Mm(100.0));
    }
}
