use super::*;

#[path = "document/auto_height.rs"]
mod auto_height;
#[path = "document/geometry.rs"]
mod geometry;
#[path = "document/items.rs"]
mod items;
#[path = "document/layouts.rs"]
mod layouts;
#[path = "document/naming.rs"]
mod naming;
#[path = "document/table.rs"]
mod table;

pub(super) use auto_height::*;
pub(super) use geometry::*;
pub(super) use items::*;
pub(super) use layouts::*;
pub(super) use naming::*;
pub(super) use table::*;

pub(super) fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{shortened}…")
    } else {
        shortened
    }
}

pub(super) fn blank_report() -> Report {
    Report {
        name: "Untitled report".to_string(),
        parameters: Vec::new(),
        data_sources: Vec::new(),
        pages: vec![Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,
            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },
            bands: Vec::new(),
        }],
    }
}

pub(super) fn same_band_kind(left: &BandKind, right: &BandKind) -> bool {
    matches!(
        (left, right),
        (BandKind::ReportHeader, BandKind::ReportHeader)
            | (BandKind::PageHeader, BandKind::PageHeader)
            | (BandKind::Data { .. }, BandKind::Data { .. })
            | (BandKind::DataHeader { .. }, BandKind::DataHeader { .. })
            | (BandKind::PageFooter, BandKind::PageFooter)
            | (BandKind::ReportFooter, BandKind::ReportFooter)
    )
}

pub(super) fn report_contains_band(report: &Report, band: usize) -> bool {
    report
        .pages
        .first()
        .is_some_and(|page| band < page.bands.len())
}

pub(super) fn report_contains_selection(report: &Report, selection: Selection) -> bool {
    item_at_selection(report, selection).is_some()
}

pub(super) fn item_layout(item: &Item) -> Option<&report_core::model::LayoutItem> {
    match item {
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => Some(layout),
        _ => None,
    }
}

pub(super) fn item_layout_mut(item: &mut Item) -> Option<&mut report_core::model::LayoutItem> {
    match item {
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => Some(layout),
        _ => None,
    }
}

pub(super) fn first_text_item(item: &Item) -> Option<&TextItem> {
    match item {
        Item::Text(text) => Some(text),
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
            layout.items.iter().find_map(first_text_item)
        }
        _ => None,
    }
}

pub(super) fn update_text_items(item: &mut Item, update: &mut impl FnMut(&mut TextItem)) -> usize {
    match item {
        Item::Text(text) => {
            update(text);
            1
        }
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => layout
            .items
            .iter_mut()
            .map(|child| update_text_items(child, update))
            .sum(),
        _ => 0,
    }
}

pub(super) fn item_at_selection(report: &Report, selection: Selection) -> Option<&Item> {
    let mut item = report
        .pages
        .first()?
        .bands
        .get(selection.band)?
        .items
        .get(selection.top_index())?;
    for &index in selection.descendants() {
        item = item_layout(item)?.items.get(index)?;
    }
    Some(item)
}

pub(super) fn item_at_selection_mut(
    report: &mut Report,
    selection: Selection,
) -> Option<&mut Item> {
    let item = report
        .pages
        .first_mut()?
        .bands
        .get_mut(selection.band)?
        .items
        .get_mut(selection.top_index())?;
    item_at_descendant_mut(item, selection.descendants())
}

pub(super) fn item_at_descendant_mut<'a>(
    item: &'a mut Item,
    path: &[usize],
) -> Option<&'a mut Item> {
    let Some((&index, rest)) = path.split_first() else {
        return Some(item);
    };
    let child = item_layout_mut(item)?.items.get_mut(index)?;
    item_at_descendant_mut(child, rest)
}

pub(super) fn remove_item_at_path(items: &mut Vec<Item>, path: &[usize]) -> bool {
    let Some((&index, rest)) = path.split_first() else {
        return false;
    };
    if rest.is_empty() {
        if index >= items.len() {
            return false;
        }
        items.remove(index);
        return true;
    }
    let Some(parent) = items.get_mut(index) else {
        return false;
    };
    let Some(layout) = item_layout_mut(parent) else {
        return false;
    };
    if !remove_item_at_path(&mut layout.items, rest) {
        return false;
    }
    reflow_layout(parent);
    true
}

pub(super) fn resize_band_height(page: &mut Page, band_index: usize, dy: f32) -> bool {
    if band_index >= page.bands.len() || !dy.is_finite() {
        return false;
    }
    let other_height: f32 = page
        .bands
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != band_index)
        .map(|(_, band)| band.height.0)
        .sum();
    let item_height = page.bands[band_index]
        .items
        .iter()
        .map(|item| {
            let (_, y, _, height) = geometry_values(item);
            if matches!(item, Item::Line(_)) {
                y.max(height)
            } else {
                y + height
            }
        })
        .fold(5.0_f32, f32::max);
    let max_height = (page.printable_height().0 - other_height).max(item_height);
    let old_height = page.bands[band_index].height.0;
    let new_height = (old_height + dy).clamp(item_height, max_height);
    page.bands[band_index].height = Mm(new_height);
    (new_height - old_height).abs() > f32::EPSILON
}

pub(super) fn fit_band_to_contents(page: &mut Page, band_index: usize) -> bool {
    let Some(band) = page.bands.get(band_index) else {
        return false;
    };
    if band.items.is_empty() {
        let delta = 5.0 - band.height.0;
        return resize_band_height(page, band_index, delta);
    }
    let content_top = band
        .items
        .iter()
        .map(|item| normalized_geometry(item).1)
        .fold(f32::INFINITY, f32::min);
    let content_bottom = band
        .items
        .iter()
        .map(|item| {
            let (_, y, _, height) = normalized_geometry(item);
            y + height
        })
        .fold(f32::NEG_INFINITY, f32::max);
    let old_height = band.height.0;
    let content_height = (content_bottom - content_top).max(5.0);

    let moved = content_top.abs() > f32::EPSILON;
    if moved && let Some(band) = page.bands.get_mut(band_index) {
        for item in &mut band.items {
            let (x, y, _, _) = normalized_geometry(item);
            set_item_origin(item, x, y - content_top);
        }
    }
    let resized = resize_band_height(page, band_index, content_height - old_height);
    moved || resized
}

pub(super) fn move_band(page: &mut Page, from: usize, to: usize) -> bool {
    if from >= page.bands.len() || to >= page.bands.len() || from == to {
        return false;
    }
    page.bands.swap(from, to);
    true
}
