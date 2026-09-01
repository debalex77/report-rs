use printpdf::{
    FontId, Line, LinePoint, Mm, Op, ParsedFont, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt,
    RawImage, RawImageData, RawImageFormat, TextItem, XObjectId, XObjectTransform,
};

use report_core::image::layout::calculate_image_placement;
use report_core::image::loader::{ImageLoadError, load_image};
use report_core::layout::{RenderedItem, RenderedPage};

use report_core::layout::text::{mm_to_pt, pt_to_mm};

use report_core::model::{HorizontalAlign, VerticalAlign};

use report_core::font::FontSpec;
use report_core::font::resolver::SystemFontResolver;

use std::collections::HashMap;
use std::path::Path;

pub struct PdfRenderer;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FontKey {
    family: String,
    bold: bool,
    italic: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Font not found: {0}")]
    FontNotFound(String),

    #[error(transparent)]
    Image(#[from] ImageLoadError),
}

fn draw_border(
    ops: &mut Vec<Op>,
    border: &report_core::model::Border,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    page_height: f32,
) {
    let left = x;
    let right = x + width;

    let top = page_height - y;
    let bottom = page_height - (y + height);

    let thickness = Pt(mm_to_pt(border.width));

    ops.push(Op::SetOutlineThickness { pt: thickness });

    let mut draw_line = |x1: f32, y1: f32, x2: f32, y2: f32| {
        ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint {
                        p: Point {
                            x: Mm(x1).into(),
                            y: Mm(y1).into(),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Mm(x2).into(),
                            y: Mm(y2).into(),
                        },
                        bezier: false,
                    },
                ],
                is_closed: false,
            },
        });
    };

    if border.top {
        draw_line(left, top, right, top);
    }

    if border.bottom {
        draw_line(left, bottom, right, bottom);
    }

    if border.left {
        draw_line(left, top, left, bottom);
    }

    if border.right {
        draw_line(right, top, right, bottom);
    }
}

fn draw_background(
    ops: &mut Vec<Op>,
    background: &report_core::model::Color,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    page_height: f32,
) {
    let left = x;
    let right = x + width;

    let top = page_height - y;
    let bottom = page_height - (y + height);

    let fill_color = printpdf::Color::Rgb(printpdf::Rgb::new(
        background.r as f32 / 255.0,
        background.g as f32 / 255.0,
        background.b as f32 / 255.0,
        None,
    ));

    ops.push(Op::SetFillColor { col: fill_color });

    ops.push(Op::DrawPolygon {
        polygon: printpdf::Polygon {
            rings: vec![printpdf::PolygonRing {
                points: vec![
                    LinePoint {
                        p: Point {
                            x: Mm(left).into(),
                            y: Mm(top).into(),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Mm(right).into(),
                            y: Mm(top).into(),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Mm(right).into(),
                            y: Mm(bottom).into(),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Mm(left).into(),
                            y: Mm(bottom).into(),
                        },
                        bezier: false,
                    },
                ],
            }],
            mode: printpdf::PaintMode::Fill,
            winding_order: printpdf::WindingOrder::NonZero,
        },
    });
}

impl PdfRenderer {
    pub fn render_to_file(pages: &[RenderedPage], path: impl AsRef<Path>) -> Result<(), PdfError> {
        Self::render_to_file_with_base_dir(pages, path, ".")
    }

