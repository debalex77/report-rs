use super::*;

mod appearance;
mod band;
mod geometry;
mod image;
mod layout;
mod shape;
mod text;

impl DesignerApp {
    pub(super) fn inspector(&self) -> Element<'_, Message> {
        let status = if self.dirty {
            format!("● {}", self.status)
        } else {
            self.status.clone()
        };
        let mut content =
            iced::widget::column![text("Properties").size(18), text(status).size(10)].spacing(5);

        if let Some((band, item)) = self.selection.and_then(|selection| {
            let band = self.report.pages.first()?.bands.get(selection.band)?;
            let item = item_at_selection(&self.report, selection)?;
            Some((band, item))
        }) {
            content = content.push(property_group_header(
                "General",
                PropertyGroup::General,
                self.collapsed_groups.is_collapsed(PropertyGroup::General),
            ));
            if !self.collapsed_groups.is_collapsed(PropertyGroup::General) {
                content = content
                    .push(text(format!("Band: {}", band_name(&band.kind))).size(12))
                    .push(text(format!("Item: {}", item_type_name(item))).size(12))
                    .push(text("Name").size(11))
                    .push(
                        text_input("Item name", item_name(item))
                            .size(12)
                            .padding(4)
                            .on_input(Message::ItemNameChanged),
                    );
            }
            content = geometry::append(self, item, content);
            content = image::append(self, item, content);
            content = text::append(self, item, content);
            content = appearance::append(self, item, content);
            content = shape::append(self, item, content);
            content = layout::append(self, item, content);
        } else if let Some(band_index) = self.active_band {
            if let Some(band) = self.report.pages[0].bands.get(band_index) {
                content = band::append(self, band, content);
            }
        } else {
            content = content.push(text("Click an item on the page to select it."));
        }

        container(scrollable(content.padding(8)).height(Fill))
            .width(self.properties_width)
            .height(Fill)
            .into()
    }
}
