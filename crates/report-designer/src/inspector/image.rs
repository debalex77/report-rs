use super::*;

pub(super) fn append<'a>(
    app: &'a DesignerApp,
    item: &'a Item,
    mut content: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    if let Item::Image(image_item) = item {
        content = content.push(property_group_header(
            "Image",
            PropertyGroup::Image,
            app.collapsed_groups.is_collapsed(PropertyGroup::Image),
        ));
        if !app.collapsed_groups.is_collapsed(PropertyGroup::Image) {
            content = content.push(
                row![
                    text("Source type").size(11).width(72),
                    pick_list(
                        vec!["File".to_string(), "Database BLOB".to_string()],
                        Some(
                            if image_item.source_type == ImageSourceType::Database {
                                "Database BLOB"
                            } else {
                                "File"
                            }
                            .to_string(),
                        ),
                        Message::ImageSourceTypeChanged,
                    )
                    .width(Fill)
                    .text_size(12)
                    .padding(4),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            );
            if image_item.source_type == ImageSourceType::File {
                content = content.push(text("Source").size(11)).push(
                    row![
                        text_input("Image path", &image_item.source)
                            .size(12)
                            .padding(4)
                            .on_input(Message::ImageSourceChanged),
                        button(text("Browse…").size(11))
                            .height(26)
                            .padding([3, 6])
                            .style(common::style_button(5.0))
                            .on_press(Message::BrowseImageSource),
                    ]
                    .spacing(5)
                    .align_y(iced::Alignment::Center),
                );
            } else {
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
                        .filter(|query| Some(query.as_str()) != main_query),
                );
                let selected_query = match &image_item.query_source {
                    QuerySource::Main => "Main Query".to_string(),
                    QuerySource::Named(query) => query.clone(),
                };
                let effective_query = match &image_item.query_source {
                    QuerySource::Main => main_query,
                    QuerySource::Named(query) => Some(query.as_str()),
                };
                let fields = effective_query
                    .and_then(|query| app.query_fields.get(query))
                    .cloned()
                    .unwrap_or_default();
                content = content
                    .push(
                        row![
                            text("Query").size(11).width(72),
                            pick_list(
                                queries,
                                Some(selected_query),
                                Message::ImageQuerySourceChanged,
                            )
                            .width(Fill)
                            .text_size(12)
                            .padding(4),
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                    )
                    .push(
                        row![
                            text("BLOB field").size(11).width(72),
                            pick_list(
                                fields,
                                image_item.field.clone(),
                                Message::ImageFieldChanged,
                            )
                            .width(Fill)
                            .text_size(12)
                            .padding(4),
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                    );
            }
            content = content.push(text("Fit").size(11)).push(
                row![
                    button(text("Contain").size(11))
                        .style(if image_item.fit == ImageFit::Contain {
                            button::primary
                        } else {
                            button::secondary
                        })
                        .on_press(Message::ImageFitChanged(ImageFit::Contain)),
                    button(text("Stretch").size(11))
                        .style(if image_item.fit == ImageFit::Stretch {
                            button::primary
                        } else {
                            button::secondary
                        })
                        .on_press(Message::ImageFitChanged(ImageFit::Stretch)),
                ]
                .spacing(5),
            );
        }
    }

    content
}
