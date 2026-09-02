use super::*;

/// A text element positioned relative to its containing band.
///
/// Geometry and padding use millimeters, while `font_size` uses typographic
/// points. Text may contain `${name}` placeholders resolved during layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextItem {
    #[serde(default)]
    pub name: String,
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,

    pub text: String,

    /// Controls how `text` (or a bound field) is interpreted and formatted.
    #[serde(default, skip_serializing_if = "ValueType::is_text")]
    pub value_type: ValueType,

    /// Selects the query used when `field` binds this item to report data.
    #[serde(default, skip_serializing_if = "QuerySource::is_main")]
    pub query_source: QuerySource,

    /// Optional column name from `query_source`. Without it, `text` remains the
    /// item's literal value or expression template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,

    /// Optional display formatting applied to the resolved value.
    #[serde(default, skip_serializing_if = "ValueFormat::is_default")]
    pub value_format: ValueFormat,

    pub font_size: f32,

    #[serde(default = "default_font_family")]
    pub font_family: String,

    #[serde(default)]
    pub bold: bool,

    #[serde(default)]
    pub italic: bool,

    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,

    #[serde(default, skip_serializing_if = "is_false")]
    pub strikeout: bool,

    #[serde(default = "default_text_color")]
    pub text_color: Color,

    pub horizontal_align: HorizontalAlign,
    pub vertical_align: VerticalAlign,

    pub word_wrap: bool,
    pub auto_height: bool,

    #[serde(default)]
    pub padding: Padding,

    #[serde(default)]
    pub background: Option<Color>,

    #[serde(default)]
    pub border: Option<Border>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// How a text item's value is interpreted by expressions and formatters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ValueType {
    #[default]
    Text,
    Integer,
    Double,
    Boolean,
    Date,
    DateTime,
    Expression,
    Function,
}

impl ValueType {
    fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }
}

/// Query supplying the field bound to an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QuerySource {
    #[default]
    Main,
    Named(String),
}

impl QuerySource {
    fn is_main(&self) -> bool {
        matches!(self, Self::Main)
    }
}

impl TextItem {
    /// Collects the text item's font properties for the measurement layer.
    pub fn font_spec(&self) -> FontSpec {
        FontSpec {
            family: self.font_family.clone(),
            size: self.font_size,
            bold: self.bold,
            italic: self.italic,
        }
    }
}

/// A line segment positioned relative to its containing band.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    #[serde(default)]
    pub name: String,
    pub x1: Mm,
    pub y1: Mm,
    pub x2: Mm,
    pub y2: Mm,
    pub width: Mm,
}

/// An outlined rectangle positioned relative to its containing band.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectangleItem {
    #[serde(default)]
    pub name: String,
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,

    pub border_width: Mm,
}

/// An image positioned and sized relative to its containing band.
///
/// `source` identifies the image resource; loading and decoding it are renderer
/// responsibilities rather than concerns of the report model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageItem {
    #[serde(default)]
    pub name: String,
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,

    pub source: String,

    #[serde(default)]
    pub fit: ImageFit,
}

/// Controls how an image is placed inside its declared bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ImageFit {
    /// Fills the complete bounds and may change the image aspect ratio.
    #[default]
    Stretch,

    /// Preserves the image aspect ratio and centers it inside the bounds.
    Contain,
}
