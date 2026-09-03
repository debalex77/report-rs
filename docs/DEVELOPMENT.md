# Development

## Architecture

The workspace separates the declarative report model from output rendering:

```text
Report JSON → data loading → layout engine → RenderedPage → Preview / PDF
```

- `report-core` owns the serializable model, SQLite integration, expressions,
  font measurement, image decoding and pagination.
- `report-designer` edits the model visually and launches Preview as a separate
  process.
- `report-preview` loads report data, renders pages and exports PDF.
- `report-pdf` converts the renderer-independent pages to PDF.
- `report-cli` is a small integration example.

## Developer commands

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo build --workspace --release --locked
```

The GitHub Actions CI workflow runs formatting and all workspace tests for
pushes and pull requests.

## Report format

Reports are stored as JSON. Geometry is expressed in millimetres and text size
in typographic points. Runtime query results and parameters are not persisted in
the template.

Band execution order is defined by the report, while nested groups are ordered
by their Group Header declarations. Data used by groups must be sorted by the
same fields, from the outermost group to the innermost group.

Examples in the `examples` directory are the current reference for supported
model properties.

## Release process

1. Run formatting, tests and the release build.
2. Smoke-test Designer, Preview and PDF export using the bundled examples.
3. Update `CHANGELOG.md` and the version in every crate manifest.
4. Commit and push the release changes.
5. Create and push an annotated tag, for example `v0.1.0-alpha.1`.
6. The release workflow builds and attaches the Linux archive and checksum.

### Publication gates

- Keep [ARTWORK_AUDIT.md](ARTWORK_AUDIT.md) current. The 44 top-level icons,
  application logo, example PNGs and restaurant image BLOBs have been reviewed.
  This artwork review does not replace dependency licensing or sample-data review.
- Recheck screenshots when artwork changes. The current twelve manual images
  have been reviewed and do not display the replaced PNGEgg stamp.
- Test the extracted archive outside the source checkout, with Designer and
  Preview together. Check SQLite examples, parameters, Save PDF, cancellation,
  overwrite confirmation and Open PDF.
- Check the documented Linux dependencies and font fallback on a clean supported system.
- Do not push a release tag until these checks are complete: the tag workflow
  publishes a prerelease automatically, not a draft.

### Dependency notice collection

Run `python3 scripts/dependency_notices.py /tmp/report-rs-notices --offline`
using a fresh output directory. The release workflow runs this collector too.
It copies license/notice files and preserves exact cached MPL-2.0 crate sources.
It accepts root-crate license evidence for subcrates only when repository,
VCS revision and license expression match. The inventory intentionally includes
resolved build/test dependencies and is not a binary linkage analysis.

Local check on 2026-09-03: collection passes for 429 Linux-target packages, all
with declared license expressions. Upstream evidence resolved 22 of the initial
35 gaps. The remaining thirteen use explicitly reviewed, version-pinned Cargo
declarations plus standard SPDX license texts, with complete original crate
sources and supplied author/notice metadata retained. The exact cases and
selected license alternatives are in `LICENSES/standard/reviewed-declarations.json`.
No automatic fallback is permitted for new packages. Manifest and standard-text
SHA-256 checks must pass; changed declarations require review.

This checks document collection, not legal compliance or copyright ownership.
In particular, the two MIT-only cases `convert_case` and `kuchiki` lack a
package-specific license/copyright file; their declared MIT grant, standard
terms, author metadata and complete source are retained without inventing a
copyright notice. See `LICENSES/standard/README.md`. Native/system dependency
obligations are outside this collector's scope.

Collector unit tests: `python3 -B -m unittest discover -s scripts -p 'test_*.py'`.

Retrieved upstream files are retained under `LICENSES/dependencies/`, with
source URLs, published VCS revisions and SHA-256 checksums in `index.json`.
`scripts/fetch_dependency_licenses.py` is a maintainer-only retrieval helper;
review its output before adding it. Release builds use the checked-in evidence
without GitHub requests. The zune-core repository mapping was checked against
its packaged original Cargo manifest at the recorded commit.

### Sample-data review

The maintainer confirmed on 2026-09-03 that all data in `examples/test.sqlite3`
is fictional, including names, phone numbers, addresses, salaries and tax IDs,
and may be published as example data. All inspected restaurant/counterparty
email domains end in `.example`. Reconfirm this if the example data is replaced.
