# report-rs

![report-rs logo](assets/report-rs-logo.png)

A report generation engine written in Rust.

`report-rs` is an experimental reporting engine inspired by traditional
band-based report systems such as LimeReport.

The project is being developed from scratch in Rust and focuses on separating
the report model, layout engine, preview, and output rendering.

## Current features

- JSON report serialization and deserialization
- Page size, orientation, and margins
- Band-based report layout
- Report header and footer
- Page header and footer
- Data bands
- Automatic pagination
- Data source tables and report variables
- Runtime report parameters
- Text items
- Word wrapping
- Automatic text height
- Horizontal and vertical text alignment
- Text padding
- Font family, size, bold, and italic styles
- Real font measurement using `cosmic-text`
- System font resolution
- Text color and background
- Borders
- Line items
- Rectangle items
- Image items with PNG and JPEG support
- Interactive report preview
- Zoom and page navigation
- PDF generation
- PDF export directly from the preview
- Initial report designer with item selection and JSON reload/save

## Project structure

```text
report-rs/
├── crates/
│   ├── report-core/
│   ├── report-pdf/
│   ├── report-preview/
│   ├── report-designer/
│   └── report-cli/
├── examples/
├── Cargo.toml
└── README.md
```

### report-core

Contains the core report model and layout engine.

Responsibilities include:

- report and page model
- bands and report items
- data sources and variables
- text layout and word wrapping
- font measurement
- font resolution
- pagination
- generation of rendered pages

### report-pdf

Converts rendered pages produced by `report-core` into PDF documents.

PDF generation is implemented using `printpdf`.

### report-preview

Interactive graphical report preview implemented using `iced`.

It supports page navigation, zooming, debug visualization, and PDF export.

### report-designer

Early graphical report designer implemented using `iced`.

It can load a report definition, display its bands and items on a design
canvas, select and drag items, inspect basic geometry, reload, and save JSON.

### report-cli

Small command-line application demonstrating how to load a report, run the
layout engine, and generate a PDF document.

## Report pipeline

```text
Report JSON
     │
     ▼
 Report model
     │
     ▼
Layout Engine
     │
     ▼
RenderedPage
   ┌─┴──────────┐
   ▼            ▼
Preview       PDF
```

The layout engine is independent from the final output format. Both the
preview and PDF renderer consume the same rendered page representation.

## Example

An example report is available in:

```text
examples/simple.report.json
```

Generate a PDF using:

```bash
cargo run -p report-cli
```

Run the graphical preview using:

```bash
cargo run -p report-preview
```

Run the report designer using:

```bash
cargo run -p report-designer -- examples/simple.report.json
```

## Building

Build the complete workspace:

```bash
cargo build --workspace
```

Run all tests:

```bash
cargo test --workspace
```

Format the source code:

```bash
cargo fmt --all
```

## Status

`report-rs` is currently in early development.

The report model, text layout, pagination, preview, and basic PDF rendering
are functional. More report items and rendering features will be added as the
engine evolves.

## Planned features

- Improved font fallback
- Additional data source support
- Expressions
- More advanced page layout
- Drag, resize, and property editing in the report designer

## License

This project is licensed under the MIT License.
