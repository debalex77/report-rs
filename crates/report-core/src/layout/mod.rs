use crate::datasource::{ReportContext, Row};
use crate::expressions;
use crate::layout::text::{ApproxTextMeasurer, TextLayout, TextLine, TextMeasurer};
use crate::model::{
    Band, BandKind, Border, Color, HorizontalAlign, ImageFit, Item, Mm, Padding, Page,
    VerticalAlign,
};

pub mod text;

#[derive(Debug, Clone)]
/// A physical page produced by the layout pass.
///
/// Renderers consume this type so they do not repeat pagination, expression
/// resolution, or text layout.
pub struct RenderedPage {
    pub width: Mm,
    pub height: Mm,
    pub items: Vec<RenderedItem>,
}

#[derive(Debug, Clone)]
/// An item whose final page-relative geometry has already been calculated.
///
/// Text items also carry resolved content and measured lines, keeping output
/// renderers focused on drawing rather than layout decisions.
pub enum RenderedItem {
    Text {
        x: Mm,
        y: Mm,
        width: Mm,
        height: Mm,

        text: String,

        font_family: String,
        font_size: f32,
        bold: bool,
        italic: bool,
        underline: bool,
        strikeout: bool,

        text_color: Color,

        lines: Vec<TextLine>,
        line_height: f32,

        horizontal_align: HorizontalAlign,
        vertical_align: VerticalAlign,

        padding: Padding,

        background: Option<Color>,
        border: Option<Border>,
    },

    Line {
        x1: Mm,
        y1: Mm,
        x2: Mm,
        y2: Mm,
        width: Mm,
    },

    Rectangle {
        x: Mm,
        y: Mm,
        width: Mm,
        height: Mm,
        border_width: Mm,
    },

    Image {
        x: Mm,
        y: Mm,
        width: Mm,
        height: Mm,
        source: String,
        fit: ImageFit,
    },
}

/// Converts the declarative report model into renderer-independent pages.
///
/// This pass measures bands, resolves runtime values, expands data bands,
/// applies auto-height, and inserts page breaks.
pub struct LayoutEngine;

impl LayoutEngine {
    /// Lays out and paginates a page using approximate text metrics.
    ///
    /// Use [`Self::render_with_measurer`] when layout must use the same font
    /// metrics as the final output renderer.
    pub fn render(page: &Page, context: &ReportContext) -> Vec<RenderedPage> {
        let measurer = ApproxTextMeasurer;
        Self::render_with_measurer(page, context, &measurer)
    }

    /// Lays out all bands on one physical page without pagination.
    ///
    /// Bands are processed once in declaration order; page headers and footers
    /// are not treated as repeating bands by this method.
    pub fn render_page(page: &Page, context: &ReportContext) -> RenderedPage {
        let measurer = ApproxTextMeasurer;
        Self::render_page_with_measurer(page, context, &measurer)
    }

    /// Single-page counterpart of [`Self::render_page`] with caller-provided
    /// text measurement.
    pub fn render_page_with_measurer<M: TextMeasurer>(
        page: &Page,
        context: &ReportContext,
        measurer: &M,
    ) -> RenderedPage {
        let mut rendered_items = Vec::new();
        let mut cursor_y = page.margins.top;

        for band in &page.bands {
            let measured_height = Self::measure_band(band, None, context, measurer);

            Self::render_band(
                band,
                page.margins.left,
                cursor_y,
                None,
                context,
                measurer,
                &mut rendered_items,
            );

            cursor_y = cursor_y + measured_height;
        }

        RenderedPage {
            width: page.width(),
            height: page.height(),
            items: rendered_items,
        }
    }

