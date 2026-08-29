use std::path::PathBuf;

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Task, Theme};

use report_core::common;
use report_core::model::{BandKind, Item, Page, Report};

// CSS pixels per millimeter at the standard 96 DPI screen scale.
const BASE_SCALE: f32 = 96.0 / 25.4;
const PAGE_MARGIN: f32 = 130.0;
const RULER_SIZE: f32 = 24.0;
const RULER_GAP: f32 = 4.0;
const BAND_BADGE_WIDTH: f32 = 74.0;
const INSPECTOR_WIDTH: f32 = 280.0;
const HANDLE_SIZE: f32 = 8.0;
const MIN_ITEM_SIZE: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Selection {
    band: usize,
    item: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
    LineStart,
    LineEnd,
}

#[derive(Debug, Clone, Copy)]
enum DragOperation {
    Move(Selection),
    Resize(Selection, ResizeHandle),
}

#[derive(Debug, Clone)]
enum Message {
    Reload,
    Save,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Select(Option<Selection>),
    MoveItem {
        selection: Selection,
        dx: f32,
        dy: f32,
    },
    ResizeItem {
        selection: Selection,
        handle: ResizeHandle,
        dx: f32,
        dy: f32,
    },
}

struct DesignerApp {
    path: PathBuf,
    report: Report,
    selection: Option<Selection>,
    status: String,
    zoom: f32,
    dirty: bool,
}

impl Default for DesignerApp {
    fn default() -> Self {
        let path = std::env::args()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../examples/simple.report.json"
                ))
            });
        let report = Report::from_file(path.to_string_lossy().as_ref())
            .expect("Cannot load report definition");

        Self {
            status: format!("Loaded {}", path.display()),
            path,
            report,
            selection: None,
            zoom: 1.0,
            dirty: false,
        }
    }
}

impl DesignerApp {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Reload => match Report::from_file(self.path.to_string_lossy().as_ref()) {
                Ok(report) => {
                    self.report = report;
                    self.selection = None;
                    self.dirty = false;
                    self.status = format!("Reloaded {}", self.path.display());
                }
                Err(error) => self.status = format!("Reload failed: {error}"),
            },
            Message::Save => match self
                .report
                .save_to_file(self.path.to_string_lossy().as_ref())
            {
                Ok(()) => {
                    self.dirty = false;
                    self.status = format!("Saved {}", self.path.display());
                }
                Err(error) => self.status = format!("Save failed: {error}"),
            },
            Message::ZoomIn => self.zoom = (self.zoom + 0.1).min(2.0),
            Message::ZoomOut => self.zoom = (self.zoom - 0.1).max(0.5),
            Message::ZoomReset => self.zoom = 1.0,
            Message::Select(selection) => self.selection = selection,
            Message::MoveItem { selection, dx, dy } => {
                if let Some(page) = self.report.pages.first_mut() {
                    let band_width = page.printable_width().0;
                    if let Some(band) = page.bands.get_mut(selection.band) {
                        let band_height = band.height.0;
                        if let Some(item) = band.items.get_mut(selection.item) {
                            move_item(item, dx, dy, band_width, band_height);
                            self.selection = Some(selection);
                            self.dirty = true;
                            self.status = "Unsaved changes".to_string();
                        }
                    }
                }
            }
            Message::ResizeItem {
                selection,
                handle,
                dx,
                dy,
            } => {
                if let Some(page) = self.report.pages.first_mut() {
                    let band_width = page.printable_width().0;
                    if let Some(band) = page.bands.get_mut(selection.band) {
                        let band_height = band.height.0;
                        if let Some(item) = band.items.get_mut(selection.item) {
                            resize_item(item, handle, dx, dy, band_width, band_height);
                            self.selection = Some(selection);
                            self.dirty = true;
                            self.status = "Unsaved changes".to_string();
                        }
                    }
                }
            }
        }

        Task::none()
    }

    fn inspector(&self) -> Element<'_, Message> {
        let status = if self.dirty {
            format!("● {}", self.status)
        } else {
            self.status.clone()
        };
        let mut content = column![text("Properties").size(22), text(status).size(12)].spacing(10);

        if let Some(selection) = self.selection {
            let band = &self.report.pages[0].bands[selection.band];
            let item = &band.items[selection.item];

            content = content
                .push(text(format!("Band: {}", band_name(&band.kind))))
                .push(text(format!("Item: {}", item_name(item))))
                .push(text(item_geometry(item)).size(13));

            if let Item::Text(text_item) = item {
                content = content
                    .push(text("Text").size(13))
                    .push(text(&text_item.text).size(13));
            }
        } else {
            content = content.push(text("Click an item on the page to select it."));
        }

        container(content.padding(16))
            .width(INSPECTOR_WIDTH)
            .height(Fill)
            .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let Some(page) = self.report.pages.first() else {
            return container(text("The report does not contain any pages"))
                .center(Fill)
                .into();
        };

        let scale = BASE_SCALE * self.zoom;
        let canvas_width = page.width().0 * scale + PAGE_MARGIN * 2.0;
        let canvas_height = page.height().0 * scale + PAGE_MARGIN * 2.0;
        let canvas = Canvas::new(DesignerCanvas {
            page,
            selection: self.selection,
            scale,
        })
        .width(canvas_width)
        .height(canvas_height);
        let workspace = scrollable(container(canvas).center_x(Fill))
            .width(Fill)
            .height(Fill);
        let toolbar = row![
            button("Reload")
                .style(common::style_button(6.0))
                .on_press(Message::Reload),
            button("Save JSON")
                .style(common::style_button(6.0))
                .on_press(Message::Save),
            button("−")
                .style(common::style_button(8.0))
                .on_press(Message::ZoomOut),
            button(text(format!("{}%", (self.zoom * 100.0).round() as u32)))
                .style(common::style_button(6.0))
                .on_press(Message::ZoomReset),
            button("+")
                .style(common::style_button(6.0))
                .on_press(Message::ZoomIn),
            text(self.path.display().to_string()).size(13)
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        column![
            container(toolbar).padding(10).width(Fill),
            row![workspace, self.inspector()].height(Fill)
        ]
        .into()
    }
}

