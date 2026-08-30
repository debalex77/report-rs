use super::*;

pub(super) fn append<'a>(
    app: &'a DesignerApp,
    item: &'a Item,
    mut content: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    content = content.push(property_group_header(
        "Geometry",
        PropertyGroup::Geometry,
        app.collapsed_groups.is_collapsed(PropertyGroup::Geometry),
    ));
    if !app.collapsed_groups.is_collapsed(PropertyGroup::Geometry) {
        if app
            .selection
            .is_some_and(|selection| !selection.is_top_level())
        {
            content = content.push(
                text("Geometry is controlled by the parent layout.")
                    .size(11)
                    .color(Color::from_rgb8(150, 155, 165)),
            );
        } else {
            for (label, field) in geometry_field_specs(item) {
                content = content.push(
                    row![
                        text(label).size(11).width(46),
                        spin_button("−", Message::GeometryStep(field, -0.10)),
                        text_input("mm", app.geometry_inputs.value(field))
                            .width(78)
                            .size(12)
                            .padding(4)
                            .on_input(move |value| Message::GeometryChanged(field, value)),
                        spin_button("+", Message::GeometryStep(field, 0.10)),
                    ]
                    .spacing(5)
                    .align_y(iced::Alignment::Center),
                );
            }
        }
    }

    content
}
