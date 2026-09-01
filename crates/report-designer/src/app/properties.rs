use super::*;

impl DesignerApp {
    pub(super) fn sync_band_inputs(&mut self, band_index: usize) {
        if let Some(band) = self
            .report
            .pages
            .first()
            .and_then(|page| page.bands.get(band_index))
        {
            self.band_inputs.sync(band);
        }
    }

    pub(super) fn set_active_band_height(&mut self, height: f32) -> bool {
        let Some(band_index) = self.active_band else {
            return false;
        };
        let Some(page) = self.report.pages.first_mut() else {
            return false;
        };
        let Some(old_height) = page.bands.get(band_index).map(|band| band.height.0) else {
            return false;
        };
        resize_band_height(page, band_index, height - old_height)
    }

    pub(super) fn update_active_data_source(&mut self, source: String) -> bool {
        let Some(band) = self
            .active_band
            .and_then(|index| self.report.pages.first_mut()?.bands.get_mut(index))
        else {
            return false;
        };
        let BandKind::Data {
            source: data_source,
        } = &mut band.kind
        else {
            return false;
        };
        *data_source = source;
        true
    }

    pub(super) fn update_selected_text(
        &mut self,
        mut update: impl FnMut(&mut report_core::model::TextItem),
    ) -> bool {
        let Some(selection) = self.selection else {
            return false;
        };
        let Some(item) = item_at_selection_mut(&mut self.report, selection) else {
            return false;
        };
        if update_text_items(item, &mut update) == 0 {
            return false;
        }
        propagate_auto_heights(&mut self.report);
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

    pub(super) fn update_selected_shape(
        &mut self,
        update: impl FnOnce(&mut report_core::model::RectangleItem),
    ) -> bool {
        let Some(selection) = self.selection else {
            return false;
        };
        let Some(Item::Rectangle(item)) = item_at_selection_mut(&mut self.report, selection) else {
            return false;
        };
        update(item);
        true
    }

    pub(super) fn sync_geometry_inputs(&mut self, selection: Selection) {
        if let Some(item) = item_at_selection(&self.report, selection) {
            self.geometry_inputs.sync(item);
            self.text_inputs.sync(item);
            self.shape_inputs.sync(item);
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
        let Some(item) = band.items.get_mut(selection.top_index()) else {
            return false;
        };
        let (_, y, _, height) = normalized_geometry(item);
        let band_height = match field {
            GeometryField::Y => band.height.0.max(value + height),
            GeometryField::Height => band.height.0.max(y + value),
            _ => band.height.0,
        };

        let changed = set_item_geometry(item, field, value, band_width, band_height);
        if changed {
            grow_band_to_fit_items(&mut self.report, selection.band);
        }
        changed
    }

    pub(super) fn geometry_value(&self, selection: Selection, field: GeometryField) -> Option<f32> {
        let item = item_at_selection(&self.report, selection)?;
        item_geometry_value(item, field)
    }
}