struct DesignerCanvas<'a> {
    page: &'a Page,
    selection: Option<Selection>,
    scale: f32,
}

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
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                let (selection, operation) =
                    if let Some((selection, handle)) = self.resize_hit_test(position) {
                        (
                            Some(selection),
                            Some(DragOperation::Resize(selection, handle)),
                        )
                    } else {
                        let selection = self.hit_test(position);
                        (selection, selection.map(DragOperation::Move))
                    };
                state.dragging = operation;
                state.last_position = operation.map(|_| position);

                Some(canvas::Action::publish(Message::Select(selection)).and_capture())
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
                };

                Some(canvas::Action::publish(message).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging.take().is_some() {
                    state.last_position = None;
                    Some(canvas::Action::capture())
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
                    width: 1.0,
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
                draw_item(
                    &mut frame,
                    item,
                    content_x,
                    band_y,
                    self.scale,
                    self.selection
                        == Some(Selection {
                            band: band_index,
                            item: item_index,
                        }),
                );
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
        cursor
            .position_in(bounds)
            .filter(|position| {
                self.resize_hit_test(*position).is_some() || self.hit_test(*position).is_some()
            })
            .map(|_| mouse::Interaction::Pointer)
            .unwrap_or_default()
    }
}

#[derive(Default)]
struct CanvasState {
    dragging: Option<DragOperation>,
    last_position: Option<Point>,
}

