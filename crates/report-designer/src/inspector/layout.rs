use super::*;

pub(super) fn append<'a>(
    app: &'a DesignerApp,
    item: &'a Item,
    mut content: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    let (layout, direction) = match item {
        Item::HorizontalLayout(layout) => (layout, "Horizontal"),
        Item::VerticalLayout(layout) => (layout, "Vertical"),
        _ => return content,
    };
    content = content.push(property_group_header(
        "Layout",
        PropertyGroup::Layout,
        app.collapsed_groups.is_collapsed(PropertyGroup::Layout),
    ));
    if app.collapsed_groups.is_collapsed(PropertyGroup::Layout) {
        return content;
    }
    content = content
        .push(text(format!("Direction: {direction}")).size(11))
        .push(text(format!("Children: {}", layout.items.len())).size(11));
    for (index, child) in layout.items.iter().enumerate() {
        let (_, _, width, height) = normalized_geometry(child);
        content = content.push(
            text(format!(
                "{}. {} — {:.2} × {:.2} mm",
                index + 1,
                item_name(child),
                width,
                height
            ))
            .size(10),
        );
    }
    content.push(
        button(text("Equalize children").size(11))
            .padding([3, 6])
            .style(common::style_button(5.0))
            .on_press_maybe((!layout.items.is_empty()).then_some(Message::EqualizeLayoutChildren)),
    )
}
