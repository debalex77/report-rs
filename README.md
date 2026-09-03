<p align="center">
  <img src="assets/report-rs-logo.png" alt="report-rs" width="180">
</p>

<h1 align="center">report-rs</h1>

<p align="center">A visual, band-based report designer and rendering engine written in Rust.</p>

<p align="center">
  <a href="https://github.com/debalex77/report-rs/actions/workflows/ci.yml"><img src="https://github.com/debalex77/report-rs/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-yellow.svg" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/Rust-2024-orange?logo=rust" alt="Rust 2024">
  <img src="https://img.shields.io/badge/release-v0.1.0--alpha.1-blue" alt="v0.1.0-alpha.1">
</p>

> **Alpha software:** the report format and user interface may still change.

## What it does

`report-rs` provides a graphical Designer, interactive Preview and PDF renderer
for JSON report templates. It is inspired by traditional band-based reporting
tools while keeping the model and layout engine independent from the renderer.

Key features:

- Report, page, data, group and footer bands with automatic pagination.
- SQLite queries, parameters, filtering and sorting.
- Text, lines, rectangles, file images and database BLOB images.
- Value formatting, report functions and group subtotals.
- Nested horizontal and vertical layouts.
- Visual table generation, including row numbers and multiple grouping levels.
- Preview progress, processing metrics and PDF export.

## Run from source

Requirements: a current stable Rust toolchain and a Linux desktop environment.

Linux runtime dependencies: `zenity` (file dialogs), `xdg-open` (external PDF
viewer launch), and installed fonts. DejaVu Sans is recommended for the examples;
Preview falls back to system fonts if its usual Linux font file is absent.
On Ubuntu/Debian, install `zenity`, `xdg-utils` and `fonts-dejavu-core`.

```bash
git clone https://github.com/debalex77/report-rs.git
cd report-rs
cargo run --release -p report-designer
```

Open a report directly:

```bash
cargo run --release -p report-designer -- examples/group_products.report.json
```

Run Preview directly:

```bash
cargo run --release -p report-preview -- examples/group_products.report.json
```

## Build and test

```bash
cargo build --workspace --release --locked
cargo test --workspace --locked
cargo fmt --all -- --check
```

Release archives contain `report-designer`, `report-preview`, examples and the
license. Download them from [GitHub Releases](https://github.com/debalex77/report-rs/releases).

## Examples

- `simple.report.json` — basic static report.
- `products_price.report.json` — SQLite data and database BLOB images.
- `group_products.report.json` — products grouped by category.
- `nested_group_products.report.json` — nested grouping and subtotals.

The database-backed examples expect `examples/test.sqlite3` beside the report.
The bundled database contains fictional demonstration data, as confirmed by
the maintainer; it is not production data.

## Project layout

```text
crates/report-core      report model, data and layout engine
crates/report-designer  graphical report designer
crates/report-preview   interactive preview and PDF export
crates/report-pdf       PDF renderer
crates/report-cli       minimal embedding example
```

Development notes and architecture are documented in
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). Changes included in each version are
listed in [CHANGELOG.md](CHANGELOG.md). End-user instructions are available in
the [User Manual](docs/UserManual.md).

## License

Project code is licensed under the [MIT License](LICENSE). Third-party artwork
retains its own licenses; see [Third-party notices](THIRD_PARTY_NOTICES.md).
