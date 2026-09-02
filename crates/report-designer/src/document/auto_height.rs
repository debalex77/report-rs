use super::*;
use report_core::font::measurer::RealFontMeasurer;
use report_core::layout::text::TextLayout;
use std::cell::RefCell;
use std::collections::HashMap;

const MAX_TEXT_LAYOUT_CACHE_ENTRIES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextLayoutKey {
    text: String,
    family: String,
    font_size: u32,
    width: u32,
    bold: bool,
    italic: bool,
    word_wrap: bool,
}

thread_local! {
    static DESIGNER_TEXT_MEASURER: RealFontMeasurer = RealFontMeasurer::new();
    static TEXT_LAYOUT_CACHE: RefCell<HashMap<TextLayoutKey, TextLayout>> =
        RefCell::new(HashMap::new());
}

pub(crate) fn designer_text_layout(item: &TextItem) -> TextLayout {
    let width = (item.width.0 - item.padding.left.0 - item.padding.right.0).max(0.0);
    let key = TextLayoutKey {
        text: item.text.clone(),
        family: item.font_family.clone(),
        font_size: item.font_size.to_bits(),
        width: width.to_bits(),
        bold: item.bold,
        italic: item.italic,
        word_wrap: item.word_wrap,
    };
    if let Some(layout) = TEXT_LAYOUT_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return layout;
    }

    let font = item.font_spec();
    let layout = DESIGNER_TEXT_MEASURER.with(|measurer| {
        if item.word_wrap {
            TextLayout::wrap(&item.text, &font, width, measurer)
        } else {
            TextLayout::single_line(&item.text, &font, measurer)
        }
    });
    TEXT_LAYOUT_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= MAX_TEXT_LAYOUT_CACHE_ENTRIES {
            cache.clear();
        }
        cache.insert(key, layout.clone());
    });
    layout
}

#[cfg(test)]
fn clear_text_layout_cache() {
    TEXT_LAYOUT_CACHE.with(|cache| cache.borrow_mut().clear());
}

#[cfg(test)]
fn text_layout_cache_len() -> usize {
    TEXT_LAYOUT_CACHE.with(|cache| cache.borrow().len())
}

pub(crate) fn text_display_height(item: &TextItem) -> f32 {
    if !item.auto_height {
        return item.height.0;
    }
    let layout = designer_text_layout(item);
    item.height
        .0
        .max(item.padding.top.0 + layout.height + item.padding.bottom.0)
}

pub(crate) fn propagate_auto_heights(report: &mut Report) -> bool {
    let mut changed = false;
    for page in &mut report.pages {
        for band in &mut page.bands {
            let mut required = band.height.0;
            for item in &mut band.items {
                let (_, y, _, _) = normalized_geometry(item);
                required = required.max(y + reflow_item(item, &mut changed));
            }
            if required > band.height.0 + f32::EPSILON {
                band.height = Mm(required);
                changed = true;
            }
        }
    }
    changed
}

fn reflow_item(item: &mut Item, changed: &mut bool) -> f32 {
    match item {
        Item::Text(text) => {
            let required = text_display_height(text);
            if required > text.height.0 + f32::EPSILON {
                text.height = Mm(required);
                *changed = true;
            }
            required
        }
        Item::HorizontalLayout(layout) => {
            if layout.items.iter().any(item_has_auto_height) {
                for child in &mut layout.items {
                    enable_auto_height(child, changed);
                }
            }
            let mut required = layout.height.0;
            for child in &mut layout.items {
                let (_, y, _, _) = normalized_geometry(child);
                required = required.max(y + reflow_item(child, changed));
            }
            let layout_height = grow_layout(layout, required, changed);
            // Every horizontal cell follows the expanded row height, not only
            // the text item that triggered AutoHeight.
            for child in &mut layout.items {
                let (x, y, width, height) = normalized_geometry(child);
                let available_height = (layout_height - y).max(height);
                if available_height > height + f32::EPSILON {
                    set_item_frame(child, x, y, width, available_height);
                    *changed = true;
                }
            }
            layout_height
        }
        Item::VerticalLayout(layout) => {
            let mut cursor = 0.0_f32;
            for child in &mut layout.items {
                let (x, original_y, width, height) = normalized_geometry(child);
                let y = original_y.max(cursor);
                if (y - original_y).abs() > f32::EPSILON {
                    set_item_frame(child, x, y, width, height);
                    *changed = true;
                }
                cursor = y + reflow_item(child, changed);
            }
            grow_layout(layout, layout.height.0.max(cursor), changed)
        }
        _ => normalized_geometry(item).3,
    }
}

fn item_has_auto_height(item: &Item) -> bool {
    match item {
        Item::Text(text) => text.auto_height,
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
            layout.items.iter().any(item_has_auto_height)
        }
        _ => false,
    }
}

fn enable_auto_height(item: &mut Item, changed: &mut bool) {
    match item {
        Item::Text(text) => {
            if !text.auto_height {
                text.auto_height = true;
                *changed = true;
            }
        }
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
            for child in &mut layout.items {
                enable_auto_height(child, changed);
            }
        }
        _ => {}
    }
}

fn grow_layout(layout: &mut LayoutItem, required: f32, changed: &mut bool) -> f32 {
    if required > layout.height.0 + f32::EPSILON {
        layout.height = Mm(required);
        *changed = true;
    }
    layout.height.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_text_layout_uses_one_cache_entry() {
        clear_text_layout_cache();
        let mut item = new_text_item("DejaVu Sans".to_string());
        let Item::Text(text) = &mut item else {
            unreachable!();
        };
        text.text = "Responsabil_sdfhjbksdfhhsgdffiaghfkhgqashg".to_string();
        text.word_wrap = true;

        let first = designer_text_layout(text);
        let second = designer_text_layout(text);

        assert_eq!(first.lines.len(), second.lines.len());
        assert_eq!(text_layout_cache_len(), 1);
    }

    #[test]
    fn horizontal_layout_propagates_auto_height_to_sibling_texts() {
        let mut first = new_text_item("DejaVu Sans".to_string());
        let mut second = new_text_item("DejaVu Sans".to_string());
        if let Item::Text(text) = &mut first {
            text.auto_height = true;
        }
        set_item_frame(&mut first, 0.0, 0.0, 40.0, 8.0);
        set_item_frame(&mut second, 40.0, 0.0, 40.0, 8.0);
        let mut layout = Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(80.0),
            height: Mm(8.0),
            items: vec![first, second],
        });
        let mut changed = false;

        reflow_item(&mut layout, &mut changed);

        let Item::HorizontalLayout(layout) = layout else {
            unreachable!();
        };
        assert!(changed);
        assert!(
            layout
                .items
                .iter()
                .all(|item| { matches!(item, Item::Text(text) if text.auto_height) })
        );
    }
}
