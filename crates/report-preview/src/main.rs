use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::widget::image::Handle;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{
    Color, Element, Fill, Font, Point, Rectangle, Renderer, Size, Task, Theme,
    font::{Style, Weight},
};

use report_core::datasource::{ReportContext, Row, Value};
use report_core::layout::{LayoutEngine, RenderedItem, RenderedPage};
use report_core::model::{HorizontalAlign, Report, VerticalAlign};

use report_core::font_measurer::RealFontMeasurer;
use report_core::image_layout::calculate_image_placement;
use report_core::image_loader::load_image;
use report_core::text_layout::pt_to_mm;

const PX_PER_MM: f32 = 96.0 / 25.4;
const PAGE_MARGIN_PX: f32 = 40.0;

fn example_context() -> ReportContext {
    let units = [
        ("Bucătăria principală", "Chef executiv"),
        ("Zona de pregătire rece", "Sous-chef"),
        ("Restaurant - salon principal", "Manager de sală"),
        ("Terasă și servire exterioară", "Supervizor terasă"),
        ("Bar și băuturi", "Manager bar"),
        ("Depozit și recepție marfă", "Gestionar"),
        ("Serviciul catering", "Coordonator catering"),
        ("Igienizare și mentenanță", "Supervizor operațional"),
    ];

    let rows = units
        .into_iter()
        .enumerate()
        .map(|(index, (unit_name, responsible_role))| {
            let mut row = Row::new();
            row.insert("nr".to_string(), Value::Number((index + 1) as f64));
            row.insert(
                "unit_name".to_string(),
                Value::String(unit_name.to_string()),
            );
            row.insert(
                "responsible_role".to_string(),
                Value::String(responsible_role.to_string()),
            );
            row
        })
        .collect();

    let mut context = ReportContext::new();
    context.set_parameter(
        "report_subtitle",
        Value::String("Model demonstrativ pentru unități și zone de lucru".to_string()),
    );
    context.set_parameter(
        "approval_role",
        Value::String("Manager operațional".to_string()),
    );
    context.add_table("horeca_units", rows);
    context
}

struct PreviewImage {
    handle: Handle,
    width: u32,
    height: u32,
}

struct PreviewApp {
    pages: Vec<RenderedPage>,
    current_page: usize,
    zoom: f32,
    debug_overlay: bool,
    images: HashMap<String, PreviewImage>,
    report_dir: PathBuf,
}

fn load_preview_images(
    pages: &[RenderedPage],
    report_dir: &FsPath,
) -> HashMap<String, PreviewImage> {
    let mut images = HashMap::new();

    for source in pages
        .iter()
        .flat_map(|page| &page.items)
        .filter_map(|item| {
            if let RenderedItem::Image { source, .. } = item {
                Some(source)
            } else {
                None
            }
        })
    {
        if images.contains_key(source) {
            continue;
        }

        let source_path = PathBuf::from(source);
        let resolved_path = if source_path.is_absolute() {
            source_path
        } else {
            report_dir.join(source_path)
        };

        match load_image(&resolved_path) {
            Ok(image) => {
                let width = image.width;
                let height = image.height;
                images.insert(
                    source.clone(),
                    PreviewImage {
                        handle: Handle::from_rgba(width, height, image.rgba),
                        width,
                        height,
                    },
                );
            }
            Err(error) => {
                eprintln!("Cannot load preview image '{source}': {error}");
            }
        }
    }

    images
}

impl Default for PreviewApp {
    fn default() -> Self {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/simple.report.json"
        );

        let measurer = RealFontMeasurer::new();
        let report = Report::from_file(path).expect("Cannot load report");
        let context = example_context();
        let pages = LayoutEngine::render_with_measurer(&report.pages[0], &context, &measurer);
        let report_dir = FsPath::new(path)
            .parent()
            .expect("Report path should have a parent directory");
        let images = load_preview_images(&pages, report_dir);

        Self {
            pages,
            current_page: 0,
            zoom: 1.0,
            debug_overlay: false,
            images,
            report_dir: report_dir.to_path_buf(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    PreviousPage,
    NextPage,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleDebug,
    ExportPdf,
    OpenPdf,
}

struct PageCanvas<'a> {
    page: &'a RenderedPage,
    zoom: f32,
    debug_overlay: bool,
    images: &'a HashMap<String, PreviewImage>,
}

fn draw_text_border(
    frame: &mut Frame<Renderer>,
    border: &report_core::model::Border,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    page_x: f32,
    page_y: f32,
    scale: f32,
) {
    let x1 = page_x + x * scale;
    let y1 = page_y + y * scale;

    let x2 = x1 + width * scale;
    let y2 = y1 + height * scale;

    let stroke_width = border.width * scale;

    let stroke = || canvas::Stroke {
        width: stroke_width,
        ..Default::default()
    };

    if border.top {
        frame.stroke(
            &Path::line(Point::new(x1, y1), Point::new(x2, y1)),
            stroke(),
        );
    }

    if border.bottom {
        frame.stroke(
            &Path::line(Point::new(x1, y2), Point::new(x2, y2)),
            stroke(),
        );
    }

    if border.left {
        frame.stroke(
            &Path::line(Point::new(x1, y1), Point::new(x1, y2)),
            stroke(),
        );
    }

    if border.right {
        frame.stroke(
            &Path::line(Point::new(x2, y1), Point::new(x2, y2)),
            stroke(),
        );
    }
}

impl<'a, Message> canvas::Program<Message> for PageCanvas<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let scale = PX_PER_MM * self.zoom;