impl DesignerCanvas<'_> {
    fn resize_hit_test(&self, position: Point) -> Option<(Selection, ResizeHandle)> {
        let selection = self.selection?;
        let (item, offset_x, offset_y) = self.selected_item(selection)?;

        resize_handle_points(item, offset_x, offset_y, self.scale)
            .into_iter()
            .find(|(point, _)| handle_bounds(*point).contains(position))
            .map(|(_, handle)| (selection, handle))
    }

    fn selected_item(&self, selection: Selection) -> Option<(&Item, f32, f32)> {
        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;

        for (band_index, band) in self.page.bands.iter().enumerate() {
            if band_index == selection.band {
                return band
                    .items
                    .get(selection.item)
                    .map(|item| (item, content_x, band_y));
            }
            band_y += band.height.0 * self.scale;
        }

        None
    }

    fn hit_test(&self, position: Point) -> Option<Selection> {
        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;
        let mut candidates = Vec::new();

        for (band_index, band) in self.page.bands.iter().enumerate() {
            for (item_index, item) in band.items.iter().enumerate() {
                if let Some(rect) = item_bounds(item, content_x, band_y, self.scale)
                    && rect.contains(position)
                {
                    candidates.push(Selection {
                        band: band_index,
                        item: item_index,
                    });
                }
            }
            band_y += band.height.0 * self.scale;
        }

        candidates.pop()
    }
}

