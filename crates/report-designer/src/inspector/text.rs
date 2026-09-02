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
            let value_types = [
                "Text",
                "Integer",
                "Double",
                "Boolean",
                "Date",
                "DateTime",
                "Expression",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
            let main_query = app
                .selection
                .and_then(|selection| app.report.pages.first()?.bands.get(selection.band))
                .and_then(|band| match &band.kind {
                    BandKind::Data { source } => Some(source.as_str()),
                    _ => None,
                });
            let mut queries = vec!["Main Query".to_string()];
            queries.extend(
                app.report
                    .data_sources
                    .iter()
                    .flat_map(|source| source.queries.iter())
                    .map(|query| query.name.clone())
                    .filter(|name| Some(name.as_str()) != main_query),
            );
            let selected_query = match &text_item.query_source {
                QuerySource::Main => "Main Query".to_string(),
                QuerySource::Named(name) => name.clone(),
            };
            let resolved_query = match &text_item.query_source {
                QuerySource::Main => main_query,
                QuerySource::Named(name) => Some(name.as_str()),
            };
            content = content
                .push(
                    row![
                        text("Value type").size(11).width(72),
                        pick_list(
                            value_types,
                            Some(value_type_name(text_item.value_type).to_string()),
                            Message::ValueTypeChanged,
                        )
                        .width(Fill)
                        .text_size(12)
                        .padding(4)
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                )
                .push(
                    row![
                        text("Query").size(11).width(72),
                        pick_list(queries, Some(selected_query), Message::QuerySourceChanged)
                            .width(Fill)
                            .text_size(12)
                            .padding(4)
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                );
            let editor = text_editor(&app.text_inputs.text)
                .placeholder("Text / Value")
                .size(12)
                .padding(6)
                .height(84)
                .on_action(Message::TextEdited);
            if text_item.value_type == ValueType::Expression {
                content = content.push(text("Expression").size(11));
                if resolved_query.is_some() {
                    let delegate = container(
                        button(container(text("•••").size(9)).center(Fill))
                            .width(30)
                            .height(26)
                            .padding(0)
                            .style(common::style_button(5.0))
                            .on_press(Message::OpenQueryFieldPicker),
                    )
                    .padding(4)
                    .align_right(Fill)
                    .align_top(Fill);
                    content = content.push(stack![editor, delegate]);
                } else {
                    content = content.push(editor);
                }
            } else {
                content = content.push(text("Text / Value").size(11)).push(editor);
            }
        }
        content = content.push(
            row![
                toggler(text_item.word_wrap)
                    .label("Word wrap")
                    .text_size(11)
                    .size(16)
                    .spacing(7)
                    .on_toggle(Message::WordWrapChanged),
                toggler(text_item.auto_height)
                    .label("Auto height")
                    .text_size(11)
                    .size(16)
                    .spacing(7)
                    .on_toggle(Message::AutoHeightChanged),
            ]
            .spacing(14),
        );
    }

    content = content.push(property_group_header(
        "Value Format",
        PropertyGroup::ValueFormat,
        app.collapsed_groups
            .is_collapsed(PropertyGroup::ValueFormat),
    ));
    if !app
        .collapsed_groups
        .is_collapsed(PropertyGroup::ValueFormat)
    {
        if matches!(
            text_item.value_type,
            ValueType::Integer | ValueType::Double | ValueType::Expression
        ) {
            content = content
                .push(text("Decimal places").size(11))
                .push(
                    text_input("Default", &app.text_inputs.decimal_places)
                        .width(92)
                        .size(11)
                        .padding(4)
                        .on_input(Message::ValueFormatDecimalChanged),
                )
                .push(
                    toggler(text_item.value_format.grouping)
                        .label("Group digits")
                        .text_size(11)
                        .size(16)
                        .spacing(7)
                        .on_toggle(Message::ValueFormatGroupingChanged),
                );
        }
        if matches!(
            text_item.value_type,
            ValueType::Date | ValueType::DateTime | ValueType::Expression
        ) {
            content = content.push(text("Date pattern").size(11)).push(
                text_input("dd.MM.yyyy", &app.text_inputs.date_pattern)
                    .size(11)
                    .padding(4)
                    .on_input(Message::ValueFormatDatePatternChanged),
            );
        }
        content = content.push(text("Prefix / suffix").size(11)).push(
            row![
                text_input("Prefix", &app.text_inputs.value_prefix)
                    .size(11)
                    .padding(4)
                    .on_input(Message::ValueFormatPrefixChanged),
                text_input("Suffix", &app.text_inputs.value_suffix)
                    .size(11)
                    .padding(4)
                    .on_input(Message::ValueFormatSuffixChanged),
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
                    alignment_button(
                        include_bytes!("../../../../assets/format-text-underline-symbolic.svg"),
                        text_item.underline,
                    )
                    .on_press(Message::UnderlineChanged(!text_item.underline)),
                    alignment_button(
                        include_bytes!("../../../../assets/format-text-strikethrough-symbolic.svg"),
                        text_item.strikeout,
                    )
                    .on_press(Message::StrikeoutChanged(!text_item.strikeout)),
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

fn value_type_name(value_type: ValueType) -> &'static str {
    match value_type {
        ValueType::Text => "Text",
        ValueType::Integer => "Integer",
        ValueType::Double => "Double",
        ValueType::Boolean => "Boolean",
        ValueType::Date => "Date",
        ValueType::DateTime => "DateTime",
        ValueType::Expression => "Expression",
    }
}
