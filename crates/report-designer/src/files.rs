use std::path::{Path, PathBuf};

use report_core::model::Report;

pub(crate) fn launch_preview(
    report: &Report,
    report_path: Option<&Path>,
) -> Result<(PathBuf, std::time::Instant), String> {
    let directory = std::fs::canonicalize(report_directory(report_path))
        .map_err(|error| format!("cannot resolve report directory: {error}"))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let preview_path = directory.join(format!(
        ".report-rs-preview-{}-{nonce}.report.json",
        std::process::id()
    ));
    let ready_path = directory.join(format!(
        ".report-rs-preview-{}-{nonce}.ready",
        std::process::id()
    ));
    report
        .save_to_file(preview_path.to_string_lossy().as_ref())
        .map_err(|error| format!("cannot write temporary report: {error}"))?;

    let current_executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate Designer executable: {error}"))?;
    let sibling_preview = current_executable.with_file_name("report-preview");
    let release_preview = current_executable
        .parent()
        .and_then(Path::parent)
        .map(|target| target.join("release").join("report-preview"));
    let preview_executable = if cfg!(debug_assertions) {
        release_preview
            .filter(|path| path.is_file())
            .unwrap_or(sibling_preview)
    } else {
        sibling_preview
    };
    // During development, Cargo may leave an older top-level binary while
    // tests build only hashed executables in `target/debug/deps`. Running via
    // Cargo guarantees Preview matches the current sources and accepts the
    // temporary report path. Packaged release builds use the sibling binary.
    let mut command = if preview_executable.is_file() {
        std::process::Command::new(preview_executable)
    } else {
        let mut command = std::process::Command::new("cargo");
        command
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .args(["run", "--quiet", "-p", "report-preview", "--"]);
        command
    };
    let started = std::time::Instant::now();
    let child = match command
        .arg(&preview_path)
        .arg("--ready-file")
        .arg(&ready_path)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let _ = std::fs::remove_file(&preview_path);
            return Err(format!("cannot start report-preview: {error}"));
        }
    };
    let ready_for_watcher = ready_path.clone();
    std::thread::spawn(move || {
        let mut child = child;
        let status = child.wait();
        let completed = std::fs::read_to_string(&ready_for_watcher)
            .ok()
            .is_some_and(|contents| !contents.starts_with("PROGRESS:"));
        if !completed {
            let message = match status {
                Ok(status) => format!("ERROR: report-preview exited before opening ({status})"),
                Err(error) => format!("ERROR: cannot monitor report-preview: {error}"),
            };
            let _ = std::fs::write(&ready_for_watcher, message);
        }
        let _ = std::fs::remove_file(preview_path);
    });
    Ok((ready_path, started))
}

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

pub(crate) fn select_sqlite_file() -> Result<Option<PathBuf>, String> {
    run_file_dialog(&[
        "--file-selection",
        "--title=Select SQLite database",
        "--file-filter=SQLite database | *.sqlite *.sqlite3 *.db",
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
