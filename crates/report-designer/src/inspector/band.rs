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
    if matches!(band.kind, BandKind::Data { .. }) {
        content = content.push(text("Data source").size(11)).push(
            text_input("Dataset name", &app.band_inputs.data_source)
                .size(12)
                .padding(4)
                .on_input(Message::BandDataSourceChanged),
        );
    }
    content.push(text("Choose an item tool to insert it in this band.").size(11))
}
