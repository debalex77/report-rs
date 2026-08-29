use super::*;

pub(super) fn band_name(kind: &BandKind) -> &'static str {
    match kind {
        BandKind::ReportHeader => "ReportHeader",
        BandKind::PageHeader => "PageHeader",
        BandKind::Data { .. } => "DataBand",
        BandKind::PageFooter => "PageFooter",
        BandKind::ReportFooter => "ReportFooter",
    }
}

pub(super) fn band_colors(kind: &BandKind) -> (Color, Color) {
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

pub(super) fn alignment_button(
    icon: &'static [u8],
    selected: bool,
) -> iced::widget::Button<'static, Message> {
    button(
        svg(svg::Handle::from_memory(icon))
            .width(16)
            .height(16)
            .style(move |theme: &Theme, _status: svg::Status| svg::Style {
                color: Some(if selected {
                    Color::WHITE
                } else {
                    theme.palette().text
                }),
            }),
    )
    .width(36)
    .height(30)
    .style(move |theme, status| {
        let mut style = if selected {
            button::primary(theme, status)
        } else {
            button::secondary(theme, status)
        };
        style.border.radius = iced::border::radius(5);
        style
    })
}

pub(super) fn status_icon_button(
    icon: &'static [u8],
    selected: bool,
) -> iced::widget::Button<'static, Message> {
    button(
        svg(svg::Handle::from_memory(icon))
            .width(14)
            .height(14)
            .style(move |theme: &Theme, _status: svg::Status| svg::Style {
                color: Some(if selected {
                    Color::WHITE
                } else {
                    theme.palette().text
                }),
            }),
    )
    .width(30)
    .height(24)
    .padding(0)
    .style(move |theme, status| {
        let mut style = if selected {
            button::primary(theme, status)
        } else {
            button::secondary(theme, status)
        };
        style.border.radius = iced::border::radius(4);
        style
    })
}

pub(super) fn toolbar_separator() -> Element<'static, Message> {
    text("│")
        .size(18)
        .color(Color::from_rgba8(150, 155, 165, 0.55))
        .into()
}

pub(super) fn menu_button(
    label: &'static str,
    menu: AppMenu,
    open_menu: Option<AppMenu>,
) -> iced::widget::Button<'static, Message> {
    button(text(label).size(13))
        .height(28)
        .padding([4, 10])
        .style(if open_menu == Some(menu) {
            button::primary
        } else {
            button::text
        })
        .on_press(Message::ToggleMenu(menu))
}

pub(super) fn popup_menu_action(
    label: &'static str,
    message: Option<Message>,
) -> iced::widget::Button<'static, Message> {
    button(text(label).size(12))
        .width(Fill)
        .height(28)
        .padding([5, 10])
        .style(button::text)
        .on_press_maybe(message)
}

pub(super) fn popup_menu_action_owned(
    label: String,
    message: Option<Message>,
) -> iced::widget::Button<'static, Message> {
    button(text(label).size(12))
        .width(Fill)
        .height(28)
        .padding([5, 10])
        .style(button::text)
        .on_press_maybe(message)
}

pub(super) fn popup_menu_separator() -> Element<'static, Message> {
    container(Space::new().height(1))
        .width(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color,
            )),
            ..Default::default()
        })
        .into()
}

pub(super) fn popup_menu_style(theme: &Theme) -> container::Style {
    let mut background = theme.palette().background;
    background.a = 1.0;
    container::Style {
        background: Some(Background::Color(background)),
        border: iced::Border {
            color: theme.extended_palette().background.strong.color,
            width: 1.0,
            radius: iced::border::radius(6),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.35),
            offset: iced::Vector::new(0.0, 5.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}

pub(super) fn toolbox_button(
    icon: &'static [u8],
    label: &'static str,
    tool: DesignerTool,
) -> iced::widget::Button<'static, Message> {
    button(
        row![
            svg(svg::Handle::from_memory(icon))
                .width(16)
                .height(16)
                .style(|theme: &Theme, _status: svg::Status| svg::Style {
                    color: Some(theme.palette().text),
                }),
            text(label).size(12),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .height(30)
    .padding([5, 8])
    .style(button::secondary)
    .on_press(Message::UseTool(tool))
}

pub(super) fn toolbox_separator() -> Element<'static, Message> {
    container(Space::new().height(1))
        .width(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color,
            )),
            ..Default::default()
        })
        .into()
}

