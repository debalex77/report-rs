use super::*;

pub(super) fn draw_item(
    frame: &mut Frame<Renderer>,
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    font_names: &HashMap<String, &'static str>,
    images: &HashMap<String, DesignerImage>,
    selected: bool,
    selected_path: Option<&[usize]>,
    draw_handles: bool,
    nested_in_horizontal: bool,
) {
    let color = if selected {
        Color::from_rgb8(225, 80, 55)
    } else {
        Color::from_rgb8(30, 140, 155)
    };
    let stroke = canvas::Stroke {
        width: if selected { 2.0 } else { 1.0 },
        style: canvas::Style::Solid(color),
        ..Default::default()
    };

    match item {
        Item::Line(line) => frame.stroke(
            &Path::line(
                Point::new(offset_x + line.x1.0 * scale, offset_y + line.y1.0 * scale),
                Point::new(offset_x + line.x2.0 * scale, offset_y + line.y2.0 * scale),
            ),
            stroke,
        ),
        Item::Text(text_item) => {
            if let Some(mut rect) = item_bounds(item, offset_x, offset_y, scale) {
                rect.height = text_display_height(text_item) * scale;
                let path = Path::rectangle(rect.position(), rect.size());
                if let Some(background) = text_item.background {
                    frame.fill(&path, report_color_to_iced(background));
                }
                frame.fill(&path, Color::from_rgba8(30, 140, 155, 0.06));
                if let Some(border) = &text_item.border {
                    let border_stroke = canvas::Stroke {
                        width: (border.width * scale).max(1.0),
                        style: canvas::Style::Solid(Color::BLACK),
                        ..Default::default()
                    };
                    let left = rect.x;
                    let top = rect.y;
                    let right = rect.x + rect.width;
                    let bottom = rect.y + rect.height;
                    if border.left {
                        frame.stroke(
                            &Path::line(Point::new(left, top), Point::new(left, bottom)),
                            border_stroke,
                        );
                    }
                    if border.top {
                        frame.stroke(
                            &Path::line(Point::new(left, top), Point::new(right, top)),
                            border_stroke,
                        );
                    }
                    if border.right {
                        frame.stroke(
                            &Path::line(Point::new(right, top), Point::new(right, bottom)),
                            border_stroke,
                        );
                    }
                    if border.bottom {
                        frame.stroke(
                            &Path::line(Point::new(left, bottom), Point::new(right, bottom)),
                            border_stroke,
                        );
                    }
                }
                frame.stroke(&path, stroke);

                if rect.width >= 50.0 && rect.height >= 14.0 {
                    let (text_x, align_x) = match text_item.horizontal_align {
                        HorizontalAlign::Left => (
                            rect.x + text_item.padding.left.0 * scale,
                            iced::alignment::Horizontal::Left.into(),
                        ),
                        HorizontalAlign::Center => (
                            rect.x + rect.width / 2.0,
                            iced::alignment::Horizontal::Center.into(),
                        ),
                        HorizontalAlign::Right => (
                            rect.x + rect.width - text_item.padding.right.0 * scale,
                            iced::alignment::Horizontal::Right.into(),
                        ),
                    };
                    let (text_y, align_y) = match text_item.vertical_align {
                        VerticalAlign::Top => (
                            rect.y + text_item.padding.top.0 * scale,
                            iced::alignment::Vertical::Top,
                        ),
                        VerticalAlign::Center => (
                            rect.y + rect.height / 2.0,
                            iced::alignment::Vertical::Center,
                        ),
                        VerticalAlign::Bottom => (
                            rect.y + rect.height - text_item.padding.bottom.0 * scale,
                            iced::alignment::Vertical::Bottom,
                        ),
                    };
                    let layout = designer_text_layout(text_item);
                    let content = layout
                        .lines
                        .iter()
                        .map(|line| line.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    frame.fill_text(canvas::Text {
                        content,
                        position: Point::new(text_x, text_y),
                        // Wrapping is already performed by designer_text_layout.
                        // A finite max_width would make iced wrap the generated
                        // lines again using different font metrics, breaking the
                        // AutoHeight calculation and the following band offsets.
                        max_width: f32::INFINITY,
                        color: report_color_to_iced(text_item.text_color),
                        size: iced::Pixels(
                            text_item.font_size * (96.0 / 72.0) * (scale / BASE_SCALE),
                        ),
                        font: designer_font(text_item, font_names),
                        align_x,
                        align_y,
                        ..Default::default()
                    });
                }
            }
        }
        Item::Image(image_item) => {
            if let Some(rect) = item_bounds(item, offset_x, offset_y, scale) {
                if let Some(image) = images.get(&image_item.source) {
                    let placement = calculate_image_placement(
                        image_item.x,
                        image_item.y,
                        image_item.width,
                        image_item.height,
                        image.width,
                        image.height,
                        image_item.fit,
                    );
                    frame.draw_image(
                        Rectangle::new(
                            Point::new(
                                offset_x + placement.x.0 * scale,
                                offset_y + placement.y.0 * scale,
                            ),
                            Size::new(placement.width.0 * scale, placement.height.0 * scale),
                        ),
                        &image.handle,
                    );
                } else {
                    frame.stroke(
                        &Path::line(
                            rect.position(),
                            Point::new(rect.x + rect.width, rect.y + rect.height),
                        ),
                        stroke,
                    );
                    frame.stroke(
                        &Path::line(
                            Point::new(rect.x + rect.width, rect.y),
                            Point::new(rect.x, rect.y + rect.height),
                        ),
                        stroke,
                    );
                }
                frame.stroke(&Path::rectangle(rect.position(), rect.size()), stroke);
            }
        }
        Item::Rectangle(rectangle_item) => {
            if let Some(rect) = item_bounds(item, offset_x, offset_y, scale) {
                frame.stroke(
                    &Path::rectangle(rect.position(), rect.size()),
                    canvas::Stroke {
                        width: (rectangle_item.border_width.0 * scale).max(1.0),
                        style: canvas::Style::Solid(Color::BLACK),
                        ..Default::default()
                    },
                );
                if selected {
                    frame.stroke(&Path::rectangle(rect.position(), rect.size()), stroke);
                }
            }
        }
        Item::HorizontalLayout(_) | Item::VerticalLayout(_) => {
            if let Some(rect) = item_bounds(item, offset_x, offset_y, scale) {
                frame.stroke(&Path::rectangle(rect.position(), rect.size()), stroke);
                if let Some(label_bounds) = layout_label_bounds(item, rect, nested_in_horizontal) {
                    let label_background = if selected {
                        Color::from_rgba8(225, 80, 55, 0.24)
                    } else {
                        Color::from_rgba8(30, 140, 155, 0.16)
                    };
                    let label_path = Path::rounded_rectangle(
                        label_bounds.position(),
                        label_bounds.size(),
                        iced::border::radius(4),
                    );
                    frame.fill(&label_path, label_background);
                    frame.stroke(&label_path, stroke);
                    frame.fill_text(canvas::Text {
                        content: item_name(item).to_string(),
                        position: Point::new(label_bounds.x + 6.0, label_bounds.y + 9.0),
                        color,
                        size: iced::Pixels(10.0),
                        align_y: iced::alignment::Vertical::Center,
                        ..Default::default()
                    });
                }
                let children = match item {
                    Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => &layout.items,
                    _ => unreachable!(),
                };
                let children_are_in_horizontal = matches!(item, Item::HorizontalLayout(_));
                for (index, child) in children.iter().enumerate() {
                    draw_item(
                        frame,
                        child,
                        rect.x,
                        rect.y,
                        scale,
                        font_names,
                        images,
                        selected_path
                            .is_some_and(|path| path.first() == Some(&index) && path.len() == 1),
                        selected_path.and_then(|path| selected_descendant_path(path, index)),
                        false,
                        children_are_in_horizontal,
                    );
                }
                if selected {
                    draw_layout_dividers(frame, item, rect, scale);
                }
            }
        }
    }

    if selected && draw_handles {
        draw_resize_handles(frame, item, offset_x, offset_y, scale);
    }
}

pub(crate) fn selected_descendant_path(path: &[usize], child_index: usize) -> Option<&[usize]> {
    match path {
        [selected_child, descendants @ ..]
            if *selected_child == child_index && !descendants.is_empty() =>
        {
            Some(descendants)
        }
        _ => None,
    }
}

fn draw_layout_dividers(frame: &mut Frame<Renderer>, item: &Item, bounds: Rectangle, scale: f32) {
    let (layout, horizontal) = match item {
        Item::HorizontalLayout(layout) => (layout, true),
        Item::VerticalLayout(layout) => (layout, false),
        _ => return,
    };
    for child in layout
        .items
        .iter()
        .take(layout.items.len().saturating_sub(1))
    {
        let child = normalized_geometry(child);
        let center = if horizontal {
            Point::new(
                bounds.x + (child.0 + child.2) * scale,
                bounds.y + bounds.height / 2.0,
            )
        } else {
            Point::new(
                bounds.x + bounds.width / 2.0,
                bounds.y + (child.1 + child.3) * scale,
            )
        };
        let size = if horizontal {
            Size::new(6.0, 24.0_f32.min(bounds.height))
        } else {
            Size::new(24.0_f32.min(bounds.width), 6.0)
        };
        frame.fill(
            &Path::rounded_rectangle(
                Point::new(center.x - size.width / 2.0, center.y - size.height / 2.0),
                size,
                iced::border::radius(3),
            ),
            Color::from_rgb8(225, 80, 55),
        );
    }
}

fn draw_resize_handles(
    frame: &mut Frame<Renderer>,
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
) {
    for (point, _) in resize_handle_points(item, offset_x, offset_y, scale) {
        let bounds = handle_bounds(point);
        let path = Path::rectangle(bounds.position(), bounds.size());
        frame.fill(&path, Color::WHITE);
        frame.stroke(
            &path,
            canvas::Stroke {
                width: 1.5,
                style: canvas::Style::Solid(Color::from_rgb8(225, 80, 55)),
                ..Default::default()
            },
        );
    }
}

pub(super) fn resize_handle_points(
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
) -> Vec<(Point, ResizeHandle)> {
    match item {
        Item::Line(line) => vec![
            (
                Point::new(offset_x + line.x1.0 * scale, offset_y + line.y1.0 * scale),
                ResizeHandle::LineStart,
            ),
            (
                Point::new(offset_x + line.x2.0 * scale, offset_y + line.y2.0 * scale),
                ResizeHandle::LineEnd,
            ),
        ],
        _ => item_bounds(item, offset_x, offset_y, scale)
            .map(|bounds| {
                vec![
                    (bounds.position(), ResizeHandle::TopLeft),
                    (
                        Point::new(bounds.x + bounds.width / 2.0, bounds.y),
                        ResizeHandle::Top,
                    ),
                    (
                        Point::new(bounds.x + bounds.width, bounds.y),
                        ResizeHandle::TopRight,
                    ),
                    (
                        Point::new(bounds.x, bounds.y + bounds.height / 2.0),
                        ResizeHandle::Left,
                    ),
                    (
                        Point::new(bounds.x + bounds.width, bounds.y + bounds.height / 2.0),
                        ResizeHandle::Right,
                    ),
                    (
                        Point::new(bounds.x, bounds.y + bounds.height),
                        ResizeHandle::BottomLeft,
                    ),
                    (
                        Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height),
                        ResizeHandle::Bottom,
                    ),
                    (
                        Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
                        ResizeHandle::BottomRight,
                    ),
                ]
            })
            .unwrap_or_default(),
    }
}

