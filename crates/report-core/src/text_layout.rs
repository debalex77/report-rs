use crate::font::FontSpec;

/// Number of millimeters in one typographic point.
pub const MM_PER_POINT: f32 = 25.4 / 72.0;

/// Multiplier used to derive the distance between consecutive text baselines.
pub const DEFAULT_LINE_HEIGHT_FACTOR: f32 = 1.2;

/// Converts typographic points to millimeters.
pub fn pt_to_mm(points: f32) -> f32 {
    points * MM_PER_POINT
}

/// Converts millimeters to typographic points.
pub fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

/// A rendered line of text and its measured width in millimeters.
#[derive(Debug, Clone)]
pub struct TextLine {
    pub text: String,
    pub width: f32,
}

/// Result of laying out a block of text.
///
/// All dimensions are expressed in millimeters. `height` is the combined
/// height of all lines and does not include the owning item's padding.
#[derive(Debug, Clone)]
pub struct TextLayout {
    pub lines: Vec<TextLine>,
    pub line_height: f32,
    pub height: f32,
}

impl TextLayout {
    /// Creates a layout containing exactly one line without word wrapping.
    pub fn single_line<M: TextMeasurer>(text: &str, font: &FontSpec, measurer: &M) -> Self {
        let line_height = pt_to_mm(font.size) * DEFAULT_LINE_HEIGHT_FACTOR;

        let width = measurer.measure_width(text, font);

        Self {
            lines: vec![TextLine {
                text: text.to_string(),
                width,
            }],

            line_height,
            height: line_height,
        }
    }

    /// Wraps text at whitespace boundaries so lines fit within `max_width`.
    ///
    /// The wrapping pass depends only on measurements supplied by `measurer`.
    /// It does not shape or draw glyphs; renderers consume the resulting lines
    /// later. A word wider than `max_width` remains intact on its own line.
    pub fn wrap<M: TextMeasurer>(
        text: &str,
        font: &FontSpec,
        max_width: f32,
        measurer: &M,
    ) -> Self {
        let mut lines: Vec<TextLine> = Vec::new();

        let mut current_line = String::new();

        // split_whitespace normalizes runs of whitespace while producing the
        // word boundaries used by this intentionally simple wrapping strategy.
        for word in text.split_whitespace() {
            let candidate = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let candidate_width = measurer.measure_width(&candidate, font);

            if candidate_width <= max_width {
                current_line = candidate;
            } else {
                // Commit the line before starting another one with the word
                // that caused the candidate to exceed the available width.
                if !current_line.is_empty() {
                    let width = measurer.measure_width(&current_line, font);

                    lines.push(TextLine {
                        text: current_line,
                        width,
                    });
                }

                // Long words are not split at character boundaries. They may
                // therefore be wider than max_width in the resulting layout.
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            let width = measurer.measure_width(&current_line, font);

            lines.push(TextLine {
                text: current_line,
                width,
            });
        }

        // Preserve one measurable line for empty or whitespace-only text.
        if lines.is_empty() {
            lines.push(TextLine {
                text: String::new(),
                width: 0.0,
            });
        }

        let line_height = pt_to_mm(font.size) * DEFAULT_LINE_HEIGHT_FACTOR;

        let height = line_height * lines.len() as f32;

        Self {
            lines,
            line_height,
            height,
        }
    }
}

/// Supplies font-dependent text widths to the layout algorithm.
///
/// Implementations return millimeters. Keeping measurement behind this trait
/// lets callers choose fast approximate metrics or real font shaping while the
/// wrapping algorithm remains renderer-independent.
pub trait TextMeasurer {
    /// Measures the natural width of `text` rendered with `font`.
    fn measure_width(&self, text: &str, font: &FontSpec) -> f32;
}

/// Fast deterministic text measurer based on character count.
///
/// It is suitable for tests and lightweight layout, but unlike a real font
/// measurer it treats every character as having the same width and ignores
/// family, weight, italic style, kerning, and glyph-specific metrics.
pub struct ApproxTextMeasurer;

impl TextMeasurer for ApproxTextMeasurer {
    fn measure_width(&self, text: &str, font: &FontSpec) -> f32 {
        let font_size_mm = pt_to_mm(font.size);

        // Approximate each Unicode scalar value as half the font size.
        text.chars().count() as f32 * font_size_mm * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, epsilon: f32) {
        assert!((a - b).abs() < epsilon, "left = {a}, right = {b}");
    }

    #[test]
    fn single_line_text() {
        let measurer = ApproxTextMeasurer;
        let font = FontSpec::default();

        let layout = TextLayout::single_line("Hello", &font, &measurer);

        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].text, "Hello");
        assert!(layout.lines[0].width > 0.0);

        let expected_line_height = pt_to_mm(font.size) * DEFAULT_LINE_HEIGHT_FACTOR;

        approx_eq(layout.line_height, expected_line_height, 0.001);

        approx_eq(layout.height, expected_line_height, 0.001);
    }

    #[test]
    fn wrap_text() {
        let measurer = ApproxTextMeasurer;
        let font = FontSpec::default();

        let layout = TextLayout::wrap("one two three four", &font, 15.0, &measurer);

        let texts: Vec<&str> = layout.lines.iter().map(|line| line.text.as_str()).collect();

        assert_eq!(texts, vec!["one two", "three", "four",]);

        for line in &layout.lines {
            assert!(line.width >= 0.0);
            assert!(line.width <= 15.0);
        }

        let expected_line_height = pt_to_mm(font.size) * DEFAULT_LINE_HEIGHT_FACTOR;

        let expected_height = expected_line_height * 3.0;

        approx_eq(layout.line_height, expected_line_height, 0.001);

        approx_eq(layout.height, expected_height, 0.001);
    }

    #[test]
    fn padding_reduces_available_width() {
        let measurer = ApproxTextMeasurer;
        let font = FontSpec::default();

        let full_width = 30.0;
        let padding_left = 5.0;
        let padding_right = 5.0;

        let content_width = full_width - padding_left - padding_right;

        let without_padding = TextLayout::wrap("one two three four", &font, full_width, &measurer);

        let with_padding = TextLayout::wrap("one two three four", &font, content_width, &measurer);

        assert!(with_padding.lines.len() >= without_padding.lines.len());
    }
}