    pub fn render_to_file_with_base_dir(
        pages: &[RenderedPage],
        path: impl AsRef<Path>,
        base_dir: impl AsRef<Path>,
    ) -> Result<(), PdfError> {
        let mut document = PdfDocument::new("report-rs");
        let base_dir = base_dir.as_ref();

        let mut warnings = Vec::new();

        // ------------------------------------------------------------
        // Fonts
        // ------------------------------------------------------------

        let font_resolver = SystemFontResolver::new();
        let mut font_cache: HashMap<FontKey, FontId> = HashMap::new();
        let mut image_cache: HashMap<String, (XObjectId, u32, u32)> = HashMap::new();

        // ------------------------------------------------------------
        // Pages
        // ------------------------------------------------------------

        let mut pdf_pages = Vec::new();

        for page in pages {
            let mut ops = Vec::new();

            for item in &page.items {
                match item {
                    // ------------------------------------------------
                    // Text
                    // ------------------------------------------------
                    RenderedItem::Text {
                        x,
                        y,
                        font_size,
                        font_family,
                        bold,
                        italic,
                        text_color,
                        lines,
                        line_height,
                        width,
                        height,
                        horizontal_align,
                        vertical_align,
                        padding,
                        background,
                        border,
                        ..
                    } => {
                        // 1. BACKGROUND - primul
                        if let Some(background) = background {
                            draw_background(
                                &mut ops,
                                background,
                                x.0,
                                y.0,
                                width.0,
                                height.0,
                                page.height.0,
                            );
                        }

                        // 2. Culoarea textului
                        ops.push(Op::SetFillColor {
                            col: printpdf::Color::Rgb(printpdf::Rgb::new(
                                text_color.r as f32 / 255.0,
                                text_color.g as f32 / 255.0,
                                text_color.b as f32 / 255.0,
                                None,
                            )),
                        });

                        // 3. Fontul
                        let font_key = FontKey {
                            family: font_family.clone(),
                            bold: *bold,
                            italic: *italic,
                        };

                        let font_id = if let Some(font_id) = font_cache.get(&font_key) {
                            font_id.clone()
                        } else {
                            let font_spec = FontSpec {
                                family: font_family.clone(),
                                size: *font_size,
                                bold: *bold,
                                italic: *italic,
                            };

                            let resolved_font = font_resolver
                                .resolve(&font_spec)
                                .ok_or_else(|| PdfError::FontNotFound(font_family.clone()))?;

                            let parsed_font = ParsedFont::from_bytes(
                                &resolved_font.data,
                                resolved_font.face_index as usize,
                                &mut warnings,
                            )
                            .expect("Cannot parse resolved font");

                            let font_id = document.add_font(&parsed_font);
                            font_cache.insert(font_key, font_id.clone());
                            font_id
                        };

                        let content_left = x.0 + padding.left.0;

                        let content_right = x.0 + width.0 - padding.right.0;

                        let content_width = (content_right - content_left).max(0.0);

                        let content_top = y.0 + padding.top.0;

                        let content_height = (height.0 - padding.top.0 - padding.bottom.0).max(0.0);

                        let total_text_height = *line_height * lines.len() as f32;

                        let start_y = match vertical_align {
                            VerticalAlign::Top => content_top,

                            VerticalAlign::Center => {
                                content_top + (content_height - total_text_height) / 2.0
                            }

                            VerticalAlign::Bottom => {
                                content_top + content_height - total_text_height
                            }
                        };

                        for (index, line) in lines.iter().enumerate() {
                            let line_x = match horizontal_align {
                                HorizontalAlign::Left => content_left,

                                HorizontalAlign::Center => {
                                    content_left + (content_width - line.width) / 2.0
                                }

                                HorizontalAlign::Right => content_right - line.width,
                            };

                            let font_height_mm = pt_to_mm(*font_size);
                            let line_y = start_y + font_height_mm + *line_height * index as f32;
                            let pdf_y = page.height.0 - line_y;

                            ops.push(Op::StartTextSection);

                            ops.push(Op::SetFontSize {
                                size: Pt(*font_size),
                                font: font_id.clone(),
                            });

                            ops.push(Op::SetTextCursor {
                                pos: Point {
                                    x: Mm(line_x).into(),
                                    y: Mm(pdf_y).into(),
                                },
                            });

                            ops.push(Op::WriteText {
                                items: vec![TextItem::Text(line.text.clone())],
                                font: font_id.clone(),
                            });

                            ops.push(Op::EndTextSection);
                        }

                        // Border - inversam Y la preview-ul coordonatele inverse
                        if let Some(border) = border {
                            draw_border(
                                &mut ops,
                                border,
                                x.0,
                                y.0,
                                width.0,
                                height.0,
                                page.height.0,
                            );
                        };
                    }

                    // ------------------------------------------------
                    // Line
                    // ------------------------------------------------
                    RenderedItem::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        width,
                    } => {
                        let pdf_y1 = page.height.0 - y1.0;

                        let pdf_y2 = page.height.0 - y2.0;

                        let line = Line {
                            points: vec![
                                LinePoint {
                                    p: Point {
                                        x: Mm(x1.0).into(),
                                        y: Mm(pdf_y1).into(),
                                    },
                                    bezier: false,
                                },
                                LinePoint {
                                    p: Point {
                                        x: Mm(x2.0).into(),
                                        y: Mm(pdf_y2).into(),
                                    },
                                    bezier: false,
                                },
                            ],

                            is_closed: false,
                        };

                        ops.push(Op::SetOutlineThickness {
                            pt: Pt(mm_to_pt(width.0)),
                        });

                        ops.push(Op::DrawLine { line });
                    }

                    // ------------------------------------------------
                    // Rectangle
                    // ------------------------------------------------
                    RenderedItem::Rectangle {
                        x,
                        y,
                        width,
                        height,
                        border_width,
                    } => {
                        let left = x.0;
                        let right = x.0 + width.0;

                        let top = page.height.0 - y.0;

                        let bottom = page.height.0 - (y.0 + height.0);

                        let rect = Line {
                            points: vec![
                                LinePoint {
                                    p: Point {
                                        x: Mm(left).into(),
                                        y: Mm(top).into(),
                                    },
                                    bezier: false,
                                },
                                LinePoint {
                                    p: Point {
                                        x: Mm(right).into(),
                                        y: Mm(top).into(),
                                    },
                                    bezier: false,
                                },
                                LinePoint {
                                    p: Point {
                                        x: Mm(right).into(),
                                        y: Mm(bottom).into(),
                                    },
                                    bezier: false,
                                },
                                LinePoint {
                                    p: Point {
                                        x: Mm(left).into(),
                                        y: Mm(bottom).into(),
                                    },
                                    bezier: false,
                                },
                            ],

                            is_closed: true,
                        };

                        ops.push(Op::SetOutlineThickness {
                            pt: Pt(mm_to_pt(border_width.0)),
                        });

                        ops.push(Op::DrawLine { line: rect });
                    }

                    // ------------------------------------------------
                    // Image
                    // ------------------------------------------------
                    RenderedItem::Image {
                        x,
                        y,
                        width,
                        height,
                        source,
                        fit,
                    } => {
                        let (image_id, pixel_width, pixel_height) =
                            if let Some(image) = image_cache.get(source) {
                                image.clone()
                            } else {
                                let source_path = Path::new(source);
                                let resolved_path = if source_path.is_absolute() {
                                    source_path.to_path_buf()
                                } else {
                                    base_dir.join(source_path)
                                };
                                let image = load_image(resolved_path)?;
                                let pixel_width = image.width;
                                let pixel_height = image.height;
                                let raw_image = RawImage {
                                    pixels: RawImageData::U8(image.rgba),
                                    width: pixel_width as usize,
                                    height: pixel_height as usize,
                                    data_format: RawImageFormat::RGBA8,
                                    tag: source.as_bytes().to_vec(),
                                };
                                let image_id = document.add_image(&raw_image);
                                let cached = (image_id, pixel_width, pixel_height);

                                image_cache.insert(source.clone(), cached.clone());
                                cached
                            };

                        if pixel_width > 0 && pixel_height > 0 && width.0 > 0.0 && height.0 > 0.0 {
                            let placement = calculate_image_placement(
                                *x,
                                *y,
                                *width,
                                *height,
                                pixel_width,
                                pixel_height,
                                *fit,
                            );

                            ops.push(Op::UseXobject {
                                id: image_id,
                                transform: XObjectTransform {
                                    translate_x: Some(Pt(mm_to_pt(placement.x.0))),
                                    translate_y: Some(Pt(mm_to_pt(
                                        page.height.0 - placement.y.0 - placement.height.0,
                                    ))),
                                    rotate: None,
                                    scale_x: Some(mm_to_pt(placement.width.0) / pixel_width as f32),
                                    scale_y: Some(
                                        mm_to_pt(placement.height.0) / pixel_height as f32,
                                    ),
                                    dpi: Some(72.0),
                                },
                            });
                        }
                    }
                }
            }

            pdf_pages.push(PdfPage::new(Mm(page.width.0), Mm(page.height.0), ops));
        }

        // ------------------------------------------------------------
        // Save PDF
        // ------------------------------------------------------------

        let bytes = document
            .with_pages(pdf_pages)
            .save(&PdfSaveOptions::default(), &mut warnings);

        std::fs::write(path, bytes)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_renderer() {
        let _renderer = PdfRenderer;
    }
}
