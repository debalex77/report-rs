use std::path::{Path, PathBuf};

pub(crate) fn select_report_file() -> Result<Option<PathBuf>, String> {
    run_file_dialog(&[
        "--file-selection",
        "--title=Load report JSON",
        "--file-filter=Report JSON | *.json",
    ])
}

pub(crate) fn select_report_save_file(
    current_path: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    let suggested = current_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("untitled.report.json"));
    let output = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--save",
            "--confirm-overwrite",
            "--title=Save report JSON",
            "--file-filter=Report JSON | *.json",
            "--filename",
        ])
        .arg(suggested)
        .output()
        .map_err(|error| error.to_string())?;
    dialog_output_path(output)
}

pub(crate) fn select_image_file() -> Result<Option<PathBuf>, String> {
    run_file_dialog(&[
        "--file-selection",
        "--title=Select image",
        "--file-filter=Images | *.png *.jpg *.jpeg",
    ])
}

pub(crate) fn ensure_json_extension(mut path: PathBuf) -> PathBuf {
    let is_json = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if !is_json {
        path.set_extension("json");
    }
    path
}

pub(crate) fn report_directory(path: Option<&Path>) -> PathBuf {
    path.and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn run_file_dialog(arguments: &[&str]) -> Result<Option<PathBuf>, String> {
    let output = std::process::Command::new("zenity")
        .args(arguments)
        .output()
        .map_err(|error| error.to_string())?;
    dialog_output_path(output)
}

fn dialog_output_path(output: std::process::Output) -> Result<Option<PathBuf>, String> {
    if !output.status.success() {
        return Ok(None);
    }
    let path = String::from_utf8(output.stdout)
        .map_err(|error| error.to_string())?
        .trim()
        .to_string();
    Ok((!path.is_empty()).then(|| PathBuf::from(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_report_extension() {
        assert_eq!(
            ensure_json_extension(PathBuf::from("report")),
            PathBuf::from("report.json")
        );
        assert_eq!(
            ensure_json_extension(PathBuf::from("report.JSON")),
            PathBuf::from("report.JSON")
        );
        assert_eq!(
            ensure_json_extension(PathBuf::from("report.txt")),
            PathBuf::from("report.json")
        );
    }
}
