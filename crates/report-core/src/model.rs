use crate::font::FontSpec;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// Default font family used when a text item does not specify one.
pub const DEFAULT_FONT_FAMILY: &str = "DejaVu Sans";

/// Returns the owned default font family expected by Serde and model fields.
pub fn default_font_family() -> String {
    DEFAULT_FONT_FAMILY.to_string()
}

/// Returns the default color used for text.
pub fn default_text_color() -> Color {
    Color::BLACK
}

/// Serializable definition of a complete report.
///
/// A report contains one or more logical pages. Each logical page may produce
/// multiple physical pages when processed by the layout engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub name: String,
    pub pages: Vec<Page>,
}

impl Report {
    /// Loads and deserializes a report definition from a JSON file.
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let report: Report = serde_json::from_str(&json)?;

        Ok(report)
    }

    /// Serializes the report as pretty-printed JSON and writes it to a file.
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;

        Ok(())
    }
}

/// Physical paper size used by a report page.
///
/// Built-in and custom dimensions are stored in millimeters and describe the
/// portrait orientation; landscape orientation swaps the two axes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PageSize {
    A4,
    A5,
    Letter,
    Custom { width: Mm, height: Mm },
}

impl PageSize {
    /// Returns the base width and height before orientation is applied.
    pub fn dimensions(&self) -> (Mm, Mm) {
        match self {
            PageSize::A4 => (Mm(210.0), Mm(297.0)),
            PageSize::A5 => (Mm(148.0), Mm(210.0)),
            PageSize::Letter => (Mm(215.9), Mm(279.4)),
            PageSize::Custom { width, height } => (*width, *height),
        }
    }

    /// Returns dimensions with portrait or landscape orientation applied.
    pub fn oriented_dimensions(&self, orientation: Orientation) -> (Mm, Mm) {
        let (width, height) = self.dimensions();

        match orientation {
            Orientation::Portrait => (width, height),
            Orientation::Landscape => (height, width),
        }
    }
}

/// Orientation applied to the base dimensions of a [`PageSize`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Non-printable spacing around the page content, in millimeters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Margins {
    pub left: Mm,
    pub top: Mm,
    pub right: Mm,
    pub bottom: Mm,
}

/// Declarative definition of one logical report page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub size: PageSize,
    pub orientation: Orientation,
    pub margins: Margins,
    pub bands: Vec<Band>,
}

impl Page {
    /// Returns the oriented physical page dimensions.
    pub fn dimensions(&self) -> (Mm, Mm) {
        self.size.oriented_dimensions(self.orientation)
    }

    /// Returns the oriented physical page width.
    pub fn width(&self) -> Mm {
        let (width, _) = self.dimensions();
        width
    }

    /// Returns the oriented physical page height.
    pub fn height(&self) -> Mm {
        let (_, height) = self.dimensions();
        height
    }

    /// Returns the width remaining after left and right margins are removed.
    pub fn printable_width(&self) -> Mm {
        self.width() - self.margins.left - self.margins.right
    }

    /// Returns the height remaining after top and bottom margins are removed.
    pub fn printable_height(&self) -> Mm {
        self.height() - self.margins.top - self.margins.bottom
    }
}

/// A vertically arranged section containing report items.
///
/// The declared height is the minimum band height. Layout may expand it when
/// an item extends beyond that height or when text uses auto-height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub kind: BandKind,
    pub height: Mm,
    pub items: Vec<Item>,
}

/// Determines when and how a band participates in report layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BandKind {
    /// Rendered once at the beginning of the report.
    ReportHeader,
    /// Repeated at the top of every physical page.
    PageHeader,
    /// Repeated for every row in the named data source.
    Data { source: String },
    /// Repeated at the bottom of every physical page.
    PageFooter,
    /// Rendered once after the report content.
    ReportFooter,
}

/// A drawable element stored inside a [`Band`].
///
/// The internally tagged representation uses the `type` property in JSON to
/// select the corresponding item variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Item {
    Text(TextItem),
    Line(LineItem),
    Rectangle(RectangleItem),
    Image(ImageItem),
}

/// An RGBA color with one byte per channel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Fully opaque black.
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    /// Fully opaque white.
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    /// Creates a fully opaque color from red, green, and blue channels.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// Inner spacing between an item's bounds and its content.
///
/// All sides are expressed in millimeters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Padding {
    pub left: Mm,
    pub top: Mm,
    pub right: Mm,
    pub bottom: Mm,
}

impl Default for Padding {
    fn default() -> Self {
        Self {
            left: Mm(0.0),
            top: Mm(0.0),
            right: Mm(0.0),
            bottom: Mm(0.0),
        }
    }
}

/// Border configuration for a text item.
///
/// Each side can be enabled independently. Width is expressed in millimeters
/// and defaults to `0.5` when omitted during deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Border {
    #[serde(default)]
    pub left: bool,

    #[serde(default)]
    pub top: bool,

    #[serde(default)]
    pub right: bool,

    #[serde(default)]
    pub bottom: bool,

    #[serde(default = "default_border_width")]
    pub width: f32,
}

fn default_border_width() -> f32 {
    0.5
}

/// A text element positioned relative to its containing band.
///
/// Geometry and padding use millimeters, while `font_size` uses typographic
/// points. Text may contain `${name}` placeholders resolved during layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextItem {
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,

    pub text: String,

    pub font_size: f32,

    #[serde(default = "default_font_family")]
    pub font_family: String,

    #[serde(default)]
    pub bold: bool,

    #[serde(default)]
    pub italic: bool,

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
    pub x1: Mm,
    pub y1: Mm,
    pub x2: Mm,
    pub y2: Mm,
    pub width: Mm,
}

