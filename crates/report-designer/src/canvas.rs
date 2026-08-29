use super::*;

#[path = "canvas/drawing.rs"]
mod drawing;
#[path = "canvas/hit_test.rs"]
mod hit_test;
#[path = "canvas/interaction.rs"]
mod interaction;

#[cfg(test)]
pub(crate) use drawing::selected_descendant_path;
use drawing::*;
#[cfg(test)]
pub(crate) use hit_test::hit_test_item;

pub(crate) struct DesignerCanvas<'a> {
    pub(crate) page: &'a Page,
    pub(crate) selection: Option<Selection>,
    pub(crate) selected_items: &'a [Selection],
    pub(crate) active_band: Option<usize>,
    pub(crate) scale: f32,
    pub(crate) font_names: &'a HashMap<String, &'static str>,
    pub(crate) guides_visible: bool,
    pub(crate) images: &'a HashMap<String, DesignerImage>,
}

pub(crate) struct ColorWheel {
    pub(crate) selected: ReportColor,
}

pub(crate) struct PropertiesResizer;

#[derive(Default)]
pub(crate) struct ResizerState {
    dragging: bool,
    last_x: Option<f32>,
}

impl canvas::Program<Message> for PropertiesResizer {
    type State = ResizerState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                state.dragging = true;
                state.last_x = cursor.position().map(|position| position.x);
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                let x = cursor.position()?.x;
                let previous = state.last_x.replace(x)?;
                Some(canvas::Action::publish(Message::ResizeProperties(x - previous)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.dragging =>
            {
                state.dragging = false;
                state.last_x = None;
                Some(canvas::Action::capture())
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
        frame.fill(
            &Path::rectangle(
                Point::new(bounds.width / 2.0 - 1.0, 0.0),
                Size::new(2.0, bounds.height),
            ),
            Color::from_rgb8(95, 100, 110),
        );
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.dragging || cursor.is_over(bounds) {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::default()
        }
    }
}

impl canvas::Program<Message> for ColorWheel {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let position = cursor.position_in(bounds)?;
            let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
            let dx = position.x - center.x;
            let dy = position.y - center.y;
            let radius = bounds.width.min(bounds.height) / 2.0 - 8.0;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance <= radius {
                let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                let saturation = (distance / radius).clamp(0.0, 1.0);
                return Some(
                    canvas::Action::publish(Message::TextColorSelected(hsv_to_report_color(
                        hue, saturation, 1.0,
                    )))
                    .and_capture(),
                );
            }
        }

        None
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
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 8.0;
        let rings = 14;
        let slices = 72;

        for ring in 0..rings {
            let inner = radius * ring as f32 / rings as f32;
            let outer = radius * (ring + 1) as f32 / rings as f32;
            let saturation = (ring as f32 + 0.5) / rings as f32;
            for slice in 0..slices {
                let a0 = std::f32::consts::TAU * slice as f32 / slices as f32;
                let a1 = std::f32::consts::TAU * (slice + 1) as f32 / slices as f32;
                let path = Path::new(|builder| {
                    builder.move_to(polar_point(center, inner, a0));
                    builder.line_to(polar_point(center, outer, a0));
                    builder.line_to(polar_point(center, outer, a1));
                    builder.line_to(polar_point(center, inner, a1));
                    builder.close();
                });
                let hue = 360.0 * (slice as f32 + 0.5) / slices as f32;
                frame.fill(
                    &path,
                    report_color_to_iced(hsv_to_report_color(hue, saturation, 1.0)),
                );
            }
        }

        frame.stroke(
            &Path::circle(center, radius),
            canvas::Stroke {
                width: 1.0,
                style: canvas::Style::Solid(Color::from_rgb8(90, 95, 105)),
                ..Default::default()
            },
        );

        let (hue, saturation, _) = report_color_to_hsv(self.selected);
        let angle = hue.to_radians();
        let marker = polar_point(center, radius * saturation, angle);
        frame.fill(
            &Path::circle(marker, 5.0),
            report_color_to_iced(self.selected),
        );
        frame.stroke(
            &Path::circle(marker, 6.0),
            canvas::Stroke {
                width: 2.0,
                style: canvas::Style::Solid(Color::from_rgb8(35, 38, 45)),
                ..Default::default()
            },
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        cursor
            .position_in(bounds)
            .map(|_| mouse::Interaction::Pointer)
            .unwrap_or_default()
    }
}
