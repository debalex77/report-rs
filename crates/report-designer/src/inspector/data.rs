use super::*;

pub(super) fn view(app: &DesignerApp) -> Element<'_, Message> {
    let mut content = iced::widget::column![
        row![
            text("Data sources").size(16),
            Space::new().width(Fill),
            button(container(text("+").size(14)).center(Fill))
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
                button(container(text("Query").size(11)).center(Fill))
                    .width(58)
                    .height(26)
                    .padding(0)
                    .style(common::style_button(6.0))
                    .on_press(Message::NewDataQuery(index)),
                Space::new().width(3),
                button(container(text("Edit").size(11)).center(Fill))
                    .width(48)
                    .height(26)
                    .padding(0)
                    .style(common::style_button(6.0))
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
                        button(text("Filter / Sorting").size(10))
                            .padding([3, 6])
                            .style(button::text)
                            .on_press(Message::OpenQueryRules {
                                source: index,
                                query: query_index,
                            }),
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
                            ]
                            .spacing(4)
                            .align_y(iced::Alignment::Center),
                        );
                    }
                    let query_name = query.name.clone();
                    let templates_query = query.name.clone();
                    let actions = row![
                        Space::new().width(Fill),
                        tooltip(
                            button(container(text("Templates").size(10)).center(Fill))
                                .width(78)
                                .height(25)
                                .padding(0)
                                .style(common::style_button(6.0))
                                .on_press(Message::ToggleDataTemplates(templates_query)),
                            text("Choose a saved table template").size(10),
                            tooltip::Position::Bottom,
                        ),
                        tooltip(
                            button(container(text("Generate").size(10)).center(Fill))
                                .width(70)
                                .height(25)
                                .padding(0)
                                .style(common::style_button(6.0))
                                .on_press(Message::GenerateDataFields(query_name)),
                            text("Generate a table from the selected query fields").size(10),
                            tooltip::Position::Bottom,
                        ),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center);
                    source_content = source_content.push(actions);
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

pub(crate) fn templates_popup(
    app: &DesignerApp,
    query: &str,
    position: Point,
) -> Element<'static, Message> {
    let mut actions = iced::widget::column![].spacing(1).width(iced::Shrink);
    if app.table_templates.is_empty() {
        actions = actions.push(container(text("No saved templates").size(11)).padding([6, 10]));
    }
    for (index, template) in app.table_templates.iter().enumerate() {
        actions = actions.push(
            button(text(template.name.clone()).size(11))
                .height(28)
                .padding([5, 10])
                .style(button::text)
                .on_press(Message::GenerateDataFieldsWithTemplate(
                    query.to_string(),
                    index,
                )),
        );
    }
    let popup = container(opaque(
        container(actions).padding(4).style(popup_menu_style),
    ))
    .padding(iced::Padding {
        top: position.y.max(0.0),
        right: 0.0,
        bottom: 0.0,
        left: position.x.max(0.0),
    })
    .align_x(iced::alignment::Horizontal::Left)
    .align_y(iced::alignment::Vertical::Top)
    .width(Fill)
    .height(Fill);
    stack![
        mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::CloseDataTemplates),
        popup,
    ]
    .into()
}
