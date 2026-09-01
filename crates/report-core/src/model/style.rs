use super::*;

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
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
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
