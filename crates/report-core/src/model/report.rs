use super::*;

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
    /// Input values requested before the report is rendered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ReportParameter>,
    /// Declarative database sources available to report data bands.
    ///
    /// The field is optional in JSON so existing report definitions remain
    /// compatible. Credentials are intentionally not stored in the report.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_sources: Vec<DataSourceDefinition>,
    pub pages: Vec<Page>,
}

/// Declarative input parameter available to expressions and data queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: ReportParameterType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
}

/// Supported report parameter value types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReportParameterType {
    #[default]
    Text,
    Integer,
    Double,
    Boolean,
    Date,
    DateTime,
}

/// A named connection and the queries whose results become report tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataSourceDefinition {
    pub name: String,
    pub connection: DataConnection,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queries: Vec<DataQuery>,
}

/// Connection information safe to persist in a report definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DataConnection {
    Sqlite { path: String },
}

/// A named query. Its name is referenced by [`BandKind::Data`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataQuery {
    pub name: String,
    pub sql: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<QueryFilter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sorts: Vec<QuerySort>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuerySort {
    pub field: String,
    #[serde(default)]
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryFilter {
    pub field: String,
    pub operator: FilterOperator,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOperator {
    Equal,
    NotEqual,
    Contains,
    StartsWith,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    IsNull,
    IsNotNull,
}

fn is_false(value: &bool) -> bool {
    !*value
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
