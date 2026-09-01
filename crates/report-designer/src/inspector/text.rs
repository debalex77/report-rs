use super::*;

pub(super) fn append<'a>(
    app: &'a DesignerApp,
    item: &'a Item,
    mut content: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    let Some(text_item) = first_text_item(item) else {
        return content;
    };
    let direct_text = matches!(item, Item::Text(_));
    if !direct_text {
        content = content.push(
            text("Changes apply to all text items in this layout.")
                .size(10)
                .color(Color::from_rgb8(150, 155, 165)),
        );
    }
    content = content.push(property_group_header(
        "Text / Value",
        PropertyGroup::TextValue,
        app.collapsed_groups.is_collapsed(PropertyGroup::TextValue),
    ));
    if !app.collapsed_groups.is_collapsed(PropertyGroup::TextValue) {
        if direct_text {
            content = content.push(
                text_editor(&app.text_inputs.text)
                    .placeholder("Text / Value")
                    .size(12)
                    .padding(6)
                    .height(84)
                    .on_action(Message::TextEdited),
            );
        }
        content = content.push(
            row![
                button(text("Word wrap").size(11))
                    .style(if text_item.word_wrap {
                        button::primary
                    } else {
                        button::secondary
                    })
                    .on_press(Message::WordWrapChanged(!text_item.word_wrap)),
                button(text("Auto height").size(11))
                    .style(if text_item.auto_height {
                        button::primary
                    } else {
                        button::secondary
                    })
                    .on_press(Message::AutoHeightChanged(!text_item.auto_height)),
            ]
            .spacing(5),
        );
    }

    content = content.push(property_group_header(
        "Font Family",
        PropertyGroup::Font,
        app.collapsed_groups.is_collapsed(PropertyGroup::Font),
    ));
    if !app.collapsed_groups.is_collapsed(PropertyGroup::Font) {
        content = content
            .push(
                row![
                    text("Size").size(11).width(46),
                    spin_button("−", Message::FontSizeStep(-1.0)),
                    text_input("pt", &app.text_inputs.font_size)
                        .width(78)
                        .size(12)
                        .padding(4)
                        .on_input(Message::FontSizeChanged),
                    spin_button("+", Message::FontSizeStep(1.0)),
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center),
            )
            .push(
                combo_box(
                    &app.font_families,
                    "Font family",
                    Some(&app.text_inputs.font_family),
                    Message::FontFamilyChanged,
                )
                .size(12)
                .padding(4)
                .on_input(Message::FontFamilyChanged),
            )
            .push(
                row![
                    alignment_button(
                        include_bytes!("../../../../assets/format-text-bold-symbolic.svg"),
                        text_item.bold,
                    )
                    .on_press(Message::BoldChanged(!text_item.bold)),
                    alignment_button(
                        include_bytes!("../../../../assets/format-text-italic-symbolic.svg"),
                        text_item.italic,
                    )
                    .on_press(Message::ItalicChanged(!text_item.italic)),
                ]
                .spacing(4),
            );
    }

    content = content.push(property_group_header(
        "Text Color",
        PropertyGroup::TextColor,
        app.collapsed_groups.is_collapsed(PropertyGroup::TextColor),
    ));

    let mut palette_top = row![].spacing(2);
    let mut palette_bottom = row![].spacing(2);
    for (index, color) in TEXT_COLOR_PALETTE.into_iter().enumerate() {
        let color_button = button(text(""))
            .width(28)
            .height(24)
            .padding(0)
            .style(color_swatch_style(color, color == text_item.text_color))
            .on_press(Message::TextColorSelected(color));
        if index < 5 {
            palette_top = palette_top.push(color_button);
        } else {
            palette_bottom = palette_bottom.push(color_button);
        }
    }

    if !app.collapsed_groups.is_collapsed(PropertyGroup::TextColor) {
        content = content
            .push(
                text_input("#RRGGBB", &app.text_inputs.text_color)
                    .size(12)
                    .padding(4)
                    .on_input(Message::TextColorChanged),
            )
            .push(iced::widget::column![palette_top, palette_bottom].spacing(4))
            .push(text("Custom color").size(11))
            .push(
                container(
                    Canvas::new(ColorWheel {
                        selected: text_item.text_color,
                        target: ColorTarget::Text,
                    })
                    .width(140)
                    .height(140),
                )
                .width(Fill)
                .center_x(Fill),
            );
    }

    content = content.push(property_group_header(
        "Alignment",
        PropertyGroup::Alignment,
        app.collapsed_groups.is_collapsed(PropertyGroup::Alignment),
    ));
    if !app.collapsed_groups.is_collapsed(PropertyGroup::Alignment) {
        content = content
            .push(text("Horizontal").size(11))
            .push(
                row![
                    alignment_button(
                        include_bytes!("../../../../assets/format-justify-left-symbolic.svg"),
                        matches!(text_item.horizontal_align, HorizontalAlign::Left),
                    )
                    .on_press(Message::HorizontalAlignChanged(HorizontalAlign::Left)),
                    alignment_button(
                        include_bytes!("../../../../assets/format-justify-center-symbolic.svg"),
                        matches!(text_item.horizontal_align, HorizontalAlign::Center),
                    )
                    .on_press(Message::HorizontalAlignChanged(HorizontalAlign::Center)),
                    alignment_button(
                        include_bytes!("../../../../assets/format-justify-right-symbolic.svg"),
                        matches!(text_item.horizontal_align, HorizontalAlign::Right),
                    )
                    .on_press(Message::HorizontalAlignChanged(HorizontalAlign::Right)),
                ]
                .spacing(4),
            )
            .push(text("Vertical").size(11))
            .push(
                row![
                    alignment_button(
                        include_bytes!("../../../../assets/go-top-symbolic.svg"),
                        matches!(text_item.vertical_align, VerticalAlign::Top),
                    )
                    .on_press(Message::VerticalAlignChanged(VerticalAlign::Top)),
                    alignment_button(
                        include_bytes!(
                            "../../../../assets/format-align-vertical-center-symbolic.svg"
                        ),
                        matches!(text_item.vertical_align, VerticalAlign::Center),
                    )
                    .on_press(Message::VerticalAlignChanged(VerticalAlign::Center)),
                    alignment_button(
                        include_bytes!("../../../../assets/go-bottom-symbolic.svg"),
                        matches!(text_item.vertical_align, VerticalAlign::Bottom),
                    )
                    .on_press(Message::VerticalAlignChanged(VerticalAlign::Bottom)),
                ]
                .spacing(4),
            );
    }
    content
}
