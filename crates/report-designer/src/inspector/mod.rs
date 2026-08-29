use super::*;

mod geometry;
mod image;
mod text;

impl DesignerApp {
    pub(super) fn inspector(&self) -> Element<'_, Message> {
        let status = if self.dirty {
            format!("● {}", self.status)
        } else {
            self.status.clone()
        };
        let mut content =
            iced::widget::column![text("Properties").size(20), text(status).size(11)].spacing(7);

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
                    .push(text(format!("Band: {}", band_name(&band.kind))).size(13))
                    .push(text(format!("Item: {}", item_type_name(item))).size(13))
                    .push(text(format!("Name: {}", item_name(item))).size(13));
            }
            content = geometry::append(self, item, content);
            content = image::append(self, item, content);
            content = text::append(self, item, content);
        } else if let Some(band_index) = self.active_band {
            if let Some(band) = self.report.pages[0].bands.get(band_index) {
                content = content
                    .push(text(format!("Band: {}", band_name(&band.kind))).size(13))
                    .push(text("Choose an item tool to insert it in this band.").size(12));
            }
        } else {
            content = content.push(text("Click an item on the page to select it."));
        }

        container(scrollable(content.padding(12)).height(Fill))
            .width(self.properties_width)
            .height(Fill)
            .into()
    }
}
