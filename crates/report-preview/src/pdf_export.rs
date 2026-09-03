use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn choose_destination(default: &Path) -> Result<Option<PathBuf>, String> {
    let output = Command::new("zenity")
        .args([
            "--file-selection",
            "--save",
            "--confirm-overwrite",
            "--title=Export PDF",
            "--file-filter=PDF files | *.pdf",
        ])
        .arg("--filename")
        .arg(default)
        .output()
        .map_err(|error| format!("Cannot open Save dialog (install zenity): {error}"))?;
    parse_destination(output)
}

fn parse_destination(output: Output) -> Result<Option<PathBuf>, String> {
    if output.status.code() == Some(1) {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(format!(
            "Save dialog failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let path = String::from_utf8(output.stdout)
        .map_err(|_| "Save dialog returned an invalid filename".to_string())?;
    let path = path.trim_end_matches(['\r', '\n']);
    if path.is_empty() {
        return Ok(None);
    }
    // Keep the exact selected path: changing the extension after the dialog
    // would bypass its overwrite confirmation for the actual destination.
    Ok(Some(PathBuf::from(path)))
}

pub(crate) fn open_pdf(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("Exported PDF no longer exists: {}", path.display()));
    }
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Cannot open PDF: {error}"))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn output(code: i32, path: &[u8]) -> Output {
        Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: path.to_vec(),
            stderr: b"dialog error".to_vec(),
        }
    }

    #[test]
    fn preserves_selected_path_and_spaces() {
        assert_eq!(
            parse_destination(output(0, b"/tmp/my report.pdf\n")).unwrap(),
            Some(PathBuf::from("/tmp/my report.pdf"))
        );
        assert_eq!(
            parse_destination(output(0, b"/tmp/report\n")).unwrap(),
            Some(PathBuf::from("/tmp/report"))
        );
    }

    #[test]
    fn cancellation_and_empty_selection_do_not_export() {
        assert!(parse_destination(output(1, b"")).unwrap().is_none());
        assert!(parse_destination(output(0, b"\n")).unwrap().is_none());
    }

    #[test]
    fn reports_dialog_failure_and_invalid_encoding() {
        assert!(parse_destination(output(2, b"")).is_err());
        assert!(parse_destination(output(0, &[0xff])).is_err());
    }
}
