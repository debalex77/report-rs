use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::widget::image::Handle;
use iced::widget::{
    Space, button, column, container, opaque, progress_bar, row, scrollable, stack, text,
    text_input,
};
use iced::{
    Color, Element, Fill, Font, Point, Rectangle, Renderer, Size, Task, Theme,
    font::{Style, Weight},
};

use report_core::common;

use report_core::datasource::{
    ReportContext, Row, Value, load_report_data_sources, parse_report_parameter_value,
};
use report_core::layout::{LayoutEngine, RenderedItem, RenderedPage};
use report_core::model::{HorizontalAlign, Report, ReportParameterType, VerticalAlign};

use report_core::font::measurer::RealFontMeasurer;
use report_core::image::layout::calculate_image_placement;
use report_core::image::loader::load_image;
use report_core::layout::text::pt_to_mm;

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

#[derive(Debug, Clone)]
struct PreviewImage {
    handle: Handle,
    width: u32,
    height: u32,
}

struct PreviewApp {
    report: Report,
    pages: Vec<RenderedPage>,
    current_page: usize,
    zoom: f32,
    debug_overlay: bool,
    images: HashMap<String, PreviewImage>,
    report_dir: PathBuf,
    error_message: Option<String>,
    parameter_values: Vec<String>,
    parameters_pending: bool,
    processing: bool,
    processing_duration: Option<std::time::Duration>,
    progress: f32,
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
        let started = std::time::Instant::now();
        let path = std::env::args().nth(1).unwrap_or_else(|| {
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/simple.report.json"
            )
            .to_string()
        });

        let report = Report::from_file(&path).expect("Cannot load report");
        let report_dir = FsPath::new(&path)
            .parent()
            .expect("Report path should have a parent directory");
        let parameter_values = report
            .parameters
            .iter()
            .map(|parameter| parameter.default_value.clone().unwrap_or_default())
            .collect();
        let parameters_pending = report.parameters.iter().any(|parameter| parameter.required);
        let (pages, images, error_message) = if parameters_pending {
            (Vec::new(), HashMap::new(), None)
        } else {
            render_report(&report, report_dir, ReportContext::new())
        };

        let page_count = pages.len();
        let processing_duration = (!parameters_pending).then(|| started.elapsed());
        if let Some(ready_path) = ready_file_argument() {
            let summary = if parameters_pending {
                "waiting for parameters".to_string()
            } else {
                format!("{page_count} pages")
            };
            let _ = std::fs::write(ready_path, summary);
        }
        Self {
            report,
            pages,
            current_page: 0,
            zoom: 1.0,
            debug_overlay: false,
            images,
            report_dir: report_dir.to_path_buf(),
            error_message,
            parameter_values,
            parameters_pending,
            processing: false,
            processing_duration,
            progress: 0.0,
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
    DismissError,
    ParameterChanged(usize, String),
    ApplyParameters,
    OpenParameters,
    CloseParameters,
    RenderFinished(RenderResult),
    ProcessingTick,
}

#[derive(Debug, Clone)]
struct RenderResult {
    pages: Vec<RenderedPage>,
    images: HashMap<String, PreviewImage>,
    error: Option<String>,
    duration: std::time::Duration,
}

fn ready_file_argument() -> Option<PathBuf> {
    let arguments = std::env::args().collect::<Vec<_>>();
    arguments
        .windows(2)
        .find(|arguments| arguments[0] == "--ready-file")
        .map(|arguments| PathBuf::from(&arguments[1]))
}

fn format_duration(duration: std::time::Duration) -> String {
    if duration.as_secs() >= 60 {
        format!(
            "{} min {} sec",
            duration.as_secs() / 60,
            duration.as_secs() % 60
        )
    } else {
        format!("{:.1} sec", duration.as_secs_f32())
    }
}

