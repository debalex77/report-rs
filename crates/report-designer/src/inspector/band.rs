use super::*;

pub(super) fn append<'a>(
    app: &'a DesignerApp,
    band: &'a Band,
    mut content: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    content = content.push(property_group_header(
        "Band",
        PropertyGroup::Band,
        app.collapsed_groups.is_collapsed(PropertyGroup::Band),
    ));
    if app.collapsed_groups.is_collapsed(PropertyGroup::Band) {
        return content;
    }
    content = content
        .push(text(format!("Type: {}", band_name(&band.kind))).size(12))
        .push(
            row![
                text("Order (up/down)").size(11).width(112),
                alignment_button(
                    include_bytes!("../../../../assets/go-top-symbolic.svg"),
                    false,
                )
                .on_press_maybe(
                    app.active_band
                        .filter(|index| *index > 0)
                        .map(Message::MoveBandUp),
                ),
                alignment_button(
                    include_bytes!("../../../../assets/go-bottom-symbolic.svg"),
                    false,
                )
                .on_press_maybe(
                    app.active_band
                        .filter(|index| index + 1 < app.report.pages[0].bands.len())
                        .map(Message::MoveBandDown),
                ),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center),
        )
        .push(
            row![
                text("Height").size(11).width(62),
                spin_button("−", Message::BandHeightStep(-0.5)),
                text_input("mm", &app.band_inputs.height)
                    .width(78)
                    .size(12)
                    .padding(4)
                    .on_input(Message::BandHeightChanged),
                spin_button("+", Message::BandHeightStep(0.5)),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center),
        );
    if matches!(
        band.kind,
        BandKind::Data { .. }
            | BandKind::DataHeader { .. }
            | BandKind::GroupHeader { .. }
            | BandKind::GroupFooter { .. }
    ) {
        let queries = app
            .report
            .data_sources
            .iter()
            .flat_map(|source| source.queries.iter())
            .map(|query| query.name.clone())
            .collect::<Vec<_>>();
        content = content.push(text("Repeat query").size(11));
        if queries.is_empty() {
            content = content.push(text("Define a query in the Data tab first.").size(10));
        } else {
            let selected = (!app.band_inputs.data_source.is_empty())
                .then(|| app.band_inputs.data_source.clone());
            content = content.push(
                pick_list(queries, selected, Message::BandDataSourceChanged)
                    .placeholder("Select query")
                    .text_size(12)
                    .padding(4),
            );
        }
    }
    if matches!(
        band.kind,
        BandKind::GroupHeader { .. } | BandKind::GroupFooter { .. }
    ) {
        let fields = app
            .query_fields
            .get(&app.band_inputs.data_source)
            .cloned()
            .unwrap_or_default();
        content = content.push(text("Group field").size(11));
        if fields.is_empty() {
            content =
                content.push(text("Expand the query in the Data tab to load fields.").size(10));
        } else {
            content = content.push(
                pick_list(
                    fields,
                    (!app.band_inputs.group_field.is_empty())
                        .then(|| app.band_inputs.group_field.clone()),
                    Message::GroupFieldChanged,
                )
                .placeholder("Select group field")
                .text_size(12)
                .padding(4),
            );
        }
    }
    if let BandKind::DataHeader {
        repeat_on_each_page,
        ..
    } = band.kind
    {
        content = content.push(
            container(
                toggler(repeat_on_each_page)
                    .label("Repeat on each page")
                    .text_size(11)
                    .size(18)
                    .spacing(8)
                    .on_toggle(Message::DataHeaderRepeatChanged),
            )
            .padding([5, 7])
            .width(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(
                    theme.extended_palette().background.weak.color,
                )),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
    }
    if let BandKind::GroupHeader {
        repeat_on_each_page,
        ..
    } = band.kind
    {
        content = content.push(
            container(
                toggler(repeat_on_each_page)
                    .label("Repeat group header on each page")
                    .text_size(11)
                    .size(18)
                    .spacing(8)
                    .on_toggle(Message::GroupHeaderRepeatChanged),
            )
            .padding([5, 7])
            .width(Fill),
        );
    }
    content.push(text("Choose an item tool to insert it in this band.").size(11))
}
