use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct TableTemplate {
    pub(crate) name: String,
    pub(crate) columns: Vec<TableTemplateColumn>,
    #[serde(default = "default_true")]
    pub(crate) center_table: bool,
    #[serde(default)]
    pub(crate) include_row_number: bool,
    #[serde(default)]
    pub(crate) groups: Vec<TableTemplateGroup>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct TableTemplateGroup {
    pub(crate) field: String,
    #[serde(default = "default_true")]
    pub(crate) include_header: bool,
    #[serde(default = "default_true")]
    pub(crate) include_footer: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct TableTemplateColumn {
    pub(crate) field: String,
    pub(crate) title: String,
    pub(crate) width: String,
    pub(crate) alignment: String,
    #[serde(default = "default_expression")]
    pub(crate) value_type: String,
    #[serde(default)]
    pub(crate) decimal_places: String,
    #[serde(default)]
    pub(crate) date_pattern: String,
    #[serde(default)]
    pub(crate) prefix: String,
    #[serde(default)]
    pub(crate) suffix: String,
    #[serde(default)]
    pub(crate) grouping: bool,
}

fn default_expression() -> String {
    "Expression".to_string()
}

fn default_true() -> bool {
    true
}

fn templates_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("report-rs").join("table-templates.json"))
}

pub(crate) fn load_table_templates() -> Result<Vec<TableTemplate>, String> {
    let Some(path) = templates_path() else {
        return Ok(Vec::new());
    };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Invalid table template file {}: {error}", path.display()))
}

pub(crate) fn save_table_templates(templates: &[TableTemplate]) -> Result<(), String> {
    let path = templates_path()
        .ok_or_else(|| "Cannot determine the user configuration directory".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create {}: {error}", parent.display()))?;
    }
    let contents = serde_json::to_string_pretty(templates)
        .map_err(|error| format!("Cannot serialize table templates: {error}"))?;
    fs::write(&path, contents).map_err(|error| format!("Cannot write {}: {error}", path.display()))
}

pub(crate) fn alignment_name(alignment: &HorizontalAlign) -> &'static str {
    match alignment {
        HorizontalAlign::Left => "Left",
        HorizontalAlign::Center => "Center",
        HorizontalAlign::Right => "Right",
    }
}

pub(crate) fn parse_alignment(alignment: &str) -> HorizontalAlign {
    match alignment {
        "Center" => HorizontalAlign::Center,
        "Right" => HorizontalAlign::Right,
        _ => HorizontalAlign::Left,
    }
}

pub(crate) fn value_type_name(value_type: ValueType) -> &'static str {
    match value_type {
        ValueType::Text => "Text",
        ValueType::Integer => "Integer",
        ValueType::Double => "Double",
        ValueType::Boolean => "Boolean",
        ValueType::Date => "Date",
        ValueType::DateTime => "DateTime",
        ValueType::Expression => "Expression",
        ValueType::Function => "Function",
    }
}

pub(crate) fn parse_value_type(value_type: &str) -> ValueType {
    match value_type {
        "Integer" => ValueType::Integer,
        "Double" => ValueType::Double,
        "Boolean" => ValueType::Boolean,
        "Date" => ValueType::Date,
        "DateTime" => ValueType::DateTime,
        "Text" => ValueType::Text,
        "Function" => ValueType::Function,
        _ => ValueType::Expression,
    }
}
