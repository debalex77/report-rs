use super::*;

pub(crate) fn new_text_item(font_family: String) -> Item {
    Item::Text(TextItem {
        name: String::new(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(50.0),
        height: Mm(10.0),
        text: "Text".to_string(),
        font_size: 12.0,
        font_family,
        bold: false,
        italic: false,
        text_color: ReportColor::BLACK,
        horizontal_align: HorizontalAlign::Left,
        vertical_align: VerticalAlign::Center,
        word_wrap: false,
        auto_height: false,
        padding: Padding::default(),
        background: None,
        border: None,
    })
}

pub(crate) fn new_image_item() -> Item {
    Item::Image(ImageItem {
        name: String::new(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(50.0),
        height: Mm(20.0),
        source: String::new(),
        fit: ImageFit::Contain,
    })
}

pub(crate) fn new_shape_item() -> Item {
    Item::Rectangle(RectangleItem {
        name: String::new(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(40.0),
        height: Mm(20.0),
        border_width: Mm(0.5),
    })
}

#[cfg(test)]
pub(crate) fn new_layout_item(horizontal: bool) -> Item {
    let layout = LayoutItem {
        name: String::new(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(60.0),
        height: Mm(20.0),
        items: Vec::new(),
    };
    if horizontal {
        Item::HorizontalLayout(layout)
    } else {
        Item::VerticalLayout(layout)
    }
}

pub(crate) fn find_free_item_position(
    item: &Item,
    siblings: &[Item],
    band_width: f32,
    band_height: f32,
) -> Option<(f32, f32)> {
    const STEP: f32 = 5.0;
    const GAP: f32 = 1.0;
    let (_, _, width, height) = normalized_geometry(item);
    if width > band_width || height > band_height {
        return None;
    }
    let rows = ((band_height - height) / STEP).floor() as usize;
    let columns = ((band_width - width) / STEP).floor() as usize;
    for row in 0..=rows {
        let y = row as f32 * STEP;
        for column in 0..=columns {
            let x = column as f32 * STEP;
            let candidate = (x, y, width, height);
            if siblings
                .iter()
                .all(|sibling| !rectangles_overlap(candidate, normalized_geometry(sibling), GAP))
            {
                return Some((x, y));
            }
        }
    }
    None
}

pub(crate) fn normalized_geometry(item: &Item) -> (f32, f32, f32, f32) {
    let (x, y, width, height) = geometry_values(item);
    if matches!(item, Item::Line(_)) {
        (
            x.min(width),
            y.min(height),
            (width - x).abs(),
            (height - y).abs(),
        )
    } else {
        (x, y, width, height)
    }
}

pub(crate) fn rectangles_overlap(
    left: (f32, f32, f32, f32),
    right: (f32, f32, f32, f32),
    gap: f32,
) -> bool {
    let (lx, ly, lw, lh) = left;
    let (rx, ry, rw, rh) = right;
    lx < rx + rw + gap && lx + lw + gap > rx && ly < ry + rh + gap && ly + lh + gap > ry
}

pub(crate) fn layout_label_bounds(
    item: &Item,
    bounds: Rectangle,
    nested_in_horizontal: bool,
) -> Option<Rectangle> {
    let width = (item_name(item).chars().count() as f32 * 6.2 + 14.0).clamp(76.0, 180.0);
    match item {
        Item::HorizontalLayout(_) => Some(Rectangle::new(
            Point::new(bounds.x, bounds.y - 20.0),
            Size::new(width, 18.0),
        )),
        Item::VerticalLayout(_) if nested_in_horizontal => Some(Rectangle::new(
            Point::new(
                (bounds.x + bounds.width - width - 2.0).max(bounds.x + 2.0),
                bounds.y + 2.0,
            ),
            Size::new(width.min((bounds.width - 4.0).max(18.0)), 18.0),
        )),
        Item::VerticalLayout(_) => Some(Rectangle::new(
            Point::new(bounds.x + bounds.width + 2.0, bounds.y),
            Size::new(width, 18.0),
        )),
        _ => None,
    }
}

pub(crate) fn set_item_origin(item: &mut Item, x: f32, y: f32) {
    let (old_x, old_y, _, _) = normalized_geometry(item);
    move_item(item, x - old_x, y - old_y, f32::MAX, f32::MAX);
}
