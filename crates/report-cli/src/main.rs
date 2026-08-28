use report_core::datasource::{ReportContext, Row, Value};
use report_core::font_measurer::RealFontMeasurer;
use report_core::layout::LayoutEngine;
use report_core::model::Report;

use report_pdf::PdfRenderer;

fn example_context() -> ReportContext {
    let units = [
        ("Bucătăria principală", "Chef executiv"),
        ("Zona de pregătire rece", "Sous-chef"),
        ("Restaurant - salon principal", "Manager de sală"),
        ("Terasă și servire exterioară", "Supervizor terasă"),
        ("Bar și băuturi", "Manager bar"),
        ("Depozit și recepție marfă", "Gestionar"),
        ("Serviciul catering", "Coordonator catering"),
        ("Igienizare și mentenanță", "Supervizor operațional"),
    ];

    let rows = units
        .into_iter()
        .enumerate()
        .map(|(index, (unit_name, responsible_role))| {
            let mut row = Row::new();
            row.insert("nr".to_string(), Value::Number((index + 1) as f64));
            row.insert(
                "unit_name".to_string(),
                Value::String(unit_name.to_string()),
            );
            row.insert(
                "responsible_role".to_string(),
                Value::String(responsible_role.to_string()),
            );
            row
        })
        .collect();

    let mut context = ReportContext::new();
    context.set_parameter(
        "report_subtitle",
        Value::String("Model demonstrativ pentru unități și zone de lucru".to_string()),
    );
    context.set_parameter(
        "approval_role",
        Value::String("Manager operațional".to_string()),
    );
    context.add_table("horeca_units", rows);
    context
}

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
    let context = example_context();

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