pub(super) fn handle_bounds(center: Point) -> Rectangle {
    Rectangle::new(
        Point::new(center.x - HANDLE_SIZE / 2.0, center.y - HANDLE_SIZE / 2.0),
        Size::new(HANDLE_SIZE, HANDLE_SIZE),
    )
}

pub(super) fn draw_page_grid(
    frame: &mut Frame<Renderer>,
    page: &Page,
    page_origin: Point,
    scale: f32,
) {
    let page_width = page.width().0 * scale;
    let page_height = page.height().0 * scale;

    for millimeter in 0..=page.width().0.floor() as u32 {
        let x = page_origin.x + millimeter as f32 * scale;
        frame.stroke(
            &Path::line(
                Point::new(x, page_origin.y),
                Point::new(x, page_origin.y + page_height),
            ),
            grid_stroke(millimeter),
        );
    }

    for millimeter in 0..=page.height().0.floor() as u32 {
        let y = page_origin.y + millimeter as f32 * scale;
        frame.stroke(
            &Path::line(
                Point::new(page_origin.x, y),
                Point::new(page_origin.x + page_width, y),
            ),
            grid_stroke(millimeter),
        );
    }
}

fn grid_stroke(millimeter: u32) -> canvas::Stroke<'static> {
    let (width, color) = if millimeter % 10 == 0 {
        (1.0, Color::from_rgba8(70, 85, 100, 0.18))
    } else if millimeter % 5 == 0 {
        (1.0, Color::from_rgba8(70, 85, 100, 0.10))
    } else {
        (0.5, Color::from_rgba8(70, 85, 100, 0.05))
    };

    canvas::Stroke {
        width,
        style: canvas::Style::Solid(color),
        ..Default::default()
    }
}