/// An outlined rectangle positioned relative to its containing band.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectangleItem {
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

/// Horizontal placement of text within its padded content area.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

/// Vertical placement of text within its padded content area.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

/// A distance or coordinate measured in millimeters.
///
/// Keeping millimeters in a distinct type prevents report geometry from being
/// confused with font points, screen pixels, or renderer-specific units.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Mm(pub f32);

impl Add for Mm {
    type Output = Mm;
    fn add(self, rhs: Mm) -> Self::Output {
        Mm(self.0 + rhs.0)
    }
}

impl Sub for Mm {
    type Output = Mm;
    fn sub(self, rhs: Mm) -> Self::Output {
        Mm(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_report() {
        let report = Report {
            name: "Test report".to_string(),

            pages: vec![Page {
                size: PageSize::A4,
                orientation: Orientation::Portrait,

                margins: Margins {
                    left: Mm(10.0),
                    top: Mm(10.0),
                    right: Mm(10.0),
                    bottom: Mm(10.0),
                },

                bands: vec![
                    Band {
                        kind: BandKind::ReportHeader,
                        height: Mm(20.0),

                        items: vec![
                            Item::Text(TextItem {
                                x: Mm(10.0),
                                y: Mm(5.0),
                                width: Mm(190.0),
                                height: Mm(10.0),

                                text: "Raport ecografic".to_string(),

                                font_size: 14.0,
                                font_family: default_font_family(),

                                bold: false,
                                italic: false,

                                text_color: default_text_color(),

                                horizontal_align: HorizontalAlign::Center,
                                vertical_align: VerticalAlign::Center,

                                word_wrap: false,
                                auto_height: false,

                                padding: Padding::default(),

                                background: None,
                                border: None,
                            }),
                            Item::Line(LineItem {
                                x1: Mm(10.0),
                                y1: Mm(18.0),
                                x2: Mm(200.0),
                                y2: Mm(18.0),
                                width: Mm(0.5),
                            }),
                        ],
                    },
                    Band {
                        kind: BandKind::Data {
                            source: "items".to_string(),
                        },
                        height: Mm(10.0),
                        items: vec![],
                    },
                ],
            }],
        };

        let json = serde_json::to_string_pretty(&report).unwrap();

        println!("{json}");

        assert!(json.contains("Test report"));
        assert!(json.contains("Raport ecografic"));
    }

    #[test]
    fn deserialize_report() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/simple.report.json"
        );

        let report = Report::from_file(path).unwrap();

        println!("{:#?}", report);

        assert_eq!(report.name, "Structura operațională HoReCa");
        assert_eq!(report.pages.len(), 1);
    }

    #[test]
    fn save_report_to_file() {
        let report = Report {
            name: "Saved report".to_string(),

            pages: vec![Page {
                size: PageSize::A4,
                orientation: Orientation::Portrait,

                margins: Margins {
                    left: Mm(10.0),
                    top: Mm(10.0),
                    right: Mm(10.0),
                    bottom: Mm(10.0),
                },

                bands: vec![],
            }],
        };

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/saved.report.json"
        );

        report.save_to_file(path).unwrap();
    }

    #[test]
    fn page_dimensions() {
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![],
        };

        assert_eq!(page.width(), Mm(210.0));
        assert_eq!(page.height(), Mm(297.0));

        assert_eq!(page.printable_width(), Mm(190.0));
        assert_eq!(page.printable_height(), Mm(277.0));
    }

    #[test]
    fn landscape_page_dimensions() {
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Landscape,
            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },
            bands: vec![],
        };

        assert_eq!(page.width(), Mm(297.0));
        assert_eq!(page.height(), Mm(210.0));
        assert_eq!(page.printable_width(), Mm(277.0));
        assert_eq!(page.printable_height(), Mm(190.0));
    }

    #[test]
    fn image_item_json_round_trip() {
        let json = r#"
    {
        "type": "Image",
        "x": 10.0,
        "y": 5.0,
        "width": 40.0,
        "height": 30.0,
        "source": "images/logo.png"
    }
    "#;

        let item: Item = serde_json::from_str(json).unwrap();

        match &item {
            Item::Image(image) => {
                assert_eq!(image.x, Mm(10.0));
                assert_eq!(image.y, Mm(5.0));
                assert_eq!(image.width, Mm(40.0));
                assert_eq!(image.height, Mm(30.0));
                assert_eq!(image.source, "images/logo.png");
                assert_eq!(image.fit, ImageFit::Stretch);
            }
            _ => panic!("expected an image item"),
        }

        let serialized = serde_json::to_string(&item).unwrap();
        assert!(serialized.contains(r#""fit":"Stretch""#));
        let _: Item = serde_json::from_str(&serialized).unwrap();
    }

    #[test]
    fn deserialize_image_item_with_contain_fit() {
        let json = r#"
        {
            "type": "Image",
            "x": 0.0,
            "y": 0.0,
            "width": 100.0,
            "height": 50.0,
            "source": "images/logo.png",
            "fit": "Contain"
        }
        "#;

        let item: Item = serde_json::from_str(json).unwrap();

        match item {
            Item::Image(image) => assert_eq!(image.fit, ImageFit::Contain),
            _ => panic!("expected an image item"),
        }
    }
}
