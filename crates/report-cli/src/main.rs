use report_core::datasource::ReportContext;
use report_core::font_measurer::RealFontMeasurer;
use report_core::layout::LayoutEngine;
use report_core::model::Report;

use report_pdf::PdfRenderer;

fn main() {
    // Build absolute paths relative to the report-cli crate.
    // CARGO_MANIFEST_DIR points to crates/report-cli.
    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/simple.report.json"
    );

    let output_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../output.pdf");

    // Load and deserialize the report definition from JSON.
    let report = Report::from_file(report_path).expect("Cannot load report");

    // Runtime context used to resolve variables and data sources
    // while the report is being rendered.
    let context = ReportContext::new();

    // Use real font metrics so text wrapping and positioning
    // match the PDF output as closely as possible.
    let measurer = RealFontMeasurer::new();

    // A report may contain multiple logical pages.
    // Each logical page may generate multiple rendered pages
    // because of pagination.
    let mut rendered_pages = Vec::new();

    for page in &report.pages {
        let pages = LayoutEngine::render_with_measurer(page, &context, &measurer);

        rendered_pages.extend(pages);
    }

    // Convert the rendered page model into a PDF document.
    let report_dir = std::path::Path::new(report_path)
        .parent()
        .expect("Report path should have a parent directory");

    PdfRenderer::render_to_file_with_base_dir(&rendered_pages, output_path, report_dir)
        .expect("Cannot create PDF");

    println!("PDF created:");
    println!("{output_path}");
}
