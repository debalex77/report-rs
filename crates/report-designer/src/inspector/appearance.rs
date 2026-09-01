use super::*;

pub(super) fn append<'a>(
    app: &'a DesignerApp,
    item: &'a Item,
    mut content: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    let Some(text_item) = first_text_item(item) else {
        return content;
    };
    content = content.push(property_group_header(
        "Padding / Background / Border",
        PropertyGroup::Appearance,
        app.collapsed_groups.is_collapsed(PropertyGroup::Appearance),
    ));
    if app.collapsed_groups.is_collapsed(PropertyGroup::Appearance) {
        return content;
    }

    content = content
        .push(text("Padding (mm)").size(11))
        .push(padding_row(app, "Left", PaddingField::Left))
        .push(padding_row(app, "Top", PaddingField::Top))
        .push(padding_row(app, "Right", PaddingField::Right))
        .push(padding_row(app, "Bottom", PaddingField::Bottom));

    let background_enabled = text_item.background.is_some();
    content = content.push(
        toggler(background_enabled)
            .label("Background")
            .text_size(11)
            .size(16)
            .spacing(7)
            .on_toggle(Message::BackgroundEnabled),
    );
    if background_enabled {
        content = content.push(
            text_input("#RRGGBB", &app.text_inputs.background)
                .size(12)
                .padding(4)
                .on_input(Message::BackgroundColorChanged),
        );
        let mut palette = row![].spacing(2);
        for color in TEXT_COLOR_PALETTE {
            palette = palette.push(
                button(text(""))
                    .width(22)
                    .height(20)
                    .padding(0)
                    .style(color_swatch_style(
                        color,
                        text_item.background == Some(color),
                    ))
                    .on_press(Message::BackgroundColorSelected(color)),
            );
        }
        content = content
            .push(palette)
            .push(text("Custom background color").size(11))
            .push(
                container(
                    Canvas::new(ColorWheel {
                        selected: text_item.background.unwrap_or(ReportColor::WHITE),
                        target: ColorTarget::Background,
                    })
                    .width(140)
                    .height(140),
                )
                .width(Fill)
                .center_x(Fill),
            );
    }

    let border_enabled = text_item.border.is_some();
    content = content.push(
        toggler(border_enabled)
            .label("Border")
            .text_size(11)
            .size(16)
            .spacing(7)
            .on_toggle(Message::BorderEnabled),
    );
    if let Some(border) = &text_item.border {
        content = content
            .push(
                row![
                    side_button("L", BorderSide::Left, border.left),
                    side_button("T", BorderSide::Top, border.top),
                    side_button("R", BorderSide::Right, border.right),
                    side_button("B", BorderSide::Bottom, border.bottom),
                ]
                .spacing(4),
            )
            .push(
                row![
                    text("Width").size(11).width(46),
                    spin_button("−", Message::BorderWidthStep(-0.1)),
                    text_input("mm", &app.text_inputs.border_width)
                        .width(78)
                        .size(12)
                        .padding(4)
                        .on_input(Message::BorderWidthChanged),
                    spin_button("+", Message::BorderWidthStep(0.1)),
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center),
            );
    }
    content
}

fn padding_row<'a>(
    app: &'a DesignerApp,
    label: &'static str,
    field: PaddingField,
) -> Element<'a, Message> {
    row![
        text(label).size(11).width(46),
        spin_button("−", Message::PaddingStep(field, -0.5)),
        text_input("mm", app.text_inputs.padding(field))
            .width(78)
            .size(12)
            .padding(4)
            .on_input(move |value| Message::PaddingChanged(field, value)),
        spin_button("+", Message::PaddingStep(field, 0.5)),
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center)
    .into()
}

fn side_button(
    label: &'static str,
    side: BorderSide,
    enabled: bool,
) -> iced::widget::Button<'static, Message> {
    button(text(label).size(11))
        .width(30)
        .height(24)
        .style(if enabled {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::BorderSideChanged(side, !enabled))
}