fn render_report(
    report: &Report,
    report_dir: &FsPath,
    mut context: ReportContext,
) -> (
    Vec<RenderedPage>,
    HashMap<String, PreviewImage>,
    Option<String>,
) {
    if report.data_sources.is_empty() {
        let example = example_context();
        for (name, value) in example.parameters() {
            if context.parameter(name).is_none() {
                context.set_parameter(name, value.clone());
            }
        }
        if let Some(rows) = example.table("horeca_units") {
            context.add_table("horeca_units", rows.clone());
        }
    }
    let error_message = load_report_data_sources(report, report_dir, &mut context)
        .err()
        .map(|error| format!("Data source failed: {error}"));
    let measurer = RealFontMeasurer::new();
    let pages = report
        .pages
        .iter()
        .flat_map(|page| LayoutEngine::render_with_measurer(page, &context, &measurer))
        .collect::<Vec<_>>();
    let images = load_preview_images(&pages, report_dir);
    (pages, images, error_message)
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
                    underline,
                    strikeout,
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
                        if *underline || *strikeout {
                            let decoration_left = match horizontal_align {
                                HorizontalAlign::Left => text_x,
                                HorizontalAlign::Center => text_x - line.width * scale / 2.0,
                                HorizontalAlign::Right => text_x - line.width * scale,
                            };
                            let decoration_color = Color::from_rgba8(
                                text_color.r,
                                text_color.g,
                                text_color.b,
                                text_color.a as f32 / 255.0,
                            );
                            for y_factor in [underline.then_some(0.92), strikeout.then_some(0.55)]
                                .into_iter()
                                .flatten()
                            {
                                let decoration_y =
                                    page_y + (line_y_mm + pt_to_mm(*font_size) * y_factor) * scale;
                                frame.stroke(
                                    &Path::line(
                                        Point::new(decoration_left, decoration_y),
                                        Point::new(
                                            decoration_left + line.width * scale,
                                            decoration_y,
                                        ),
                                    ),
                                    canvas::Stroke {
                                        width: 1.0,
                                        style: canvas::Style::Solid(decoration_color),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
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

            Message::DismissError => self.error_message = None,
            Message::ParameterChanged(index, value) => {
                if let Some(target) = self.parameter_values.get_mut(index) {
                    *target = value;
                }
            }
            Message::ApplyParameters => {
                let mut context = ReportContext::new();
                for (index, parameter) in self.report.parameters.iter().enumerate() {
                    let value = self
                        .parameter_values
                        .get(index)
                        .map(|value| value.trim())
                        .unwrap_or_default();
                    if value.is_empty() {
                        if parameter.required {
                            self.error_message =
                                Some(format!("Parameter `{}` is required", parameter.name));
                            return Task::none();
                        }
                        continue;
                    }
                    match parse_report_parameter_value(parameter, value) {
                        Ok(value) => context.set_parameter(&parameter.name, value),
                        Err(error) => {
                            self.error_message = Some(error.to_string());
                            return Task::none();
                        }
                    }
                }
                let report = self.report.clone();
                let report_dir = self.report_dir.clone();
                self.parameters_pending = false;
                self.processing = true;
                self.progress = 0.0;
                self.error_message = None;
                return Task::perform(
                    async move {
                        let started = std::time::Instant::now();
                        let (pages, images, error) = render_report(&report, &report_dir, context);
                        RenderResult {
                            pages,
                            images,
                            error,
                            duration: started.elapsed(),
                        }
                    },
                    Message::RenderFinished,
                );
            }
            Message::OpenParameters => {
                self.error_message = None;
                self.parameter_values = self
                    .report
                    .parameters
                    .iter()
                    .map(|parameter| parameter.default_value.clone().unwrap_or_default())
                    .collect();
                self.parameters_pending = true;
            }
            Message::CloseParameters => {
                if !self.pages.is_empty() {
                    self.error_message = None;
                    self.parameters_pending = false;
                }
            }
            Message::RenderFinished(result) => {
                self.pages = result.pages;
                self.images = result.images;
                self.error_message = result.error;
                self.current_page = 0;
                self.processing = false;
                self.processing_duration = Some(result.duration);
            }
            Message::ProcessingTick => {
                self.progress = (self.progress + 3.0) % 200.0;
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let base: Element<'_, Message> = if self.pages.is_empty() {
            container(text(if self.processing {
                "Processing report data and rendering pages…"
            } else if self.parameters_pending {
                "Complete the report parameters to generate the preview"
            } else {
                "No pages to preview"
            }))
            .center(Fill)
            .into()
        } else {
            self.preview_content()
        };
        let status_content: Element<'_, Message> = if self.processing {
            row![
                text("Processing data and rendering pages…").size(11),
                progress_bar(
                    0.0..=100.0,
                    if self.progress <= 100.0 {
                        self.progress
                    } else {
                        200.0 - self.progress
                    },
                )
                .length(220)
                .girth(6),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .into()
        } else if let Some(duration) = self.processing_duration {
            text(format!(
                "Generated {} pages in {}",
                self.pages.len(),
                format_duration(duration)
            ))
            .size(11)
            .into()
        } else {
            text("Ready").size(11).into()
        };
        let base: Element<'_, Message> = column![
            container(base).height(Fill),
            container(status_content).padding([5, 10]).width(Fill),
        ]
        .into();
        if self.parameters_pending {
            stack![base, self.parameter_dialog()].into()
        } else {
            base
        }
    }

    fn preview_content(&self) -> Element<'_, Message> {
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

        let preview_area = container(canvas)
            .center_x(Fill)
            .width(Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(196, 200, 205))),
                ..Default::default()
            });
        let viewport = scrollable(preview_area).width(Fill).height(Fill);
        let zoom_percent = (self.zoom * 100.0).round() as u32;

        let toolbar = row![
            button("◀")
                .style(common::style_button(6.0))
                .on_press(Message::PreviousPage),
            text(format!(
                "Page {} / {}",
                self.current_page + 1,
                self.pages.len()
            )),
            button("▶")
                .style(common::style_button(6.0))
                .on_press(Message::NextPage),
            button("−")
                .style(common::style_button(6.0))
                .on_press(Message::ZoomOut),
            button(text(format!("{}%", zoom_percent)))
                .style(common::style_button(6.0))
                .on_press(Message::ZoomReset),
            button("+")
                .style(common::style_button(6.0))
                .on_press(Message::ZoomIn),
            button("Debug")
                .style(common::style_button(6.0))
                .on_press(Message::ToggleDebug),
            button("Export PDF")
                .style(common::style_button(6.0))
                .on_press(Message::ExportPdf),
            button("Open PDF")
                .style(common::style_button(6.0))
                .on_press(Message::OpenPdf),
            if self.report.parameters.is_empty() {
                Element::<Message>::from(Space::new().width(0))
            } else {
                button("Parameters")
                    .style(common::style_button(6.0))
                    .on_press(Message::OpenParameters)
                    .into()
            }
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let mut content = column![container(toolbar).padding(10)];
        if let Some(error) = &self.error_message {
            content = content.push(
                container(
                    row![
                        text(format!("⚠  {error}")).size(12),
                        iced::widget::Space::new().width(Fill),
                        button("×").on_press(Message::DismissError),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                )
                .padding([4, 8])
                .width(Fill)
                .style(container::danger),
            );
        }
        content.push(viewport).into()
    }

    fn parameter_dialog(&self) -> Element<'_, Message> {
        let mut fields = column![
            text("Report parameters").size(20),
            text("Complete the values before generating the preview.").size(11),
        ]
        .spacing(8);
        for (index, parameter) in self.report.parameters.iter().enumerate() {
            let kind = match parameter.value_type {
                ReportParameterType::Text => "Text",
                ReportParameterType::Integer => "Integer",
                ReportParameterType::Double => "Double",
                ReportParameterType::Boolean => "Boolean (true/false)",
                ReportParameterType::Date => "Date (YYYY-MM-DD)",
                ReportParameterType::DateTime => "Date-time",
            };
            fields = fields.push(
                row![
                    column![
                        text(format!(
                            "{}{}",
                            parameter.name,
                            if parameter.required { " *" } else { "" }
                        ))
                        .size(12),
                        text(kind).size(9),
                    ]
                    .width(150),
                    text_input(
                        "Value",
                        self.parameter_values
                            .get(index)
                            .map(String::as_str)
                            .unwrap_or(""),
                    )
                    .width(Fill)
                    .size(12)
                    .padding(6)
                    .on_input(move |value| Message::ParameterChanged(index, value)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            );
        }
        if let Some(error) = &self.error_message {
            fields = fields.push(text(error).size(11).color(Color::from_rgb8(220, 75, 70)));
        }
        fields = fields.push(
            row![
                Space::new().width(Fill),
                if self.pages.is_empty() {
                    Element::<Message>::from(Space::new().width(0))
                } else {
                    button(text("Cancel").size(12))
                        .style(common::style_button(6.0))
                        .on_press(Message::CloseParameters)
                        .into()
                },
                button(text("Generate preview").size(12))
                    .style(common::style_button(6.0))
                    .on_press(Message::ApplyParameters),
            ]
            .align_y(iced::Alignment::Center),
        );
        let dialog = container(fields)
            .padding(18)
            .width(520)
            .style(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(theme.palette().background)),
                border: iced::Border {
                    color: theme.extended_palette().background.strong.color,
                    width: 1.0,
                    radius: iced::border::radius(12),
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.35),
                    offset: iced::Vector::new(0.0, 7.0),
                    blur_radius: 20.0,
                },
                ..Default::default()
            });
        opaque(
            container(dialog)
                .center(Fill)
                .width(Fill)
                .height(Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba8(0, 0, 0, 0.55))),
                    ..Default::default()
                }),
        )
        .into()
    }
}