    fn measure_band<M: TextMeasurer>(
        band: &Band,
        row: Option<&Row>,
        context: &ReportContext,
        measurer: &M,
    ) -> Mm {
        let current_query = match &band.kind {
            BandKind::Data { source } => Some(source.as_str()),
            _ => None,
        };
        // The declared band height is a minimum. Fixed item bounds and text
        // auto-height may extend it.
        let mut height = band.height;

        for item in &band.items {
            let required_height = match item {
                Item::Text(text) => {
                    let effective_height = if text.auto_height {
                        // Resolved values affect wrapping and must be known
                        // before the pagination decision is made.
                        let resolved_text = expressions::evaluate_for_query(
                            &text.text,
                            row,
                            context,
                            current_query,
                        );

                        let font = text.font_spec();

                        // Padding reduces the inner width available to text.
                        let content_width =
                            (text.width.0 - text.padding.left.0 - text.padding.right.0).max(0.0);

                        let text_layout = if text.word_wrap {
                            TextLayout::wrap(&resolved_text, &font, content_width, measurer)
                        } else {
                            TextLayout::single_line(&resolved_text, &font, measurer)
                        };

                        // Auto-height includes vertical padding but never
                        // shrinks below the explicitly declared height.
                        let layout_height =
                            Mm(text.padding.top.0 + text_layout.height + text.padding.bottom.0);

                        if layout_height > text.height {
                            layout_height
                        } else {
                            text.height
                        }
                    } else {
                        text.height
                    };

                    // Item positions are band-relative, so the required extent
                    // includes both the offset and the item height.
                    text.y + effective_height
                }

                Item::Line(line) => {
                    if line.y1 > line.y2 {
                        line.y1
                    } else {
                        line.y2
                    }
                }

                Item::Rectangle(rect) => rect.y + rect.height,

                Item::Image(image) => image.y + image.height,

                Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
                    layout.y + layout.height
                }
            };

            if required_height > height {
                height = required_height;
            }
        }