pub(super) fn draw_rulers(
    frame: &mut Frame<Renderer>,
    page: &Page,
    page_origin: Point,
    page_size: Size,
    scale: f32,
    selected_bounds: Option<Rectangle>,
) {
    let ruler_fill = Color::from_rgb8(224, 226, 230);
    let ruler_line = Color::from_rgb8(75, 80, 88);
    let horizontal_y = page_origin.y - RULER_GAP - RULER_SIZE;
    let vertical_x = page_origin.x - RULER_GAP - RULER_SIZE;

    frame.fill(
        &Path::rectangle(
            Point::new(page_origin.x, horizontal_y),
            Size::new(page_size.width, RULER_SIZE),
        ),
        ruler_fill,
    );
    frame.fill(
        &Path::rectangle(
            Point::new(vertical_x, page_origin.y),
            Size::new(RULER_SIZE, page_size.height),
        ),
        ruler_fill,
    );

    if let Some(bounds) = selected_bounds {
        let shadow = Color::from_rgba8(225, 80, 55, 0.32);
        frame.fill(
            &Path::rectangle(
                Point::new(bounds.x, horizontal_y),
                Size::new(bounds.width, RULER_SIZE),
            ),
            shadow,
        );
        frame.fill(
            &Path::rectangle(
                Point::new(vertical_x, bounds.y),
                Size::new(RULER_SIZE, bounds.height),
            ),
            shadow,
        );
    }

    let tick_stroke = canvas::Stroke {
        width: 1.0,
        style: canvas::Style::Solid(ruler_line),
        ..Default::default()
    };

    for millimeter in 0..=page.width().0.floor() as u32 {
        let x = page_origin.x + millimeter as f32 * scale;
        let tick_height = ruler_tick_length(millimeter);
        frame.stroke(
            &Path::line(
                Point::new(x, page_origin.y - RULER_GAP),
                Point::new(x, page_origin.y - RULER_GAP - tick_height),
            ),
            tick_stroke,
        );
        if millimeter % 10 == 0 {
            frame.fill_text(canvas::Text {
                content: millimeter.to_string(),
                position: Point::new(x + 2.0, horizontal_y + 2.0),
                color: ruler_line,
                size: iced::Pixels(9.0),
                ..Default::default()
            });
        }
    }

    for millimeter in 0..=page.height().0.floor() as u32 {
        let y = page_origin.y + millimeter as f32 * scale;
        let tick_width = ruler_tick_length(millimeter);
        frame.stroke(
            &Path::line(
                Point::new(page_origin.x - RULER_GAP, y),
                Point::new(page_origin.x - RULER_GAP - tick_width, y),
            ),
            tick_stroke,
        );
        if millimeter % 10 == 0 {
            frame.fill_text(canvas::Text {
                content: millimeter.to_string(),
                position: Point::new(vertical_x + 2.0, y + 3.0),
                color: ruler_line,
                size: iced::Pixels(9.0),
                ..Default::default()
            });
        }
    }
}

