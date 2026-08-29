use super::*;

impl DesignerApp {
    pub(super) fn load_selected_font(&self) -> Task<Message> {
        let Some(selection) = self.selection else {
            return Task::none();
        };
        let Some(Item::Text(item)) = item_at_selection(&self.report, selection) else {
            return Task::none();
        };
        let Some(font) = self.font_resolver.resolve(&item.font_spec()) else {
            return Task::none();
        };

        iced::font::load(font.data).map(|_| Message::FontLoaded)
    }

    pub(super) fn load_font_family(&self, family: &str) -> Task<Message> {
        let spec = FontSpec {
            family: family.to_string(),
            ..FontSpec::default()
        };
        let Some(font) = self.font_resolver.resolve(&spec) else {
            return Task::none();
        };

        iced::font::load(font.data).map(|_| Message::FontLoaded)
    }

    pub(super) fn refresh_images(&mut self) {
        self.images = load_designer_images(&self.report, report_directory(self.path.as_deref()));
    }
}