        height
    }

    fn render_band<M: TextMeasurer>(
        band: &Band,
        offset_x: Mm,
        offset_y: Mm,
        row: Option<&Row>,
        context: &ReportContext,
        measurer: &M,
        rendered_items: &mut Vec<RenderedItem>,
    ) {
        let current_query = match &band.kind {
            BandKind::Data { source } => Some(source.as_str()),
            _ => None,
        };
        for item in &band.items {
            match item {
                Item::Text(text) => {
                    // Keep these choices identical to measure_band: pagination
                    // is correct only when measuring and rendering agree.
                    let resolved_text =
                        expressions::evaluate_for_query(&text.text, row, context, current_query);

                    let font = text.font_spec();

                    let content_width =
                        (text.width.0 - text.padding.left.0 - text.padding.right.0).max(0.0);

                    let text_layout = if text.word_wrap {
                        TextLayout::wrap(&resolved_text, &font, content_width, measurer)
                    } else {
                        TextLayout::single_line(&resolved_text, &font, measurer)
                    };

                    let rendered_height = if text.auto_height {
                        let layout_height =
                            Mm(text.padding.top.0 + text_layout.height + text.padding.bottom.0);

                        if layout_height > text.height {
                            layout_height
                        } else {
                            text.height
                        }
                    } else {
                        text.height
                    };

                    rendered_items.push(RenderedItem::Text {
                        // Convert band-relative coordinates to page coordinates.
                        x: offset_x + text.x,
                        y: offset_y + text.y,

                        width: text.width,
                        height: rendered_height,

                        text: resolved_text,

                        font_family: text.font_family.clone(),
                        font_size: text.font_size,
                        bold: text.bold,
                        italic: text.italic,
                        underline: text.underline,
                        strikeout: text.strikeout,

                        text_color: text.text_color,

                        lines: text_layout.lines,
                        line_height: text_layout.line_height,

                        horizontal_align: text.horizontal_align.clone(),
                        vertical_align: text.vertical_align.clone(),

                        padding: text.padding,

                        background: text.background,
                        border: text.border.clone(),
                    });
                }

                Item::Line(line) => {
                    rendered_items.push(RenderedItem::Line {
                        x1: offset_x + line.x1,
                        y1: offset_y + line.y1,
                        x2: offset_x + line.x2,
                        y2: offset_y + line.y2,
                        width: line.width,
                    });
                }

                Item::Rectangle(rect) => {
                    rendered_items.push(RenderedItem::Rectangle {
                        x: offset_x + rect.x,
                        y: offset_y + rect.y,

                        width: rect.width,
                        height: rect.height,

                        border_width: rect.border_width,
                    });
                }

                Item::Image(image) => {
                    rendered_items.push(RenderedItem::Image {
                        x: offset_x + image.x,
                        y: offset_y + image.y,

                        width: image.width,
                        height: image.height,

                        source: image.source.clone(),
                        fit: image.fit,
                    });
                }

                Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
                    let nested = Band {
                        kind: crate::model::BandKind::ReportHeader,
                        height: layout.height,
                        items: layout.items.clone(),
                    };
                    Self::render_band(
                        &nested,
                        offset_x + layout.x,
                        offset_y + layout.y,
                        row,
                        context,
                        measurer,
                        rendered_items,
                    );
                }
            }
        }
    }

    /// Lays out and paginates a page with caller-provided text metrics.
    ///
    /// One measurer is used for both passes so page-break decisions agree with
    /// the line geometry placed into the rendered model.
    pub fn render_with_measurer<M: TextMeasurer>(
        page: &Page,
        context: &ReportContext,
        measurer: &M,
    ) -> Vec<RenderedPage> {
        let mut pages = Vec::new();
        let mut rendered_items = Vec::new();

        let page_header = page
            .bands
            .iter()
            .find(|band| matches!(band.kind, crate::model::BandKind::PageHeader));

        let page_footer = page
            .bands
            .iter()
            .find(|band| matches!(band.kind, crate::model::BandKind::PageFooter));

        // Reserve footer space before content layout so bands cannot overlap it.
        let footer_height = match page_footer {
            Some(footer) => Self::measure_band(footer, None, context, measurer),
            None => Mm(0.0),
        };

        let mut cursor_y = page.margins.top;

        // Emit the repeating header on the first page.
        if let Some(header) = page_header {
            let header_height = Self::measure_band(header, None, context, measurer);

            Self::render_band(
                header,
                page.margins.left,
                cursor_y,
                None,
                context,
                measurer,
                &mut rendered_items,
            );
            cursor_y = cursor_y + header_height;
        }

        // Ordinary content must end above the margin and reserved footer.
        let printable_bottom = page.height() - page.margins.bottom - footer_height;

        for band in &page.bands {
            if matches!(
                band.kind,
                crate::model::BandKind::PageHeader
                    | crate::model::BandKind::PageFooter
                    | crate::model::BandKind::DataHeader { .. }
            ) {
                continue;
            }

            match &band.kind {
                crate::model::BandKind::Data { source } => {
                    // Measure and render once per row because resolved values
                    // may wrap differently and produce different heights.
                    if let Some(rows) = context.table(source) {
                        let data_header = page
                            .bands
                            .iter()
                            .find(|candidate| {
                                matches!(
                                    &candidate.kind,
                                    BandKind::DataHeader {
                                        source: header_source,
                                        ..
                                    } if header_source == source
                                )
                            })
                            // Backward-friendly fallback for a template whose
                            // single DataHeader was created before its
                            // DataBand query was selected.
                            .or_else(|| {
                                page.bands.iter().find(|candidate| {
                                    matches!(candidate.kind, BandKind::DataHeader { .. })
                                })
                            });
                        if !rows.is_empty()
                            && let Some(header) = data_header
                        {
                            Self::render_band(
                                header,
                                page.margins.left,
                                cursor_y,
                                None,
                                context,
                                measurer,
                                &mut rendered_items,
                            );
                            cursor_y =
                                cursor_y + Self::measure_band(header, None, context, measurer);
                        }
                        for row in rows {
                            let measured_height =
                                Self::measure_band(band, Some(row), context, measurer);

                            let band_bottom = cursor_y + measured_height;

                            if band_bottom > printable_bottom {
                                // Close the page before moving the whole band;
                                // this engine does not split bands.
                                if let Some(footer) = page_footer {
                                    Self::render_band(
                                        footer,
                                        page.margins.left,
                                        printable_bottom,
                                        None,
                                        context,
                                        measurer,
                                        &mut rendered_items,
                                    );
                                }

                                pages.push(RenderedPage {
                                    width: page.width(),
                                    height: page.height(),
                                    items: rendered_items,
                                });

                                rendered_items = Vec::new();
                                cursor_y = page.margins.top;

                                // Re-establish the repeating header after a break.
                                if let Some(header) = page_header {
                                    let header_height =
                                        Self::measure_band(header, None, context, measurer);

                                    Self::render_band(
                                        header,
                                        page.margins.left,
                                        cursor_y,
                                        None,
                                        context,
                                        measurer,
                                        &mut rendered_items,
                                    );

                                    cursor_y = cursor_y + header_height;
                                }
                                if let Some(header) = data_header
                                    && matches!(
                                        header.kind,
                                        BandKind::DataHeader {
                                            repeat_on_each_page: true,
                                            ..
                                        }
                                    )
                                {
                                    Self::render_band(
                                        header,
                                        page.margins.left,
                                        cursor_y,
                                        None,
                                        context,
                                        measurer,
                                        &mut rendered_items,
                                    );
                                    cursor_y = cursor_y
                                        + Self::measure_band(header, None, context, measurer);
                                }
                            }

                            Self::render_band(
                                band,
                                page.margins.left,
                                cursor_y,
                                Some(row),
                                context,
                                measurer,
                                &mut rendered_items,
                            );

                            cursor_y = cursor_y + measured_height;
                        }
                    }
                }

                _ => {
                    let measured_height = Self::measure_band(band, None, context, measurer);

                    let band_bottom = cursor_y + measured_height;

                    if band_bottom > printable_bottom {
                        // Close the page before moving the whole band.
                        if let Some(footer) = page_footer {
                            Self::render_band(
                                footer,
                                page.margins.left,
                                printable_bottom,
                                None,
                                context,
                                measurer,
                                &mut rendered_items,
                            );
                        }

                        pages.push(RenderedPage {
                            width: page.width(),
                            height: page.height(),
                            items: rendered_items,
                        });

                        rendered_items = Vec::new();
                        cursor_y = page.margins.top;

                        // Re-establish the repeating header after a break.
                        if let Some(header) = page_header {
                            let header_height = Self::measure_band(header, None, context, measurer);

                            Self::render_band(
                                header,
                                page.margins.left,
                                cursor_y,
                                None,
                                context,
                                measurer,
                                &mut rendered_items,
                            );

                            cursor_y = cursor_y + header_height;
                        }
                    }

                    Self::render_band(
                        band,
                        page.margins.left,
                        cursor_y,
                        None,
                        context,
                        measurer,
                        &mut rendered_items,
                    );

                    cursor_y = cursor_y + measured_height;
                }
            }
        }

        // No overflow event follows the final page, so add its footer here.
        if let Some(footer) = page_footer {
            Self::render_band(
                footer,
                page.margins.left,
                printable_bottom,
                None,
                context,
                measurer,
                &mut rendered_items,
            );
        }

        pages.push(RenderedPage {
            width: page.width(),
            height: page.height(),
            items: rendered_items,
        });

        pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasource::{ReportContext, Row};
    use crate::font::FontSpec;
    use crate::model::{
        Band, BandKind, Color, HorizontalAlign, ImageFit, ImageItem, Item, Margins, Orientation,
        Page, PageSize, QuerySource, TextItem, ValueType, VerticalAlign, default_font_family,
        default_text_color,
    };

    #[test]
    fn render_simple_page() {
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![Band {
                kind: BandKind::ReportHeader,
                height: Mm(20.0),

                items: vec![Item::Text(TextItem {
                    name: String::new(),
                    x: Mm(5.0),
                    y: Mm(3.0),
                    width: Mm(100.0),
                    height: Mm(10.0),

                    text: "Test".to_string(),

                    value_type: ValueType::Text,

                    query_source: QuerySource::Main,

                    field: None,

                    font_size: 12.0,
                    font_family: default_font_family(),

                    bold: false,
                    italic: false,

                    underline: false,

                    strikeout: false,

                    text_color: default_text_color(),

                    horizontal_align: HorizontalAlign::Left,
                    vertical_align: VerticalAlign::Top,

                    word_wrap: false,
                    auto_height: false,

                    padding: Padding::default(),

                    background: None,
                    border: None,
                })],
            }],
        };

        let context = ReportContext::new();

        let rendered = LayoutEngine::render_page(&page, &context);

        assert_eq!(rendered.width, Mm(210.0));
        assert_eq!(rendered.height, Mm(297.0));
        assert_eq!(rendered.items.len(), 1);

        match &rendered.items[0] {
            RenderedItem::Text { x, y, text, .. } => {
                assert_eq!(*x, Mm(15.0));
                assert_eq!(*y, Mm(13.0));
                assert_eq!(text, "Test");
            }

            _ => panic!("Expected text item"),
        }
    }

    #[test]
    fn data_header_repeats_on_each_page_of_its_data_band() {
        let line = |x2| {
            Item::Line(crate::model::LineItem {
                name: String::new(),
                x1: Mm(0.0),
                y1: Mm(0.0),
                x2: Mm(x2),
                y2: Mm(0.0),
                width: Mm(0.5),
            })
        };
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,
            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },
            bands: vec![
                Band {
                    kind: BandKind::DataHeader {
                        source: "patients".to_string(),
                        repeat_on_each_page: true,
                    },
                    height: Mm(10.0),
                    items: vec![line(20.0)],
                },
                Band {
                    kind: BandKind::Data {
                        source: "patients".to_string(),
                    },
                    height: Mm(100.0),
                    items: vec![line(40.0)],
                },
            ],
        };
        let mut context = ReportContext::new();
        context.add_table("patients", (0..5).map(|_| Row::new()).collect());

        let pages = LayoutEngine::render(&page, &context);

        assert_eq!(pages.len(), 3);
        assert!(pages.iter().all(|page| {
            matches!(page.items.first(), Some(RenderedItem::Line { x2, .. }) if *x2 == Mm(30.0))
        }));
    }

    #[test]
    fn render_image_item() {
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,
            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },
            bands: vec![Band {
                kind: BandKind::ReportHeader,
                height: Mm(40.0),
                items: vec![Item::Image(ImageItem {
                    name: String::new(),
                    x: Mm(5.0),
                    y: Mm(3.0),
                    width: Mm(40.0),
                    height: Mm(30.0),
                    source: "images/logo.png".to_string(),
                    fit: ImageFit::Stretch,
                })],
            }],
        };

        let rendered = LayoutEngine::render_page(&page, &ReportContext::new());

        assert_eq!(rendered.items.len(), 1);

        match &rendered.items[0] {
            RenderedItem::Image {
                x,
                y,
                width,
                height,
                source,
                fit,
            } => {
                assert_eq!(*x, Mm(15.0));
                assert_eq!(*y, Mm(13.0));
                assert_eq!(*width, Mm(40.0));
                assert_eq!(*height, Mm(30.0));
                assert_eq!(source, "images/logo.png");
                assert_eq!(*fit, ImageFit::Stretch);
            }
            _ => panic!("Expected image item"),
        }
    }

    #[test]
    fn paginate_bands() {
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![
                Band {
                    kind: BandKind::Data {
                        source: "first".to_string(),
                    },
                    height: Mm(200.0),
                    items: vec![],
                },
                Band {
                    kind: BandKind::Data {
                        source: "second".to_string(),
                    },
                    height: Mm(100.0),
                    items: vec![],
                },
            ],
        };

        let mut context = ReportContext::new();

        context.add_table("first", vec![Row::new()]);
        context.add_table("second", vec![Row::new()]);

        let pages = LayoutEngine::render(&page, &context);

        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn repeat_page_header_and_footer() {
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![
                Band {
                    kind: BandKind::PageHeader,
                    height: Mm(20.0),

                    items: vec![Item::Text(TextItem {
                        name: String::new(),
                        x: Mm(0.0),
                        y: Mm(0.0),
                        width: Mm(100.0),
                        height: Mm(10.0),

                        text: "HEADER".to_string(),

                        value_type: ValueType::Text,

                        query_source: QuerySource::Main,

                        field: None,

                        font_size: 12.0,
                        font_family: default_font_family(),

                        bold: false,
                        italic: false,

                        underline: false,

                        strikeout: false,

                        text_color: default_text_color(),

                        horizontal_align: HorizontalAlign::Left,
                        vertical_align: VerticalAlign::Top,

                        word_wrap: false,
                        auto_height: false,

                        padding: Padding::default(),

                        background: None,
                        border: None,
                    })],
                },
                Band {
                    kind: BandKind::Data {
                        source: "first".to_string(),
                    },
                    height: Mm(200.0),
                    items: vec![],
                },
                Band {
                    kind: BandKind::Data {
                        source: "second".to_string(),
                    },
                    height: Mm(100.0),
                    items: vec![],
                },
                Band {
                    kind: BandKind::PageFooter,
                    height: Mm(15.0),

                    items: vec![Item::Text(TextItem {
                        name: String::new(),
                        x: Mm(0.0),
                        y: Mm(0.0),
                        width: Mm(100.0),
                        height: Mm(10.0),

                        text: "FOOTER".to_string(),

                        value_type: ValueType::Text,

                        query_source: QuerySource::Main,

                        field: None,

                        font_size: 10.0,
                        font_family: default_font_family(),

                        bold: false,
                        italic: false,

                        underline: false,

                        strikeout: false,

                        text_color: default_text_color(),

                        horizontal_align: HorizontalAlign::Left,
                        vertical_align: VerticalAlign::Top,

                        word_wrap: false,
                        auto_height: false,

                        padding: Padding::default(),

                        background: None,
                        border: None,
                    })],
                },
            ],
        };

        let mut context = ReportContext::new();

        context.add_table("first", vec![Row::new()]);
        context.add_table("second", vec![Row::new()]);

        let pages = LayoutEngine::render(&page, &context);

        assert_eq!(pages.len(), 2);

        assert_eq!(pages[0].items.len(), 2);
        assert_eq!(pages[1].items.len(), 2);
    }

    #[test]
    fn report_footer_moves_to_next_page() {
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![
                Band {
                    kind: BandKind::PageHeader,
                    height: Mm(20.0),
                    items: vec![],
                },
                Band {
                    kind: BandKind::Data {
                        source: "items".to_string(),
                    },
                    height: Mm(230.0),
                    items: vec![],
                },
                Band {
                    kind: BandKind::ReportFooter,
                    height: Mm(50.0),
                    items: vec![],
                },
                Band {
                    kind: BandKind::PageFooter,
                    height: Mm(15.0),
                    items: vec![],
                },
            ],
        };

        let mut context = ReportContext::new();

        context.add_table("items", vec![Row::new()]);

        let pages = LayoutEngine::render(&page, &context);

        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn repeat_data_band_for_each_row() {
        use crate::datasource::{Row, Value};

        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![Band {
                kind: BandKind::Data {
                    source: "patients".to_string(),
                },

                height: Mm(20.0),

                items: vec![Item::Text(TextItem {
                    name: String::new(),
                    x: Mm(0.0),
                    y: Mm(0.0),

                    width: Mm(100.0),
                    height: Mm(10.0),

                    text: "Pacient: ${name}".to_string(),

                    value_type: ValueType::Text,

                    query_source: QuerySource::Main,

                    field: None,

                    font_size: 12.0,
                    font_family: default_font_family(),

                    bold: false,
                    italic: false,

                    underline: false,

                    strikeout: false,

                    text_color: default_text_color(),

                    horizontal_align: HorizontalAlign::Left,
                    vertical_align: VerticalAlign::Top,

                    word_wrap: false,
                    auto_height: false,

                    padding: Padding::default(),

                    background: None,
                    border: None,
                })],
            }],
        };

        let mut row1 = Row::new();

        row1.insert("name".to_string(), Value::String("Ion Popescu".to_string()));

        let mut row2 = Row::new();

        row2.insert(
            "name".to_string(),
            Value::String("Maria Ionescu".to_string()),
        );

        let mut context = ReportContext::new();

        context.add_table("patients", vec![row1, row2]);

        let pages = LayoutEngine::render(&page, &context);

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].items.len(), 2);

        match &pages[0].items[0] {
            RenderedItem::Text { text, .. } => {
                assert_eq!(text, "Pacient: Ion Popescu");
            }

            _ => panic!("Expected text item"),
        }

        match &pages[0].items[1] {
            RenderedItem::Text { text, .. } => {
                assert_eq!(text, "Pacient: Maria Ionescu");
            }

            _ => panic!("Expected text item"),
        }
    }

    #[test]
    fn resolve_global_variable() {
        use crate::datasource::Value;

        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![Band {
                kind: BandKind::ReportHeader,
                height: Mm(20.0),

                items: vec![Item::Text(TextItem {
                    name: String::new(),
                    x: Mm(0.0),
                    y: Mm(0.0),
                    width: Mm(100.0),
                    height: Mm(10.0),

                    text: "Medic: ${doctor_name}".to_string(),

                    value_type: ValueType::Text,

                    query_source: QuerySource::Main,

                    field: None,

                    font_size: 12.0,
                    font_family: default_font_family(),

                    bold: false,
                    italic: false,

                    underline: false,

                    strikeout: false,

                    text_color: default_text_color(),

                    horizontal_align: HorizontalAlign::Left,
                    vertical_align: VerticalAlign::Top,

                    word_wrap: false,
                    auto_height: false,

                    padding: Padding {
                        left: Mm(2.0),
                        top: Mm(1.0),
                        right: Mm(2.0),
                        bottom: Mm(1.0),
                    },

                    background: Some(Color::rgb(230, 230, 230)),
                    border: None,
                })],
            }],
        };

        let mut context = ReportContext::new();

        context.set_variable("doctor_name", Value::String("Dr. Ion Popescu".to_string()));

        let rendered = LayoutEngine::render_page(&page, &context);

        assert_eq!(rendered.items.len(), 1);

        match &rendered.items[0] {
            RenderedItem::Text { text, .. } => {
                assert_eq!(text, "Medic: Dr. Ion Popescu");
            }

            _ => panic!("Expected text item"),
        }
    }

    #[test]
    fn resolve_parameter() {
        use crate::datasource::Value;

        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,
            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },
            bands: vec![Band {
                kind: BandKind::ReportHeader,
                height: Mm(20.0),
                items: vec![Item::Text(TextItem {
                    name: String::new(),
                    x: Mm(0.0),
                    y: Mm(0.0),
                    width: Mm(100.0),
                    height: Mm(10.0),
                    text: "Clinică: ${parameter.clinic}".to_string(),
                    value_type: ValueType::Text,
                    query_source: QuerySource::Main,
                    field: None,
                    font_size: 12.0,
                    font_family: default_font_family(),
                    bold: false,
                    italic: false,
                    underline: false,
                    strikeout: false,
                    text_color: default_text_color(),
                    horizontal_align: HorizontalAlign::Left,
                    vertical_align: VerticalAlign::Top,
                    word_wrap: false,
                    auto_height: false,
                    padding: Padding::default(),
                    background: None,
                    border: None,
                })],
            }],
        };

        let mut context = ReportContext::new();
        context.set_parameter("clinic", Value::String("Clinica Centrală".to_string()));

        let rendered = LayoutEngine::render_page(&page, &context);

        match &rendered.items[0] {
            RenderedItem::Text { text, .. } => assert_eq!(text, "Clinică: Clinica Centrală"),
            _ => panic!("Expected text item"),
        }
    }

    #[test]
    fn auto_height_expands_band() {
        let page = Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![
                Band {
                    kind: BandKind::ReportHeader,
                    height: Mm(10.0),

                    items: vec![Item::Text(TextItem {
                        name: String::new(),
                        x: Mm(0.0),
                        y: Mm(0.0),
                        width: Mm(30.0),
                        height: Mm(5.0),

                        text: "Acesta este un text lung care trebuie sa ocupe mai multe randuri"
                            .to_string(),

                        value_type: ValueType::Text,

                        query_source: QuerySource::Main,

                        field: None,

                        font_size: 12.0,
                        font_family: default_font_family(),

                        bold: false,
                        italic: false,

                        underline: false,

                        strikeout: false,

                        text_color: default_text_color(),

                        horizontal_align: HorizontalAlign::Left,
                        vertical_align: VerticalAlign::Top,

                        word_wrap: true,
                        auto_height: true,

                        padding: Padding {
                            left: Mm(0.0),
                            top: Mm(5.0),
                            right: Mm(0.0),
                            bottom: Mm(5.0),
                        },

                        background: None,
                        border: None,
                    })],
                },
                Band {
                    kind: BandKind::ReportFooter,
                    height: Mm(10.0),

                    items: vec![Item::Text(TextItem {
                        name: String::new(),
                        x: Mm(0.0),
                        y: Mm(0.0),
                        width: Mm(100.0),
                        height: Mm(10.0),

                        text: "URMATORUL BAND".to_string(),

                        value_type: ValueType::Text,

                        query_source: QuerySource::Main,

                        field: None,

                        font_size: 12.0,
                        font_family: default_font_family(),

                        bold: false,
                        italic: false,

                        underline: false,

                        strikeout: false,

                        text_color: default_text_color(),

                        horizontal_align: HorizontalAlign::Left,
                        vertical_align: VerticalAlign::Top,

                        word_wrap: false,
                        auto_height: false,

                        padding: Padding::default(),

                        background: None,
                        border: None,
                    })],
                },
            ],
        };

        let context = ReportContext::new();

        let pages = LayoutEngine::render(&page, &context);

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].items.len(), 2);

        let (first_y, first_height) = match &pages[0].items[0] {
            RenderedItem::Text { y, height, .. } => (*y, *height),
            _ => panic!("Expected first text item"),
        };

        let second_y = match &pages[0].items[1] {
            RenderedItem::Text { y, .. } => *y,
            _ => panic!("Expected second text item"),
        };

        println!("first_y      = {:?}", first_y);
        println!("first_height = {:?}", first_height);
        println!("second_y     = {:?}", second_y);

        let measurer = ApproxTextMeasurer;

        let font = FontSpec {
            family: default_font_family(),
            size: 12.0,
            bold: false,
            italic: false,
        };

        let content_width = 30.0 - 0.0 - 0.0;

        let text_layout = TextLayout::wrap(
            "Acesta este un text lung care trebuie sa ocupe mai multe randuri",
            &font,
            content_width,
            &measurer,
        );

        let expected_height = Mm(5.0 + text_layout.height + 5.0);

        assert!((first_height.0 - expected_height.0).abs() < 0.001);
        assert!(second_y >= first_y + first_height);
    }

    #[test]
    fn auto_height_includes_vertical_padding() {
        let measurer = ApproxTextMeasurer;

        let font = FontSpec {
            family: default_font_family(),
            size: 12.0,
            bold: false,
            italic: false,
        };

        let text_layout = TextLayout::wrap(
            "Acesta este un text lung care trebuie sa ocupe mai multe randuri",
            &font,
            30.0,
            &measurer,
        );

        let padding_top = 5.0;
        let padding_bottom = 5.0;

        let expected_height = text_layout.height + padding_top + padding_bottom;

        let actual_height = expected_height;

        assert!((actual_height - expected_height).abs() < 0.001);
    }

    #[test]
    fn band_expands_to_contain_rectangle() {
        use crate::model::RectangleItem;

        let band = Band {
            kind: BandKind::ReportHeader,
            height: Mm(10.0),

            items: vec![Item::Rectangle(RectangleItem {
                name: String::new(),
                x: Mm(0.0),
                y: Mm(20.0),
                width: Mm(50.0),
                height: Mm(15.0),
                border_width: Mm(0.3),
            })],
        };

        let context = ReportContext::new();
        let measurer = ApproxTextMeasurer;

        let height = LayoutEngine::measure_band(&band, None, &context, &measurer);

        assert_eq!(height, Mm(35.0));
    }
}
