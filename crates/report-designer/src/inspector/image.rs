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
            content = content
                .push(text("Source").size(11))
                .push(
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
                )
                .push(text("Fit").size(11))
                .push(
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