fn main() -> iced::Result {
    let arguments = std::env::args().collect::<Vec<_>>();
    if arguments.get(1).is_some_and(|value| value == "--benchmark") {
        let Some(path) = arguments.get(2) else {
            eprintln!("usage: report-preview --benchmark <report.json>");
            return Ok(());
        };
        let started = std::time::Instant::now();
        let report = match Report::from_file(path) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("Cannot load report: {error}");
                return Ok(());
            }
        };
        let report_dir = FsPath::new(path).parent().unwrap_or_else(|| FsPath::new("."));
        let (pages, _, error) = render_report(&report, report_dir, ReportContext::new());
        if let Some(error) = error {
            eprintln!("{error}");
        }
        println!(
            "Rendered {} pages in {}",
            pages.len(),
            format_duration(started.elapsed())
        );
        return Ok(());
    }

    let font_bytes = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
        .expect("Cannot load DejaVuSans.ttf");

    iced::application(PreviewApp::default, PreviewApp::update, PreviewApp::view)
        .title("report-rs Preview")
        .font(font_bytes)
        .subscription(|app| {
            if app.processing {
                iced::time::every(std::time::Duration::from_millis(80))
                    .map(|_| Message::ProcessingTick)
            } else {
                iced::Subscription::none()
            }
        })
        .run()
}