pub(super) fn dialog_container<'a>(
    content: impl Into<Element<'a, Message>>,
    width: f32,
) -> Element<'a, Message> {
    container(content).width(width).style(dialog_style).into()
}

pub(super) fn dialog_style(theme: &Theme) -> container::Style {
    let mut dialog_background = theme.palette().background;
    dialog_background.a = 1.0;
    container::Style {
        background: Some(Background::Color(dialog_background)),
        border: iced::Border {
            color: theme.extended_palette().background.strong.text,
            width: 1.0,
            radius: iced::border::radius(14),
        },
        text_color: Some(theme.palette().text),
        shadow: iced::Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.38),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}

pub(super) fn modal_backdrop_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.58))),
        ..Default::default()
    }
}

pub(super) fn spin_button(
    label: &'static str,
    message: Message,
) -> iced::widget::Button<'static, Message> {
    button(container(text(label).size(14)).center(Fill))
        .width(28)
        .height(28)
        .padding(0)
        .style(common::style_button(4.0))
        .on_press(message)
}

pub(super) fn property_group_header(
    label: &'static str,
    group: PropertyGroup,
    collapsed: bool,
) -> Element<'static, Message> {
    let marker = if collapsed { "▶" } else { "▼" };
    button(text(format!("{marker}  {label}")).size(14))
        .width(Fill)
        .padding([5, 8])
        .on_press(Message::ToggleGroup(group))
        .into()
}

pub(super) fn color_swatch_style(
    color: ReportColor,
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, _status| button::Style {
        background: Some(Background::Color(report_color_to_iced(color))),
        border: iced::Border {
            color: if selected {
                Color::from_rgb8(225, 80, 55)
            } else {
                Color::from_rgb8(95, 100, 110)
            },
            width: if selected { 3.0 } else { 1.0 },
            radius: iced::border::radius(4),
        },
        ..Default::default()
    }
}

pub(super) fn polar_point(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

pub(super) fn hsv_to_report_color(hue: f32, saturation: f32, value: f32) -> ReportColor {
    let chroma = value * saturation;
    let hue_segment = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (hue_segment.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hue_segment as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = value - chroma;

    ReportColor {
        r: ((r + m) * 255.0).round() as u8,
        g: ((g + m) * 255.0).round() as u8,
        b: ((b + m) * 255.0).round() as u8,
        a: 255,
    }
}

pub(super) fn report_color_to_hsv(color: ReportColor) -> (f32, f32, f32) {
    let r = color.r as f32 / 255.0;
    let g = color.g as f32 / 255.0;
    let b = color.b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };

    (hue, saturation, max)
}

pub(super) fn format_report_color(color: ReportColor) -> String {
    if color.a == 255 {
        format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
    } else {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            color.r, color.g, color.b, color.a
        )
    }
}

pub(super) fn parse_report_color(value: &str) -> Option<ReportColor> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if value.len() != 6 && value.len() != 8 {
        return None;
    }

    Some(ReportColor {
        r: u8::from_str_radix(&value[0..2], 16).ok()?,
        g: u8::from_str_radix(&value[2..4], 16).ok()?,
        b: u8::from_str_radix(&value[4..6], 16).ok()?,
        a: if value.len() == 8 {
            u8::from_str_radix(&value[6..8], 16).ok()?
        } else {
            255
        },
    })
}

pub(super) fn report_color_to_iced(color: ReportColor) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a as f32 / 255.0)
}

pub(super) fn designer_font(
    item: &report_core::model::TextItem,
    font_names: &HashMap<String, &'static str>,
) -> iced::Font {
    let family_name = item.font_family.to_ascii_lowercase();
    let family = if let Some(name) = font_names.get(&item.font_family) {
        iced::font::Family::Name(name)
    } else if family_name.contains("mono") {
        iced::font::Family::Monospace
    } else if family_name.contains("serif") && !family_name.contains("sans") {
        iced::font::Family::Serif
    } else {
        iced::font::Family::SansSerif
    };

    iced::Font {
        family,
        weight: if item.bold {
            iced::font::Weight::Bold
        } else {
            iced::font::Weight::Normal
        },
        style: if item.italic {
            iced::font::Style::Italic
        } else {
            iced::font::Style::Normal
        },
        ..iced::Font::DEFAULT
    }
}
