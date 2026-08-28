use crate::model::DEFAULT_FONT_FAMILY;

/// Describes the font used to render a text item.
#[derive(Debug, Clone, PartialEq)]
pub struct FontSpec {
    /// Font family name, for example "DejaVu Sans".
    pub family: String,

    /// Font size in points.
    pub size: f32,

    /// Enables the bold font variant.
    pub bold: bool,

    /// Enables the italic font variant.
    pub italic: bool,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self {
            family: DEFAULT_FONT_FAMILY.to_string(),
            size: 12.0,
            bold: false,
            italic: false,
        }
    }
}