        let page_width = self.page.width.0 * scale;
        let page_height = self.page.height.0 * scale;

        let page_x = PAGE_MARGIN_PX;
        let page_y = PAGE_MARGIN_PX;

        // Pagina albă
        let page_rect = Path::rectangle(
            Point::new(page_x, page_y),
            Size::new(page_width, page_height),
        );

        frame.fill(&page_rect, Color::WHITE);

        // Elementele calculate de LayoutEngine
        for item in &self.page.items {
            match item {
                RenderedItem::Text {
                    x,
                    y,
                    width,
                    height,
                    font_size,
                    bold,
                    italic,
                    text_color,
                    lines,
                    line_height,
                    horizontal_align,
                    vertical_align,
                    padding,
                    background,
                    border,
                    ..
                } => {
                    if let Some(background) = background {
                        let rect = Path::rectangle(
                            Point::new(page_x + x.0 * scale, page_y + y.0 * scale),
                            Size::new(width.0 * scale, height.0 * scale),
                        );

                        frame.fill(
                            &rect,
                            Color::from_rgba8(
                                background.r,
                                background.g,
                                background.b,
                                background.a as f32 / 255.0,
                            ),
                        );
                    }

                    let content_left = x.0 + padding.left.0;

                    let content_right = x.0 + width.0 - padding.right.0;

                    let content_width = (content_right - content_left).max(0.0);

                    let text_x = match horizontal_align {
                        HorizontalAlign::Left => page_x + content_left * scale,

                        HorizontalAlign::Center => {
                            page_x + (content_left + content_width / 2.0) * scale
                        }

                        HorizontalAlign::Right => page_x + content_right * scale,
                    };

                    let text_align = match horizontal_align {
                        HorizontalAlign::Left => text::Alignment::Left,
                        HorizontalAlign::Center => text::Alignment::Center,
                        HorizontalAlign::Right => text::Alignment::Right,
                    };

                    let total_text_height_mm = *line_height * lines.len() as f32;

                    let content_top = y.0 + padding.top.0;

                    let content_height = (height.0 - padding.top.0 - padding.bottom.0).max(0.0);

                    let start_y_mm = match vertical_align {
                        VerticalAlign::Top => content_top,

                        VerticalAlign::Center => {
                            content_top + (content_height - total_text_height_mm) / 2.0
                        }

                        VerticalAlign::Bottom => {
                            content_top + content_height - total_text_height_mm
                        }
                    };

                    let font_size_px = pt_to_mm(*font_size) * scale;

                    if self.debug_overlay {
                        let item_rect = Path::rectangle(
                            Point::new(page_x + x.0 * scale, page_y + y.0 * scale),
                            Size::new(width.0 * scale, height.0 * scale),
                        );

                        frame.stroke(
                            &item_rect,
                            canvas::Stroke {
                                width: 1.0,
                                ..Default::default()
                            },
                        );
                    }

                    let family = iced::font::Family::Name("DejaVu Sans");

                    let font = Font {
                        family,

                        weight: if *bold { Weight::Bold } else { Weight::Normal },

                        stretch: iced::font::Stretch::Normal,

                        style: if *italic {
                            Style::Italic
                        } else {
                            Style::Normal
                        },
                    };

                    for (index, line) in lines.iter().enumerate() {
                        let line_y_mm = start_y_mm + *line_height * index as f32;

                        frame.fill_text(canvas::Text {
                            content: line.text.clone(),
                            position: Point::new(text_x, page_y + line_y_mm * scale),
                            color: Color::from_rgba8(
                                text_color.r,
                                text_color.g,
                                text_color.b,
                                text_color.a as f32 / 255.0,
                            ),
                            size: iced::Pixels(font_size_px),
                            font,
                            align_x: text_align,
                            ..Default::default()
                        });
                    }

                    if let Some(border) = border {
                        draw_text_border(
                            &mut frame, border, x.0, y.0, width.0, height.0, page_x, page_y, scale,
                        );
                    }
                }

                RenderedItem::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    width,
                } => {
                    let line = Path::line(
                        Point::new(page_x + x1.0 * scale, page_y + y1.0 * scale),
                        Point::new(page_x + x2.0 * scale, page_y + y2.0 * scale),
                    );

                    frame.stroke(
                        &line,
                        canvas::Stroke {
                            width: width.0 * scale,
                            ..Default::default()
                        },
                    );
                }

                RenderedItem::Rectangle {
                    x,
                    y,
                    width,
                    height,
                    border_width,
                } => {
                    let rect = Path::rectangle(
                        Point::new(page_x + x.0 * scale, page_y + y.0 * scale),
                        Size::new(width.0 * scale, height.0 * scale),
                    );

                    frame.stroke(
                        &rect,
                        canvas::Stroke {
                            width: border_width.0 * scale,
                            ..Default::default()
                        },
                    );
                }

                RenderedItem::Image {
                    x,
                    y,
                    width,
                    height,
                    source,
                    fit,
                } => {
                    if let Some(image) = self.images.get(source) {
                        let placement = calculate_image_placement(
                            *x,
                            *y,
                            *width,
                            *height,
                            image.width,
                            image.height,
                            *fit,
                        );
                        let bounds = Rectangle::new(
                            Point::new(
                                page_x + placement.x.0 * scale,
                                page_y + placement.y.0 * scale,
                            ),
                            Size::new(placement.width.0 * scale, placement.height.0 * scale),
                        );

                        frame.draw_image(bounds, &image.handle);
                    } else {
                        let bounds = Rectangle::new(
                            Point::new(page_x + x.0 * scale, page_y + y.0 * scale),
                            Size::new(width.0 * scale, height.0 * scale),
                        );
                        let placeholder = Path::rectangle(bounds.position(), bounds.size());
                        let top_left = bounds.position();
                        let bottom_right =
                            Point::new(bounds.x + bounds.width, bounds.y + bounds.height);
                        let top_right = Point::new(bounds.x + bounds.width, bounds.y);
                        let bottom_left = Point::new(bounds.x, bounds.y + bounds.height);
                        let stroke = canvas::Stroke {
                            style: canvas::Style::Solid(Color::from_rgb8(180, 60, 60)),
                            width: 1.0,
                            ..Default::default()
                        };

                        frame.stroke(&placeholder, stroke.clone());
                        frame.stroke(&Path::line(top_left, bottom_right), stroke.clone());
                        frame.stroke(&Path::line(top_right, bottom_left), stroke);
                    }
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

impl PreviewApp {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PreviousPage => {
                if self.current_page > 0 {
                    self.current_page -= 1;
                }
            }

            Message::NextPage => {
                if self.current_page + 1 < self.pages.len() {
                    self.current_page += 1;
                }
            }

            Message::ZoomIn => {
                self.zoom = (self.zoom + 0.1).min(2.0);
            }

            Message::ZoomOut => {
                self.zoom = (self.zoom - 0.1).max(0.5);
            }

            Message::ZoomReset => {
                self.zoom = 1.0;
            }

            Message::ToggleDebug => {
                self.debug_overlay = !self.debug_overlay;
            }

            Message::ExportPdf => {
                let output_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../output.pdf");

                match report_pdf::PdfRenderer::render_to_file_with_base_dir(
                    &self.pages,
                    output_path,
                    &self.report_dir,
                ) {
                    Ok(()) => {
                        println!("PDF created: {output_path}");

                        if let Err(error) = std::process::Command::new("xdg-open")
                            .arg(output_path)
                            .spawn()
                        {
                            eprintln!("Cannot open PDF: {error}");
                        }
                    }

                    Err(error) => {
                        eprintln!("Cannot create PDF: {error:?}");
                    }
                }
            }

            Message::OpenPdf => {
                let output_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../output.pdf");

                if let Err(error) = std::process::Command::new("xdg-open")
                    .arg(output_path)
                    .spawn()
                {
                    eprintln!("Cannot open PDF: {error}");
                }
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        if self.pages.is_empty() {
            return container(text("No pages to preview")).center(Fill).into();
        }

        let page = &self.pages[self.current_page];

        let scale = PX_PER_MM * self.zoom;

        let canvas_width = page.width.0 * scale + PAGE_MARGIN_PX * 2.0;

        let canvas_height = page.height.0 * scale + PAGE_MARGIN_PX * 2.0;

        let canvas = Canvas::new(PageCanvas {
            page,
            zoom: self.zoom,
            debug_overlay: self.debug_overlay,
            images: &self.images,
        })
        .width(canvas_width)
        .height(canvas_height);

        let preview_area = container(canvas).center_x(Fill);
        let viewport = scrollable(preview_area).width(Fill).height(Fill);
        let zoom_percent = (self.zoom * 100.0).round() as u32;

        let toolbar = row![
            button("◀").on_press(Message::PreviousPage),
            text(format!(
                "Page {} / {}",
                self.current_page + 1,
                self.pages.len()
            )),
            button("▶").on_press(Message::NextPage),
            button("−").on_press(Message::ZoomOut),
            button(text(format!("{}%", zoom_percent))).on_press(Message::ZoomReset),
            button("+").on_press(Message::ZoomIn),
            button("Debug").on_press(Message::ToggleDebug),
            button("Export PDF").on_press(Message::ExportPdf),
            button("Open PDF").on_press(Message::OpenPdf)
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        column![container(toolbar).padding(10), viewport,].into()
    }
}

fn main() -> iced::Result {
    let font_bytes = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
        .expect("Cannot load DejaVuSans.ttf");

    iced::application(PreviewApp::default, PreviewApp::update, PreviewApp::view)
        .title("report-rs Preview")
        .font(font_bytes)
        .run()
}
