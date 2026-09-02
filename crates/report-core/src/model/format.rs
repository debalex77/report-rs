use super::*;

/// Optional presentation rules applied after a text item's value is resolved.
///
/// An empty format preserves the resolved value exactly, which keeps existing
/// report definitions backward compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ValueFormat {
    /// Number of digits displayed after the decimal separator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<u8>,

    /// Date/time pattern, for example `dd.MM.yyyy` or `dd.MM.yyyy HH:mm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_pattern: Option<String>,

    /// Text inserted before the formatted value.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prefix: String,

    /// Text appended after the formatted value.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub suffix: String,

    /// Enables digit grouping for numeric values.
    #[serde(default, skip_serializing_if = "is_false")]
    pub grouping: bool,
}

impl ValueFormat {
    pub(crate) fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}