pub(crate) fn selection_bounds(page: &Page, selection: Selection, scale: f32) -> Option<Rectangle> {
    let content_x = PAGE_MARGIN + page.margins.left.0 * scale;
    let mut band_y = PAGE_MARGIN + page.margins.top.0 * scale;

    for (band_index, band) in page.bands.iter().enumerate() {
        if band_index == selection.band {
            let mut item = band.items.get(selection.top_index())?;
            let mut offset_x = content_x;
            let mut offset_y = band_y;
            for &index in selection.descendants() {
                let layout = item_layout(item)?;
                offset_x += layout.x.0 * scale;
                offset_y += layout.y.0 * scale;
                item = layout.items.get(index)?;
            }
            return item_bounds(item, offset_x, offset_y, scale);
        }
        band_y += band.height.0 * scale;
    }

    None
}

fn ruler_tick_length(millimeter: u32) -> f32 {
    if millimeter % 10 == 0 {
        10.0
    } else if millimeter % 5 == 0 {
        7.0
    } else {
        4.0
    }
}

pub(crate) fn item_bounds(
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
) -> Option<Rectangle> {
    let rect = match item {
        Item::Text(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, text_display_height(item) * scale),
        ),
        Item::Rectangle(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::Image(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::Line(item) => {
            let x1 = offset_x + item.x1.0 * scale;
            let y1 = offset_y + item.y1.0 * scale;
            let x2 = offset_x + item.x2.0 * scale;
            let y2 = offset_y + item.y2.0 * scale;
            Rectangle::new(
                Point::new(x1.min(x2) - 4.0, y1.min(y2) - 4.0),
                Size::new((x2 - x1).abs() + 8.0, (y2 - y1).abs() + 8.0),
            )
        }
    };

    Some(rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_auto_height_text_expands_designer_bounds() {
        let mut item = new_text_item("DejaVu Sans".to_string());
        let Item::Text(text) = &mut item else {
            unreachable!();
        };
        text.width = Mm(30.0);
        text.height = Mm(5.0);
        text.text = "Responsabil_sdfhjbksdfhhsgdffiaghfkhgqashg".to_string();
        text.word_wrap = true;
        text.auto_height = true;

        let expected_height = {
            let layout = designer_text_layout(text);
            assert!(layout.lines.len() > 1);
            let height = text_display_height(text);
            assert!(height > text.height.0);
            height
        };

        let bounds = item_bounds(&item, 0.0, 0.0, 1.0).expect("text has bounds");
        assert_eq!(bounds.height, expected_height);
    }
}
