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
                "Function",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
            let band_query = app
                .selection
                .and_then(|selection| app.report.pages.first()?.bands.get(selection.band))
                .and_then(|band| match &band.kind {
                    BandKind::Data { source } => Some(source.as_str()),
                    _ => None,
                });
            let mut report_queries = app
                .report
                .data_sources
                .iter()
                .flat_map(|source| source.queries.iter());
            let first_query = report_queries.next().map(|query| query.name.as_str());
            let only_query = first_query.filter(|_| report_queries.next().is_none());
            let main_query = band_query.or(only_query);
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
            let editor: Element<'_, Message> = mouse_area(
                text_editor(&app.text_inputs.text)
                    .placeholder("Text / Value")
                    .size(12)
                    .padding(6)
                    .height(84)
                    .on_action(Message::TextEdited),
            )
            .on_right_press(Message::OpenQueryTextMenu(QueryTextTarget::ItemText))
            .into();
            if matches!(
                text_item.value_type,
                ValueType::Integer
                    | ValueType::Double
                    | ValueType::Boolean
                    | ValueType::Date
                    | ValueType::DateTime
                    | ValueType::Expression
            ) {
                let label = if text_item.value_type == ValueType::Expression {
                    "Expression"
                } else {
                    "Field / Value"
                };
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
                content = content.push(text(label).size(11));
                content = content.push(stack![editor, delegate]);
            } else if text_item.value_type == ValueType::Function {
                let delegate = container(
                    button(container(text("•••").size(9)).center(Fill))
                        .width(30)
                        .height(26)
                        .padding(0)
                        .style(common::style_button(5.0))
                        .on_press(Message::OpenFunctionPicker),
                )
                .padding(4)
                .align_right(Fill)
                .align_top(Fill);
                content = content
                    .push(text("Function").size(11))
                    .push(stack![editor, delegate]);
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
                mouse_area(
                    text_input("dd.MM.yyyy", &app.text_inputs.date_pattern)
                        .id(app.date_pattern_input_id.clone())
                        .size(11)
                        .padding(4)
                        .on_input(Message::ValueFormatDatePatternChanged),
                )
                .on_right_press(Message::OpenQueryTextMenu(QueryTextTarget::DatePattern)),
            );
        }
        content = content.push(text("Prefix / suffix").size(11)).push(
            row![
                mouse_area(
                    text_input("Prefix", &app.text_inputs.value_prefix)
                        .id(app.value_prefix_input_id.clone())
                        .size(11)
                        .padding(4)
                        .on_input(Message::ValueFormatPrefixChanged)
                )
                .on_right_press(Message::OpenQueryTextMenu(QueryTextTarget::ValuePrefix)),
                mouse_area(
                    text_input("Suffix", &app.text_inputs.value_suffix)
                        .id(app.value_suffix_input_id.clone())
                        .size(11)
                        .padding(4)
                        .on_input(Message::ValueFormatSuffixChanged)
                )
                .on_right_press(Message::OpenQueryTextMenu(QueryTextTarget::ValueSuffix)),
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
                mouse_area(
                    text_input("#RRGGBB", &app.text_inputs.text_color)
                        .id(app.text_color_input_id.clone())
                        .size(12)
                        .padding(4)
                        .on_input(Message::TextColorChanged),
                )
                .on_right_press(Message::OpenQueryTextMenu(QueryTextTarget::TextColor)),
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
        ValueType::Function => "Function",
    }
}

impl DesignerApp {
    pub(crate) fn function_picker_dialog(&self) -> Element<'_, Message> {
        let mut available = vec![(
            "Row number".to_string(),
            "Sequential number of the current DataBand row".to_string(),
            "${row_number}".to_string(),
        )];
        let mut query_names = self
            .report
            .data_sources
            .iter()
            .flat_map(|source| source.queries.iter().map(|query| query.name.clone()))
            .collect::<Vec<_>>();
        query_names.sort();
        query_names.dedup();
        for query in query_names {
            available.push((
                format!("Count · {query}"),
                format!("Number of rows returned by {query}"),
                format!("${{count({query})}}"),
            ));
            for field in self.query_fields.get(&query).into_iter().flatten() {
                if !matches!(
                    self.query_field_types.get(&(query.clone(), field.clone())),
                    Some(ValueType::Integer | ValueType::Double)
                ) {
                    continue;
                }
                for (label, function, description) in [
                    ("Sum", "sum", "Sum of all numeric values"),
                    ("Average", "average", "Average of all numeric values"),
                    ("Minimum", "min", "Smallest numeric value"),
                    ("Maximum", "max", "Largest numeric value"),
                ] {
                    available.push((
                        format!("{label} · {query}.{field}"),
                        description.to_string(),
                        format!("${{{function}({query}.{field})}}"),
                    ));
                }
            }
        }

        let mut functions = iced::widget::column![].spacing(6);
        for (label, description, expression) in available {
            functions = functions.push(
                button(container(
                    row![
                        iced::widget::column![
                            text(label).size(13),
                            text(description)
                                .size(10)
                                .color(Color::from_rgb8(155, 160, 170)),
                        ]
                        .spacing(2)
                        .width(Fill),
                        container(text(expression.clone()).size(11))
                            .padding([4, 8])
                            .style(|theme: &Theme| container::Style {
                                background: Some(Background::Color(
                                    theme.extended_palette().background.weak.color,
                                )),
                                border: iced::Border {
                                    radius: iced::border::radius(5),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }),
                    ]
                    .spacing(16)
                    .align_y(iced::Alignment::Center),
                ))
                .width(Fill)
                .padding([8, 10])
                .style(common::style_button(7.0))
                .on_press(Message::SelectFunction(expression)),
            );
        }
        dialog_container(
            iced::widget::column![
                text("Select function").size(20),
                text("Choose a built-in value to insert at the cursor position.")
                    .size(11)
                    .color(Color::from_rgb8(155, 160, 170)),
                rule::horizontal(1),
                scrollable(functions).height(360),
                row![
                    Space::new().width(Fill),
                    button(
                        container(text("Cancel").size(12))
                            .width(Fill)
                            .center_x(Fill),
                    )
                    .width(88)
                    .height(30)
                    .padding(0)
                    .style(common::style_button(6.0))
                    .on_press(Message::CloseFunctionPicker),
                ]
            ]
            .spacing(14)
            .padding(20),
            620.0,
        )
    }
}
