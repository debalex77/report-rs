use super::*;

pub(crate) fn move_item(item: &mut Item, dx: f32, dy: f32, band_width: f32, band_height: f32) {
    let (min_x, max_x, min_y, max_y) = match item {
        Item::Text(item) => (
            item.x.0,
            item.x.0 + item.width.0,
            item.y.0,
            item.y.0 + item.height.0,
        ),
        Item::Rectangle(item) => (
            item.x.0,
            item.x.0 + item.width.0,
            item.y.0,
            item.y.0 + item.height.0,
        ),
        Item::Image(item) => (
            item.x.0,
            item.x.0 + item.width.0,
            item.y.0,
            item.y.0 + item.height.0,
        ),
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => (
            item.x.0,
            item.x.0 + item.width.0,
            item.y.0,
            item.y.0 + item.height.0,
        ),
        Item::Line(item) => (
            item.x1.0.min(item.x2.0),
            item.x1.0.max(item.x2.0),
            item.y1.0.min(item.y2.0),
            item.y1.0.max(item.y2.0),
        ),
    };
    let (dx, dy) = constrained_delta(min_x, max_x, min_y, max_y, dx, dy, band_width, band_height);

    match item {
        Item::Text(item) => {
            item.x.0 += dx;
            item.y.0 += dy;
        }
        Item::Rectangle(item) => {
            item.x.0 += dx;
            item.y.0 += dy;
        }
        Item::Image(item) => {
            item.x.0 += dx;
            item.y.0 += dy;
        }
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => {
            item.x.0 += dx;
            item.y.0 += dy;
        }
        Item::Line(item) => {
            item.x1.0 += dx;
            item.y1.0 += dy;
            item.x2.0 += dx;
            item.y2.0 += dy;
        }
    }
}

