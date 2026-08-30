use super::*;

pub(crate) fn equalize_layout_children(item: &mut Item) -> bool {
    let (layout, horizontal) = match item {
        Item::HorizontalLayout(layout) => (layout, true),
        Item::VerticalLayout(layout) => (layout, false),
        _ => return false,
    };
    if layout.items.is_empty() {
        return false;
    }
    arrange_layout_children(
        &mut layout.items,
        horizontal,
        layout.width.0,
        layout.height.0,
    );
    true
}

pub(crate) fn arrange_layout_children(
    items: &mut [Item],
    horizontal: bool,
    width: f32,
    height: f32,
) {
    if items.is_empty() {
        return;
    }
    let count = items.len() as f32;
    for (index, item) in items.iter_mut().enumerate() {
        let previous = normalized_geometry(item);
        if horizontal {
            let child_width = width / count;
            set_item_frame(item, index as f32 * child_width, 0.0, child_width, height);
            scale_layout_contents(item, previous.2, previous.3, child_width, height);
        } else {
            let child_height = height / count;
            set_item_frame(item, 0.0, index as f32 * child_height, width, child_height);
            scale_layout_contents(item, previous.2, previous.3, width, child_height);
        }
    }
}

pub(crate) fn scale_layout_contents(
    item: &mut Item,
    old_width: f32,
    old_height: f32,
    new_width: f32,
    new_height: f32,
) {
    let Some(layout) = item_layout_mut(item) else {
        return;
    };
    let scale_x = if old_width > 0.0 {
        new_width / old_width
    } else {
        1.0
    };
    let scale_y = if old_height > 0.0 {
        new_height / old_height
    } else {
        1.0
    };
    for child in &mut layout.items {
        let geometry = normalized_geometry(child);
        let child_width = geometry.2 * scale_x;
        let child_height = geometry.3 * scale_y;
        set_item_frame(
            child,
            geometry.0 * scale_x,
            geometry.1 * scale_y,
            child_width,
            child_height,
        );
        scale_layout_contents(child, geometry.2, geometry.3, child_width, child_height);
    }
}

pub(crate) fn flatten_matching_layouts(
    selected: Vec<Item>,
    horizontal: bool,
) -> (Vec<Item>, Option<String>) {
    let retained_name = selected.iter().find_map(|item| match (item, horizontal) {
        (Item::HorizontalLayout(layout), true) | (Item::VerticalLayout(layout), false) => {
            Some(layout.name.clone())
        }
        _ => None,
    });
    let mut children = Vec::new();
    for item in selected {
        match (item, horizontal) {
            (Item::HorizontalLayout(layout), true) | (Item::VerticalLayout(layout), false) => {
                children.extend(layout.items);
            }
            (item, _) => children.push(item),
        }
    }
    (children, retained_name)
}

pub(crate) fn resize_layout_divider(
    item: &mut Item,
    divider: usize,
    horizontal: bool,
    delta: f32,
) -> bool {
    const MIN_CHILD_SIZE: f32 = 1.0;
    let items = match (item, horizontal) {
        (Item::HorizontalLayout(layout), true) => &mut layout.items,
        (Item::VerticalLayout(layout), false) => &mut layout.items,
        _ => return false,
    };
    if divider + 1 >= items.len() || !delta.is_finite() {
        return false;
    }
    let (left_items, right_items) = items.split_at_mut(divider + 1);
    let before = &mut left_items[divider];
    let after = &mut right_items[0];
    let before_geometry = normalized_geometry(before);
    let after_geometry = normalized_geometry(after);
    let available_before = if horizontal {
        before_geometry.2
    } else {
        before_geometry.3
    };
    let available_after = if horizontal {
        after_geometry.2
    } else {
        after_geometry.3
    };
    let delta = delta.clamp(
        MIN_CHILD_SIZE - available_before,
        available_after - MIN_CHILD_SIZE,
    );
    if delta.abs() <= f32::EPSILON {
        return false;
    }
    if horizontal {
        set_item_frame(
            before,
            before_geometry.0,
            before_geometry.1,
            before_geometry.2 + delta,
            before_geometry.3,
        );
        set_item_frame(
            after,
            after_geometry.0 + delta,
            after_geometry.1,
            after_geometry.2 - delta,
            after_geometry.3,
        );
    } else {
        set_item_frame(
            before,
            before_geometry.0,
            before_geometry.1,
            before_geometry.2,
            before_geometry.3 + delta,
        );
        set_item_frame(
            after,
            after_geometry.0,
            after_geometry.1 + delta,
            after_geometry.2,
            after_geometry.3 - delta,
        );
    }
    reflow_layout(before);
    reflow_layout(after);
    true
}

pub(crate) fn set_item_frame(item: &mut Item, x: f32, y: f32, width: f32, height: f32) {
    match item {
        Item::Text(item) => {
            (item.x, item.y, item.width, item.height) = (Mm(x), Mm(y), Mm(width), Mm(height));
        }
        Item::Rectangle(item) => {
            (item.x, item.y, item.width, item.height) = (Mm(x), Mm(y), Mm(width), Mm(height));
        }
        Item::Image(item) => {
            (item.x, item.y, item.width, item.height) = (Mm(x), Mm(y), Mm(width), Mm(height));
        }
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => {
            (item.x, item.y, item.width, item.height) = (Mm(x), Mm(y), Mm(width), Mm(height));
        }
        Item::Line(item) => {
            (item.x1, item.y1, item.x2, item.y2) = (Mm(x), Mm(y), Mm(x + width), Mm(y + height));
        }
    }
}
