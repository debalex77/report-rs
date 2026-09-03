# Artwork release audit

Status: all 44 top-level SVG icons are accounted for: 19 Adwaita collection
matches and 25 created for report-rs. The application logo was generated for
the project, the supplied FreeSVG chef logo source is documented as CC0, and
the PNGEgg approval stamp has been replaced. The database's sole non-NULL
restaurant logo has been visually identified as the same FreeSVG chef artwork.
The current artwork inventory review is complete; see scope limitations below.

## Confirmed collection matches

There are 44 SVG files directly in `assets/`. Nineteen match the StorageB
GNOME 48 Adwaita collection byte-for-byte. Their inventory and licensing notice
are in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

## Project-created SVGs: generation history verified

The other 25 SVGs were created in the project's Codex development session.
The local session record was checked on 2026-09-03, beyond the Git introduction
commits listed below. Current file contents match the SVGs in the creation
patches, ignoring leading/trailing whitespace; the four border icons match
their subsequent correction patches. These project-created SVGs use the
project MIT license, not the Adwaita attribution.

Evidence timestamps (UTC): vertical-centre icon, 2026-08-29 08:30:03;
settings icon, 2026-08-29 10:21:19; eight toolbox/item icons,
2026-08-29 13:38:26; eleven toolbar/sidebar icons, 2026-09-01 19:35:20;
four border icons, 2026-09-02 10:57:09, corrected at 11:20:42/11:21:00.
The private session transcript is not included in the release.

### `a36f593` — interactive report editing

- `delete-item-symbolic.svg`
- `format-align-vertical-center-symbolic.svg`
- `horizontal-layout-symbolic.svg`
- `image-item-symbolic.svg`
- `preferences-system-symbolic.svg`
- `report-band-symbolic.svg`
- `shape-item-symbolic.svg`
- `text-item-symbolic.svg`
- `toolbox-symbolic.svg`
- `vertical-layout-symbolic.svg`

### `605b34d` — controls and text styling

- `data-symbolic.svg`
- `document-new-symbolic.svg`
- `document-open-symbolic.svg`
- `document-save-symbolic.svg`
- `edit-redo-symbolic.svg`
- `edit-undo-symbolic.svg`
- `preview-symbolic.svg`
- `properties-symbolic.svg`
- `status-symbolic.svg`
- `structure-symbolic.svg`
- `view-refresh-symbolic.svg`

### `d1bf716` — query data tools

- `border-bottom-symbolic.svg`
- `border-left-symbolic.svg`
- `border-right-symbolic.svg`
- `border-top-symbolic.svg`

## Raster and embedded artwork

### Application logo

`assets/report-rs-logo.png` was generated for report-rs in the development
session. The recorded command at 2026-08-28 14:05:17 UTC copied the generated
image into this asset; commit `adbbf74` introduced it. The maintainer also
confirmed this provenance. It is not downloaded third-party stock artwork.

### Documented FreeSVG source

The maintainer identified https://freesvg.org/chef-restaurant-logo as the source
of `assets/exemples/chef-restaurant-logo-publicdomainvectors.png`.
The page identifies SVG `180955`, publisher OpenClipart, and Public Domain,
with a link to CC0 1.0. Checked on 2026-09-03; see the third-party notices.
The local PNG and the current website preview have different SHA-256 hashes;
no byte-identical match or particular conversion history is asserted.
On 2026-09-03, the database image columns were inspected. Among the eight
restaurant rows, only `id = 2` has a non-NULL `logo`: a 20,362-byte SVG with
dimensions 430 × 540. Rendering it confirms the same chef hat and crossed
spoons artwork as the maintainer-supplied FreeSVG source and local PNG.
Identification is visual, not a byte-for-byte comparison with a downloaded
upstream SVG. This CC0 source attribution also applies to that BLOB.
No separate Freepik logo was found in the current restaurant image columns;
the supplied Freepik download link is not a license for this project's artwork.

### Replaced approval stamp

On 2026-09-03, replaced the PNGEgg stamp from
https://www.pngegg.com/en/png-ddoau (the maintainer's screenshot shows
“Non-commercial use”) with original geometric SVG artwork under MIT.
`assets/exemples/approved.svg` is the editable source;
`assets/exemples/approved.png` is its transparent 770 × 400 raster rendering.
The PNG also replaces `restaurants.approved` for `id = 2` in
`examples/test.sqlite3`. Other rows have NULL approval images.
SQLite integrity and byte equality with the new PNG were checked after update;
secure deletion and VACUUM were used to remove the old BLOB from database pages.
All twelve current manual screenshots were visually reviewed: none displays
the old stamp. Updated screenshots 02 and 11 also remove or mask the personal
username in the report path.

## Before publication

- Keep notices and license files in the binary release archive.
- Reopen this audit if any artwork or database image is replaced or added.

This audit concerns artwork only; it does not establish redistribution rights
for database records or complete the Rust dependency license review.
