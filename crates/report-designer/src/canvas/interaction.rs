use super::*;

impl canvas::Program<Message> for DesignerCanvas<'_> {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let local_position = cursor.position_in(bounds)?;
                let position = cursor.position()?;
                let selection = self.hit_test(local_position);
                let band = selection
                    .map(|selection| selection.band)
                    .or_else(|| self.band_hit_test(local_position));
                state.dragging = None;
                state.last_position = None;
                Some(
                    canvas::Action::publish(Message::OpenContextMenu {
                        selection,
                        band,
                        position,
                    })
                    .and_capture(),
                )
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                let (selection, band, operation) = if let Some((selection, divider, horizontal)) =
                    self.layout_divider_hit_test(position)
                {
                    (
                        Some(selection),
                        Some(selection.band),
                        Some(DragOperation::ResizeLayoutDivider(
                            selection, divider, horizontal,
                        )),
                    )
                } else if let Some(band) = self.band_resize_hit_test(position) {
                    (None, Some(band), Some(DragOperation::ResizeBand(band)))
                } else if let Some((selection, handle)) = self.resize_hit_test(position) {
                    (
                        Some(selection),
                        Some(selection.band),
                        Some(DragOperation::Resize(selection, handle)),
                    )
                } else {
                    let selection = self.hit_test(position);
                    let band = selection
                        .map(|selection| selection.band)
                        .or_else(|| self.band_hit_test(position));
                    let operation = selection
                        .filter(|selection| selection.is_top_level())
                        .map(DragOperation::Move);
                    (selection, band, operation)
                };
                state.dragging = operation;
                state.last_position = operation.map(|_| position);

                Some(
                    canvas::Action::publish(Message::BeginCanvasInteraction {
                        selection,
                        band,
                        additive: state.modifiers.control(),
                    })
                    .and_capture(),
                )
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let operation = state.dragging?;
                let position = cursor.position_in(bounds)?;
                let previous = state.last_position.replace(position)?;
                let dx = (position.x - previous.x) / self.scale;
                let dy = (position.y - previous.y) / self.scale;
                let message = match operation {
                    DragOperation::Move(selection) => Message::MoveItem { selection, dx, dy },
                    DragOperation::Resize(selection, handle) => Message::ResizeItem {
                        selection,
                        handle,
                        dx,
                        dy,
                    },
                    DragOperation::ResizeBand(band) => Message::ResizeBand { band, dy },
                    DragOperation::ResizeLayoutDivider(selection, divider, horizontal) => {
                        Message::ResizeLayoutDivider {
                            selection,
                            divider,
                            horizontal,
                            delta: if horizontal { dx } else { dy },
                        }
                    }
                };

                Some(canvas::Action::publish(message).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.data_field_drag {
                    state.dragging = None;
                    state.last_position = None;
                    let message = cursor
                        .position_in(bounds)
                        .and_then(|position| self.band_hit_test(position))
                        .map(Message::DropDataFields)
                        .unwrap_or(Message::CancelDataFieldDrag);
                    Some(canvas::Action::publish(message).and_capture())
                } else if state.dragging.take().is_some() {
                    state.last_position = None;
                    Some(canvas::Action::publish(Message::EndCanvasInteraction).and_capture())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let page_origin = Point::new(PAGE_MARGIN, PAGE_MARGIN);
        let page_size = Size::new(
            self.page.width().0 * self.scale,
            self.page.height().0 * self.scale,
        );
        let page_rect = Path::rectangle(page_origin, page_size);

        frame.fill(&page_rect, Color::WHITE);
        frame.stroke(
            &page_rect,
            canvas::Stroke {
                width: 1.0,
                style: canvas::Style::Solid(Color::from_rgb8(90, 95, 105)),
                ..Default::default()
            },
        );
        if self.guides_visible {
            draw_page_grid(&mut frame, self.page, page_origin, self.scale);

            let selected_bounds = self
                .selection
                .and_then(|selection| selection_bounds(self.page, selection, self.scale));
            draw_rulers(
                &mut frame,
                self.page,
                page_origin,
                page_size,
                self.scale,
                selected_bounds,
            );
        }

        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let content_width = self.page.printable_width().0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;

        for (band_index, band) in self.page.bands.iter().enumerate() {
            let band_height = band.height.0 * self.scale;
            let (band_fill, band_border) = band_colors(&band.kind);
            let band_rect = Path::rectangle(
                Point::new(content_x, band_y),
                Size::new(content_width, band_height),
            );
            frame.fill(&band_rect, band_fill);
            frame.stroke(
                &band_rect,
                canvas::Stroke {
                    width: if self.active_band == Some(band_index) {
                        2.0
                    } else {
                        1.0
                    },
                    style: canvas::Style::Solid(band_border),
                    ..Default::default()
                },
            );
            frame.fill(
                &Path::rectangle(Point::new(content_x, band_y), Size::new(3.0, band_height)),
                band_border,
            );
            frame.fill_text(canvas::Text {
                content: band_name(&band.kind).to_string(),
                position: Point::new(content_x + content_width - 5.0, band_y + 10.0),
                color: Color {
                    a: 0.55,
                    ..band_border
                },
                size: iced::Pixels(10.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                ..Default::default()
            });
            let badge = Path::rectangle(
                Point::new(
                    PAGE_MARGIN - RULER_SIZE - RULER_GAP - BAND_BADGE_WIDTH - 8.0,
                    band_y,
                ),
                Size::new(BAND_BADGE_WIDTH, band_height),
            );
            frame.fill(&badge, band_border);
            frame.fill_text(canvas::Text {
                content: band_name(&band.kind).to_string(),
                position: Point::new(
                    PAGE_MARGIN - RULER_SIZE - RULER_GAP - BAND_BADGE_WIDTH - 3.0,
                    band_y + band_height / 2.0,
                ),
                color: Color::WHITE,
                size: iced::Pixels(10.0),
                align_y: iced::alignment::Vertical::Center,
                ..Default::default()
            });

            for (item_index, item) in band.items.iter().enumerate() {
                let top_level = Selection::top_level(band_index, item_index);
                let selected_path = self
                    .selected_items
                    .iter()
                    .find(|selection| {
                        selection.band == band_index && selection.top_index() == item_index
                    })
                    .map(Selection::descendants);
                draw_item(
                    &mut frame,
                    item,
                    content_x,
                    band_y,
                    self.scale,
                    self.font_names,
                    self.images,
                    self.selected_items.contains(&top_level),
                    selected_path,
                    true,
                    false,
                );
            }

            if self.active_band == Some(band_index) {
                let handle = Path::rounded_rectangle(
                    Point::new(
                        content_x + content_width / 2.0 - 14.0,
                        band_y + band_height - 3.0,
                    ),
                    Size::new(28.0, 6.0),
                    iced::border::radius(3),
                );
                frame.fill(&handle, Color::from_rgb8(225, 80, 55));
            }

            band_y += band_height;
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(position) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };
        if self.band_resize_hit_test(position).is_some() {
            mouse::Interaction::ResizingVertically
        } else if let Some((_, _, horizontal)) = self.layout_divider_hit_test(position) {
            if horizontal {
                mouse::Interaction::ResizingHorizontally
            } else {
                mouse::Interaction::ResizingVertically
            }
        } else if self.resize_hit_test(position).is_some()
            || self.hit_test(position).is_some()
            || self.band_hit_test(position).is_some()
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

#[derive(Default)]
pub(crate) struct CanvasState {
    dragging: Option<DragOperation>,
    last_position: Option<Point>,
    modifiers: keyboard::Modifiers,
}
