<p align="center">
  <a href="https://www.rust-lang.org/">
    <img src="https://img.shields.io/badge/Rust-2024-orange?logo=rust" alt="Rust">
  </a>
<a href="https://iced.rs/">
  <img src="https://img.shields.io/badge/GUI-Iced-blue?logo=rust&logoColor=white" alt="Iced">
</a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
  </a>
  <a href="https://github.com/debalex77/report-rs/stargazers">
    <img src="https://img.shields.io/github/stars/debalex77/report-rs?style=flat" alt="GitHub stars">
  </a>
  <a href="https://github.com/debalex77/report-rs/commits/main">
    <img src="https://img.shields.io/github/last-commit/debalex77/report-rs" alt="Last commit">
  </a>
  <a href="https://github.com/debalex77/report-rs/actions/workflows/ci.yml">
    <img src="https://github.com/debalex77/report-rs/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
</p>

# report-rs

![report-rs logo](assets/report-rs-logo.png)

A report generation engine written in Rust.

`report-rs` is an experimental reporting engine inspired by traditional
band-based report systems such as LimeReport.

The project is being developed from scratch in Rust and focuses on separating
the report model, layout engine, preview, and output rendering.

## ✨ Current features

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
- Graphical report designer with New, Load, Save, Reload, Undo, and Redo
- Designer toolbox for bands, text, images, shapes, and layouts
- Interactive item selection, drag, resize, and property editing
- Collapsible Properties panel with geometry, text, font, color, and alignment controls
- Millimeter rulers, design grid, zoom, and resizeable side panels
- Horizontal and vertical layouts with nested-layout support
- Designer settings for page orientation, margins, and default font family
- Properties and Structure tabs with a recursive report tree
- Structure tree SVG icons, expand/collapse, direct rename, and synchronized selection
- Structure tree multi-selection with Ctrl/Shift and hierarchical Drag & Drop
- Drag & Drop between bands and layouts with insertion/containment indicators
- Layout dismantling and explicit handling when moving nested items between bands
- Automatic band growth and contextual Fit band to contents

## 🧱 Project structure

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

```text
report-core/src/
├── datasource/
│   ├── context.rs
│   ├── mod.rs
│   ├── provider.rs
│   └── sqlite.rs
├── font/
│   ├── mod.rs
│   ├── measurer.rs
│   └── resolver.rs
├── image/
│   ├── mod.rs
│   ├── layout.rs
│   └── loader.rs
├── layout/
│   ├── mod.rs
│   └── text.rs
├── model/
│   ├── band.rs
│   ├── item.rs
│   ├── mod.rs
│   ├── page.rs
│   ├── report.rs
│   ├── style.rs
│   └── tests.rs
├── common.rs
└── lib.rs
```

The original flat module paths remain available as compatibility re-exports,
while new code uses the grouped module paths shown above.

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

Graphical report designer implemented using `iced`.

It can create, load, edit, and save JSON report definitions. The design canvas
provides millimeter rulers and a grid, band and item tools, drag and resize
handles, Undo/Redo history, and a resizeable Properties panel. Text, image,
shape, horizontal-layout, and vertical-layout items can be added visually.
Layouts can contain other layouts while their nested items remain selectable
and editable. The Structure tab presents the full report hierarchy and supports
direct renaming, multi-selection, and Drag & Drop between bands and layouts.
Layouts can be dismantled from the context menu, while bands grow automatically
when their content moves beyond the lower edge and can be fitted explicitly to
their contents.

### report-cli

Small command-line application demonstrating how to load a report, run the
layout engine, and generate a PDF document.

## 📈 Report pipeline

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

## 📝 Example

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

Start it with an empty report using:

```bash
cargo run -p report-designer
```

## 📦 Building

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

## 🟢 Status

`report-rs` is currently in early development.

The report model, text layout, pagination, preview, and basic PDF rendering
are functional. More report items and rendering features will be added as the
engine evolves.

## 💭 Planned features

- Improved font fallback
- Additional data source support
- Expressions
- More advanced page layout and layout constraints
- Database-backed report data and query configuration
- Additional Designer property editors and item types

## ⚖️ License

This project is licensed under the MIT License.