fn draw_item(
    frame: &mut Frame<Renderer>,
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    selected: bool,
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
            if let Some(rect) = item_bounds(item, offset_x, offset_y, scale) {
                let path = Path::rectangle(rect.position(), rect.size());
                frame.fill(&path, Color::from_rgba8(30, 140, 155, 0.06));
                frame.stroke(&path, stroke);

                if rect.width >= 50.0 && rect.height >= 14.0 {
                    frame.fill_text(canvas::Text {
                        content: truncate(&text_item.text, 34),
                        position: Point::new(rect.x + 4.0, rect.y + rect.height / 2.0),
                        max_width: (rect.width - 8.0).max(0.0),
                        color,
                        size: iced::Pixels(11.0),
                        align_y: iced::alignment::Vertical::Center,
                        ..Default::default()
                    });
                }
            }
        }
        Item::Rectangle(_) | Item::Image(_) => {
            if let Some(rect) = item_bounds(item, offset_x, offset_y, scale) {
                frame.stroke(&Path::rectangle(rect.position(), rect.size()), stroke);
            }
        }
    }

    if selected {
        draw_resize_handles(frame, item, offset_x, offset_y, scale);
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

fn resize_handle_points(
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

fn handle_bounds(center: Point) -> Rectangle {
    Rectangle::new(
        Point::new(center.x - HANDLE_SIZE / 2.0, center.y - HANDLE_SIZE / 2.0),
        Size::new(HANDLE_SIZE, HANDLE_SIZE),
    )
}

fn draw_page_grid(frame: &mut Frame<Renderer>, page: &Page, page_origin: Point, scale: f32) {
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

fn draw_rulers(
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

fn selection_bounds(page: &Page, selection: Selection, scale: f32) -> Option<Rectangle> {
    let content_x = PAGE_MARGIN + page.margins.left.0 * scale;
    let mut band_y = PAGE_MARGIN + page.margins.top.0 * scale;

    for (band_index, band) in page.bands.iter().enumerate() {
        if band_index == selection.band {
            let item = band.items.get(selection.item)?;
            return item_bounds(item, content_x, band_y, scale);
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

fn item_bounds(item: &Item, offset_x: f32, offset_y: f32, scale: f32) -> Option<Rectangle> {
    let rect = match item {
        Item::Text(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::Rectangle(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::Image(item) => Rectangle::new(
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

fn band_name(kind: &BandKind) -> &'static str {
    match kind {
        BandKind::ReportHeader => "ReportHeader",
        BandKind::PageHeader => "PageHeader",
        BandKind::Data { .. } => "DataBand",
        BandKind::PageFooter => "PageFooter",
        BandKind::ReportFooter => "ReportFooter",
    }
}

fn band_colors(kind: &BandKind) -> (Color, Color) {
    match kind {
        BandKind::ReportHeader => (
            Color::from_rgba8(224, 238, 250, 0.30),
            Color::from_rgb8(52, 115, 165),
        ),
        BandKind::PageHeader => (
            Color::from_rgba8(224, 242, 238, 0.30),
            Color::from_rgb8(35, 135, 120),
        ),
        BandKind::Data { .. } => (
            Color::from_rgba8(250, 239, 218, 0.30),
            Color::from_rgb8(185, 120, 35),
        ),
        BandKind::PageFooter => (
            Color::from_rgba8(236, 231, 248, 0.30),
            Color::from_rgb8(115, 85, 165),
        ),
        BandKind::ReportFooter => (
            Color::from_rgba8(245, 230, 235, 0.30),
            Color::from_rgb8(165, 75, 105),
        ),
    }
}

fn move_item(item: &mut Item, dx: f32, dy: f32, band_width: f32, band_height: f32) {
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
        Item::Line(item) => {
            item.x1.0 += dx;
            item.y1.0 += dy;
            item.x2.0 += dx;
            item.y2.0 += dy;
        }
    }
}

fn resize_item(
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
fn resize_rectangle(
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
fn constrained_delta(
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

fn item_name(item: &Item) -> &'static str {
    match item {
        Item::Text(_) => "Text",
        Item::Line(_) => "Line",
        Item::Rectangle(_) => "Rectangle",
        Item::Image(_) => "Image",
    }
}

fn item_geometry(item: &Item) -> String {
    match item {
        Item::Text(item) => format!(
            "X: {:.1} mm  Y: {:.1} mm\nWidth: {:.1} mm  Height: {:.1} mm",
            item.x.0, item.y.0, item.width.0, item.height.0
        ),
        Item::Rectangle(item) => format!(
            "X: {:.1} mm  Y: {:.1} mm\nWidth: {:.1} mm  Height: {:.1} mm",
            item.x.0, item.y.0, item.width.0, item.height.0
        ),
        Item::Image(item) => format!(
            "X: {:.1} mm  Y: {:.1} mm\nWidth: {:.1} mm  Height: {:.1} mm",
            item.x.0, item.y.0, item.width.0, item.height.0
        ),
        Item::Line(item) => format!(
            "X1: {:.1} mm  Y1: {:.1} mm\nX2: {:.1} mm  Y2: {:.1} mm\nWidth: {:.1} mm  Height: {:.1} mm",
            item.x1.0,
            item.y1.0,
            item.x2.0,
            item.y2.0,
            (item.x2.0 - item.x1.0).abs(),
            (item.y2.0 - item.y1.0).abs()
        ),
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{shortened}…")
    } else {
        shortened
    }
}

fn main() -> iced::Result {
    iced::application(DesignerApp::default, DesignerApp::update, DesignerApp::view)
        .title("report-rs Designer")
        .window_size(Size::new(1280.0, 850.0))
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_delta_is_clamped_to_band_bounds() {
        let delta = constrained_delta(10.0, 30.0, 5.0, 15.0, -50.0, 100.0, 100.0, 40.0);

        assert_eq!(delta, (-10.0, 25.0));
    }

    #[test]
    fn resize_is_clamped_and_preserves_minimum_size() {
        let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

        resize_rectangle(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            ResizeHandle::BottomRight,
            100.0,
            100.0,
            40.0,
            20.0,
        );
        assert_eq!((x, y, width, height), (10.0, 5.0, 30.0, 15.0));

        resize_rectangle(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            ResizeHandle::TopLeft,
            100.0,
            100.0,
            40.0,
            20.0,
        );
        assert_eq!((x, y, width, height), (39.0, 19.0, 1.0, 1.0));
    }

    #[test]
    fn middle_handle_changes_only_height() {
        let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

        resize_rectangle(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            ResizeHandle::Bottom,
            12.0,
            3.0,
            100.0,
            40.0,
        );

        assert_eq!((x, y, width, height), (10.0, 5.0, 20.0, 13.0));
    }

    #[test]
    fn side_handle_changes_only_width() {
        let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

        resize_rectangle(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            ResizeHandle::Right,
            4.0,
            12.0,
            100.0,
            40.0,
        );

        assert_eq!((x, y, width, height), (10.0, 5.0, 24.0, 10.0));
    }
}
