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
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct TableTemplateColumn {
    pub(crate) field: String,
    pub(crate) title: String,
    pub(crate) width: String,
    pub(crate) alignment: String,
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