pub(crate) fn resize_item(
    item: &mut Item,
    handle: ResizeHandle,
    dx: f32,
    dy: f32,
    band_width: f32,
    band_height: f32,
) {
    match item {
        Item::Text(item) => resize_rectangle(
            &mut item.x.0,
            &mut item.y.0,
            &mut item.width.0,
            &mut item.height.0,
            handle,
            dx,
            dy,
            band_width,
            band_height,
        ),
        Item::Rectangle(item) => resize_rectangle(
            &mut item.x.0,
            &mut item.y.0,
            &mut item.width.0,
            &mut item.height.0,
            handle,
            dx,
            dy,
            band_width,
            band_height,
        ),
        Item::Image(item) => resize_rectangle(
            &mut item.x.0,
            &mut item.y.0,
            &mut item.width.0,
            &mut item.height.0,
            handle,
            dx,
            dy,
            band_width,
            band_height,
        ),
        Item::HorizontalLayout(item) => {
            resize_layout_trailing_edge(item, true, handle, dx, dy, band_width, band_height)
        }
        Item::VerticalLayout(item) => {
            resize_layout_trailing_edge(item, false, handle, dx, dy, band_width, band_height)
        }
        Item::Line(item) => match handle {
            ResizeHandle::LineStart => {
                item.x1.0 = (item.x1.0 + dx).clamp(0.0, band_width);
                item.y1.0 = (item.y1.0 + dy).clamp(0.0, band_height);
            }
            ResizeHandle::LineEnd => {
                item.x2.0 = (item.x2.0 + dx).clamp(0.0, band_width);
                item.y2.0 = (item.y2.0 + dy).clamp(0.0, band_height);
            }
            _ => {}
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resize_layout_trailing_edge(
    layout: &mut LayoutItem,
    horizontal: bool,
    handle: ResizeHandle,
    dx: f32,
    dy: f32,
    band_width: f32,
    band_height: f32,
) {
    let old_width = layout.width.0;
    let old_height = layout.height.0;
    resize_rectangle(
        &mut layout.x.0,
        &mut layout.y.0,
        &mut layout.width.0,
        &mut layout.height.0,
        handle,
        dx,
        dy,
        band_width,
        band_height,
    );
    if horizontal
        && matches!(
            handle,
            ResizeHandle::TopRight | ResizeHandle::Right | ResizeHandle::BottomRight
        )
    {
        let Some(last) = layout.items.last_mut() else {
            return;
        };
        let geometry = normalized_geometry(last);
        let requested_delta = layout.width.0 - old_width;
        let actual_delta = requested_delta.max(MIN_ITEM_SIZE - geometry.2);
        layout.width.0 = old_width + actual_delta;
        set_item_frame(
            last,
            geometry.0,
            geometry.1,
            geometry.2 + actual_delta,
            geometry.3,
        );
        scale_layout_contents(
            last,
            geometry.2,
            geometry.3,
            geometry.2 + actual_delta,
            geometry.3,
        );
    } else if !horizontal
        && matches!(
            handle,
            ResizeHandle::BottomLeft | ResizeHandle::Bottom | ResizeHandle::BottomRight
        )
    {
        let Some(last) = layout.items.last_mut() else {
            return;
        };
        let geometry = normalized_geometry(last);
        let requested_delta = layout.height.0 - old_height;
        let actual_delta = requested_delta.max(MIN_ITEM_SIZE - geometry.3);
        layout.height.0 = old_height + actual_delta;
        set_item_frame(
            last,
            geometry.0,
            geometry.1,
            geometry.2,
            geometry.3 + actual_delta,
        );
        scale_layout_contents(
            last,
            geometry.2,
            geometry.3,
            geometry.2,
            geometry.3 + actual_delta,
        );
    }

    if horizontal
        && matches!(
            handle,
            ResizeHandle::TopLeft
                | ResizeHandle::Top
                | ResizeHandle::TopRight
                | ResizeHandle::BottomLeft
                | ResizeHandle::Bottom
                | ResizeHandle::BottomRight
        )
    {
        for child in &mut layout.items {
            let geometry = normalized_geometry(child);
            set_item_frame(child, geometry.0, 0.0, geometry.2, layout.height.0);
            scale_layout_contents(child, geometry.2, geometry.3, geometry.2, layout.height.0);
        }
    } else if !horizontal
        && matches!(
            handle,
            ResizeHandle::TopLeft
                | ResizeHandle::Left
                | ResizeHandle::BottomLeft
                | ResizeHandle::TopRight
                | ResizeHandle::Right
                | ResizeHandle::BottomRight
        )
    {
        for child in &mut layout.items {
            let geometry = normalized_geometry(child);
            set_item_frame(child, 0.0, geometry.1, layout.width.0, geometry.3);
            scale_layout_contents(child, geometry.2, geometry.3, layout.width.0, geometry.3);
        }
    }
}

pub(crate) fn reflow_layout(item: &mut Item) {
    match item {
        Item::HorizontalLayout(layout) => {
            arrange_layout_children(&mut layout.items, true, layout.width.0, layout.height.0);
        }
        Item::VerticalLayout(layout) => {
            arrange_layout_children(&mut layout.items, false, layout.width.0, layout.height.0);
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resize_rectangle(
    x: &mut f32,
    y: &mut f32,
    width: &mut f32,
    height: &mut f32,
    handle: ResizeHandle,
    dx: f32,
    dy: f32,
    band_width: f32,
    band_height: f32,
) {
    let right = *x + *width;
    let bottom = *y + *height;

    match handle {
        ResizeHandle::TopLeft | ResizeHandle::Left | ResizeHandle::BottomLeft => {
            let new_x = (*x + dx).clamp(0.0, right - MIN_ITEM_SIZE);
            *x = new_x;
            *width = right - new_x;
        }
        ResizeHandle::TopRight | ResizeHandle::Right | ResizeHandle::BottomRight => {
            *width = (right + dx).clamp(*x + MIN_ITEM_SIZE, band_width) - *x;
        }
        _ => {}
    }

    match handle {
        ResizeHandle::TopLeft | ResizeHandle::Top | ResizeHandle::TopRight => {
            let new_y = (*y + dy).clamp(0.0, bottom - MIN_ITEM_SIZE);
            *y = new_y;
            *height = bottom - new_y;
        }
        ResizeHandle::BottomLeft | ResizeHandle::Bottom | ResizeHandle::BottomRight => {
            *height = (bottom + dy).clamp(*y + MIN_ITEM_SIZE, band_height) - *y;
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn constrained_delta(
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    dx: f32,
    dy: f32,
    band_width: f32,
    band_height: f32,
) -> (f32, f32) {
    let item_width = max_x - min_x;
    let item_height = max_y - min_y;
    let target_x = (min_x + dx).clamp(0.0, (band_width - item_width).max(0.0));
    let target_y = (min_y + dy).clamp(0.0, (band_height - item_height).max(0.0));

    (target_x - min_x, target_y - min_y)
}

pub(crate) fn item_name(item: &Item) -> &str {
    match item {
        Item::Text(item) => non_empty_name(&item.name, "TextItem"),
        Item::Line(item) => non_empty_name(&item.name, "LineItem"),
        Item::Rectangle(item) => non_empty_name(&item.name, "ShapeItem"),
        Item::Image(item) => non_empty_name(&item.name, "ImageItem"),
        Item::HorizontalLayout(item) => non_empty_name(&item.name, "HorizontalLayout"),
        Item::VerticalLayout(item) => non_empty_name(&item.name, "VerticalLayout"),
    }
}

pub(crate) fn item_type_name(item: &Item) -> &'static str {
    match item {
        Item::Text(_) => "TextItem",
        Item::Line(_) => "LineItem",
        Item::Rectangle(_) => "ShapeItem",
        Item::Image(_) => "ImageItem",
        Item::HorizontalLayout(_) => "HorizontalLayout",
        Item::VerticalLayout(_) => "VerticalLayout",
    }
}

pub(crate) fn non_empty_name<'a>(name: &'a str, fallback: &'static str) -> &'a str {
    if name.is_empty() { fallback } else { name }
}

pub(crate) fn geometry_field_specs(item: &Item) -> [(&'static str, GeometryField); 4] {
    match item {
        Item::Line(_) => [
            ("X1", GeometryField::X1),
            ("Y1", GeometryField::Y1),
            ("X2", GeometryField::X2),
            ("Y2", GeometryField::Y2),
        ],
        _ => [
            ("X", GeometryField::X),
            ("Y", GeometryField::Y),
            ("Width", GeometryField::Width),
            ("Height", GeometryField::Height),
        ],
    }
}

pub(crate) fn geometry_values(item: &Item) -> (f32, f32, f32, f32) {
    match item {
        Item::Text(item) => (item.x.0, item.y.0, item.width.0, item.height.0),
        Item::Rectangle(item) => (item.x.0, item.y.0, item.width.0, item.height.0),
        Item::Image(item) => (item.x.0, item.y.0, item.width.0, item.height.0),
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => {
            (item.x.0, item.y.0, item.width.0, item.height.0)
        }
        Item::Line(item) => (item.x1.0, item.y1.0, item.x2.0, item.y2.0),
    }
}

pub(crate) fn item_geometry_value(item: &Item, field: GeometryField) -> Option<f32> {
    match (item, field) {
        (Item::Text(item), GeometryField::X) => Some(item.x.0),
        (Item::Text(item), GeometryField::Y) => Some(item.y.0),
        (Item::Text(item), GeometryField::Width) => Some(item.width.0),
        (Item::Text(item), GeometryField::Height) => Some(item.height.0),
        (Item::Rectangle(item), GeometryField::X) => Some(item.x.0),
        (Item::Rectangle(item), GeometryField::Y) => Some(item.y.0),
        (Item::Rectangle(item), GeometryField::Width) => Some(item.width.0),
        (Item::Rectangle(item), GeometryField::Height) => Some(item.height.0),
        (Item::Image(item), GeometryField::X) => Some(item.x.0),
        (Item::Image(item), GeometryField::Y) => Some(item.y.0),
        (Item::Image(item), GeometryField::Width) => Some(item.width.0),
        (Item::Image(item), GeometryField::Height) => Some(item.height.0),
        (Item::HorizontalLayout(item), GeometryField::X)
        | (Item::VerticalLayout(item), GeometryField::X) => Some(item.x.0),
        (Item::HorizontalLayout(item), GeometryField::Y)
        | (Item::VerticalLayout(item), GeometryField::Y) => Some(item.y.0),
        (Item::HorizontalLayout(item), GeometryField::Width)
        | (Item::VerticalLayout(item), GeometryField::Width) => Some(item.width.0),
        (Item::HorizontalLayout(item), GeometryField::Height)
        | (Item::VerticalLayout(item), GeometryField::Height) => Some(item.height.0),
        (Item::Line(item), GeometryField::X1) => Some(item.x1.0),
        (Item::Line(item), GeometryField::Y1) => Some(item.y1.0),
        (Item::Line(item), GeometryField::X2) => Some(item.x2.0),
        (Item::Line(item), GeometryField::Y2) => Some(item.y2.0),
        _ => None,
    }
}

pub(crate) fn format_mm(value: f32) -> String {
    format!("{value:.2}")
}

pub(crate) fn format_pt(value: f32) -> String {
    format!("{value:.2}")
}

pub(crate) fn set_item_geometry(
    item: &mut Item,
    field: GeometryField,
    value: f32,
    band_width: f32,
    band_height: f32,
) -> bool {
    match item {
        Item::Text(item) => {
            return set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
        }
        Item::HorizontalLayout(item) => {
            if field == GeometryField::Width {
                let delta = value - item.width.0;
                resize_layout_trailing_edge(
                    item,
                    true,
                    ResizeHandle::Right,
                    delta,
                    0.0,
                    band_width,
                    band_height,
                );
                return delta.abs() > f32::EPSILON;
            }
            if field == GeometryField::Height {
                let delta = value - item.height.0;
                resize_layout_trailing_edge(
                    item,
                    true,
                    ResizeHandle::Bottom,
                    0.0,
                    delta,
                    band_width,
                    band_height,
                );
                return delta.abs() > f32::EPSILON;
            }
            let changed = set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
            return changed;
        }
        Item::VerticalLayout(item) => {
            if field == GeometryField::Width {
                let delta = value - item.width.0;
                resize_layout_trailing_edge(
                    item,
                    false,
                    ResizeHandle::Right,
                    delta,
                    0.0,
                    band_width,
                    band_height,
                );
                return delta.abs() > f32::EPSILON;
            }
            if field == GeometryField::Height {
                let delta = value - item.height.0;
                resize_layout_trailing_edge(
                    item,
                    false,
                    ResizeHandle::Bottom,
                    0.0,
                    delta,
                    band_width,
                    band_height,
                );
                return delta.abs() > f32::EPSILON;
            }
            let changed = set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
            return changed;
        }
        Item::Rectangle(item) => {
            return set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
        }
        Item::Image(item) => {
            return set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
        }
        Item::Line(item) => match field {
            GeometryField::X1 => item.x1.0 = value.clamp(0.0, band_width),
            GeometryField::Y1 => item.y1.0 = value.clamp(0.0, band_height),
            GeometryField::X2 => item.x2.0 = value.clamp(0.0, band_width),
            GeometryField::Y2 => item.y2.0 = value.clamp(0.0, band_height),
            _ => return false,
        },
    }

    true
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn set_rectangle_geometry(
    x: &mut f32,
    y: &mut f32,
    width: &mut f32,
    height: &mut f32,
    field: GeometryField,
    value: f32,
    band_width: f32,
    band_height: f32,
) -> bool {
    match field {
        GeometryField::X => *x = value.clamp(0.0, (band_width - *width).max(0.0)),
        GeometryField::Y => *y = value.clamp(0.0, (band_height - *height).max(0.0)),
        GeometryField::Width => {
            *width = value.clamp(MIN_ITEM_SIZE, (band_width - *x).max(MIN_ITEM_SIZE))
        }
        GeometryField::Height => {
            *height = value.clamp(MIN_ITEM_SIZE, (band_height - *y).max(MIN_ITEM_SIZE))
        }
        _ => return false,
    }

    true
}
