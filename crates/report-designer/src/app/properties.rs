use super::*;

impl DesignerApp {
    pub(super) fn update_selected_text(
        &mut self,
        update: impl FnOnce(&mut report_core::model::TextItem),
    ) -> bool {
        let Some(selection) = self.selection else {
            return false;
        };
        let Some(Item::Text(item)) = item_at_selection_mut(&mut self.report, selection) else {
            return false;
        };
        update(item);
        true
    }

    pub(super) fn update_selected_image(
        &mut self,
        update: impl FnOnce(&mut report_core::model::ImageItem),
    ) -> bool {
        let Some(selection) = self.selection else {
            return false;
        };
        let Some(Item::Image(item)) = item_at_selection_mut(&mut self.report, selection) else {
            return false;
        };
        update(item);
        true
    }

    pub(super) fn sync_geometry_inputs(&mut self, selection: Selection) {
        if let Some(item) = item_at_selection(&self.report, selection) {
            self.geometry_inputs.sync(item);
            self.text_inputs.sync(item);
        }
    }

    pub(super) fn set_geometry(
        &mut self,
        selection: Selection,
        field: GeometryField,
        value: f32,
    ) -> bool {
        if !selection.is_top_level() {
            return false;
        }
        let Some(page) = self.report.pages.first_mut() else {
            return false;
        };
        let band_width = page.printable_width().0;
        let Some(band) = page.bands.get_mut(selection.band) else {
            return false;
        };
        let band_height = band.height.0;
        let Some(item) = band.items.get_mut(selection.top_index()) else {
            return false;
        };

        set_item_geometry(item, field, value, band_width, band_height)
    }

    pub(super) fn geometry_value(&self, selection: Selection, field: GeometryField) -> Option<f32> {
        let item = item_at_selection(&self.report, selection)?;
        item_geometry_value(item, field)
    }
}
