use std::cell::RefCell;

use crate::font::FontSpec;
use crate::layout::text::TextMeasurer;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Style, Weight};

/// DPI used internally when converting font sizes between
/// typographic points, pixels, and millimeters.
const MEASUREMENT_DPI: f32 = 96.0;

/// Converts typographic points to pixels using the measurement DPI.
fn pt_to_px(points: f32) -> f32 {
    points * MEASUREMENT_DPI / 72.0
}

/// Converts pixels to millimeters using the measurement DPI.
fn px_to_mm(px: f32) -> f32 {
    px * 25.4 / MEASUREMENT_DPI
}

/// Measures text using real font metrics provided by `cosmic-text`.
///
/// Unlike the approximate text measurer, this implementation takes
/// the actual font family, size, weight, and style into account.
pub struct RealFontMeasurer {
    // !!! Important
    // FontSystem is mutable while shaping text. RefCell provides
    // interior mutability because TextMeasurer::measure_width()
    // receives &self rather than &mut self.
    font_system: RefCell<FontSystem>,
}

impl RealFontMeasurer {
    /// Creates a new text measurer with its own font system.
    pub fn new() -> Self {
        Self {
            font_system: RefCell::new(FontSystem::new()),
        }
    }
}

impl Default for RealFontMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextMeasurer for RealFontMeasurer {
    /// Measures the rendered width of `text` in millimeters.
    fn measure_width(&self, text: &str, font: &FontSpec) -> f32 {
        use cosmic_text::Family;

        let mut font_system = self.font_system.borrow_mut();

        // cosmic-text works with pixel-based font metrics,
        // while report-rs stores font sizes in points.
        let font_size_px = pt_to_px(font.size);

        // Use the same 1.2 line-height factor as the text layout engine.
        let line_height_px = font_size_px * 1.2;

        let metrics = Metrics::new(font_size_px, line_height_px);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        // No width or height constraint is applied because this
        // buffer is used only to measure the natural text width.
        buffer.set_size(&mut font_system, None, None);

        // Select the requested font weight.
        let weight = if font.bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        };

        // Select the requested font style.
        let style = if font.italic {
            Style::Italic
        } else {
            Style::Normal
        };

        // Describe the font that cosmic-text should use for shaping.
        let attrs = Attrs::new()
            .family(Family::Name(font.family.as_str()))
            .weight(weight)
            .style(style);

        // Advanced shaping handles real glyph metrics and positioning.
        buffer.set_text(&mut font_system, text, &attrs, Shaping::Advanced);

        // Perform text shaping before reading the resulting layout.
        buffer.shape_until_scroll(&mut font_system, false);

        // A buffer can contain multiple layout runs.
        // The widest run determines the measured text width.
        let width_px = buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0_f32, f32::max);

        // report-rs performs layout in millimeters.
        px_to_mm(width_px)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that real font metrics distinguish narrow
    /// characters from wide characters.
    #[test]
    fn real_font_width_differs() {
        let measurer = RealFontMeasurer::new();

        let font = FontSpec::default();

        let narrow = measurer.measure_width("iiiiiiiiii", &font);

        let wide = measurer.measure_width("WWWWWWWWWW", &font);

        println!("iiiiiiiiii = {narrow} mm");
        println!("WWWWWWWWWW = {wide} mm");

        assert!(wide > narrow);
    }

    /// Verifies that changing the font weight affects
    /// the measured text width.
    #[test]
    fn bold_font_width_differs() {
        let measurer = RealFontMeasurer::new();

        let normal = FontSpec {
            family: "DejaVu Sans".to_string(),
            size: 12.0,
            bold: false,
            italic: false,
        };

        let bold = FontSpec {
            bold: true,
            ..normal.clone()
        };

        let text = "Acesta este un text pentru test";

        let normal_width = measurer.measure_width(text, &normal);

        let bold_width = measurer.measure_width(text, &bold);

        println!("normal = {normal_width} mm");
        println!("bold   = {bold_width} mm");

        assert_ne!(normal_width, bold_width);
    }
}
