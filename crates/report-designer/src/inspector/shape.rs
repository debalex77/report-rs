use super::*;

pub(super) fn append<'a>(
    app: &'a DesignerApp,
    item: &'a Item,
    mut content: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    if !matches!(item, Item::Rectangle(_)) {
        return content;
    }
    content = content.push(property_group_header(
        "Shape",
        PropertyGroup::Shape,
        app.collapsed_groups.is_collapsed(PropertyGroup::Shape),
    ));
    if !app.collapsed_groups.is_collapsed(PropertyGroup::Shape) {
        content = content.push(
            row![
                text("Border width").size(11).width(72),
                spin_button("−", Message::ShapeBorderWidthStep(-0.1)),
                text_input("mm", &app.shape_inputs.border_width)
                    .width(78)
                    .size(12)
                    .padding(4)
                    .on_input(Message::ShapeBorderWidthChanged),
                spin_button("+", Message::ShapeBorderWidthStep(0.1)),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center),
        );
    }
    content
}
