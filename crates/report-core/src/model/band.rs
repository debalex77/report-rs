use super::*;

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
    /// Optional column header associated with a data source.
    DataHeader {
        source: String,
        #[serde(default = "default_true")]
        repeat_on_each_page: bool,
    },
    /// Printed when the value of `field` changes in the associated data source.
    GroupHeader {
        source: String,
        field: String,
        #[serde(default)]
        repeat_on_each_page: bool,
    },
    /// Printed after the last row of each group in the associated data source.
    GroupFooter { source: String, field: String },
    /// Repeated at the bottom of every physical page.
    PageFooter,
    /// Rendered once after the report content.
    ReportFooter,
}

fn default_true() -> bool {
    true
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
    HorizontalLayout(LayoutItem),
    VerticalLayout(LayoutItem),
}

/// A container that will arrange its child items along one axis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutItem {
    #[serde(default)]
    pub name: String,
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,
    #[serde(default)]
    pub items: Vec<Item>,
}
