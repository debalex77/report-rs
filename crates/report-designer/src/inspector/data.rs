use super::*;

pub(super) fn view(app: &DesignerApp) -> Element<'_, Message> {
    let mut content = iced::widget::column![
        row![
            text("Data sources").size(16),
            Space::new().width(Fill),
            button(text("+").size(14))
                .width(28)
                .height(26)
                .padding(0)
                .style(common::style_button(6.0))
                .on_press(Message::NewDataSource),
        ]
        .align_y(iced::Alignment::Center),
    ]
    .spacing(6);

    if app.report.data_sources.is_empty() {
        content = content.push(text("No data sources. Add a SQLite connection.").size(11));
    }
    for (index, source) in app.report.data_sources.iter().enumerate() {
        let DataConnection::Sqlite { path } = &source.connection;
        let mut source_content = iced::widget::column![
            row![
                text(format!("▾  {}", source.name)).size(12),
                Space::new().width(Fill),
                button(text("+ Query").size(11))
                    .padding([3, 7])
                    .style(button::secondary)
                    .on_press(Message::NewDataQuery(index)),
                button(text("Edit").size(11))
                    .padding([3, 7])
                    .style(button::secondary)
                    .on_press(Message::EditDataSource(index)),
            ]
            .align_y(iced::Alignment::Center),
            text(format!("SQLite: {}", truncate(path, 36))).size(10),
        ]
        .spacing(3);
        if source.queries.is_empty() {
            source_content = source_content.push(text("    No queries").size(10));
        } else {
            for (query_index, query) in source.queries.iter().enumerate() {
                let expanded = app.expanded_data_queries.contains(&query.name);
                source_content = source_content.push(
                    row![
                        button(text(if expanded { "▾" } else { "▸" }).size(10))
                            .width(24)
                            .padding([3, 5])
                            .style(button::text)
                            .on_press(Message::ToggleDataQueryFields {
                                source: index,
                                query: query_index,
                            }),
                        text(format!("◇ {}", query.name)).size(11).width(Fill),
                        button(text("Edit").size(10))
                            .padding([3, 6])
                            .style(button::text)
                            .on_press(Message::EditDataQuery {
                                source: index,
                                query: query_index,
                            }),
                    ]
                    .align_y(iced::Alignment::Center),
                );
                if expanded {
                    let fields = app
                        .query_fields
                        .get(&query.name)
                        .cloned()
                        .unwrap_or_default();
                    if fields.is_empty() {
                        source_content = source_content.push(text("      No fields").size(10));
                    }
                    for field in fields {
                        let key = (query.name.clone(), field.clone());
                        let selected = app.selected_data_fields.contains(&key);
                        let query_name = query.name.clone();
                        let field_name = field.clone();
                        let drag_field_name = field.clone();
                        source_content = source_content.push(
                            row![
                                Space::new().width(25),
                                checkbox(selected)
                                    .label(field)
                                    .size(14)
                                    .text_size(11)
                                    .spacing(6)
                                    .on_toggle(move |_| Message::ToggleDataField {
                                        query: query_name.clone(),
                                        field: field_name.clone(),
                                    }),
                                mouse_area(
                                    container(text("⠿").size(11))
                                        .width(18)
                                        .height(18)
                                        .align_x(iced::Alignment::Center)
                                        .align_y(iced::Alignment::Center)
                                        .style(|theme: &Theme| container::Style {
                                            background: Some(Background::Color(
                                                theme.extended_palette().background.weak.color,
                                            )),
                                            border: iced::Border {
                                                color: theme
                                                    .extended_palette()
                                                    .background
                                                    .strong
                                                    .color,
                                                width: 1.0,
                                                radius: 4.0.into(),
                                                ..Default::default()
                                            },
                                            text_color: Some(
                                                theme.extended_palette().primary.strong.color,
                                            ),
                                            ..Default::default()
                                        }),
                                )
                                .on_press(Message::BeginDataFieldDrag {
                                    query: query.name.clone(),
                                    field: drag_field_name,
                                })
                                .interaction(mouse::Interaction::Grab),
                            ]
                            .spacing(4)
                            .align_y(iced::Alignment::Center),
                        );
                    }
                }
            }
        }
        content = content.push(container(source_content).padding(7).width(Fill).style(
            |theme: &Theme| container::Style {
                border: iced::Border {
                    color: theme.extended_palette().background.strong.color,
                    width: 1.0,
                    radius: iced::border::radius(7),
                },
                ..Default::default()
            },
        ));
    }

    scrollable(content.padding(5)).height(Fill).into()
}
