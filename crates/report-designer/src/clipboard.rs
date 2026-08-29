use super::*;

impl DesignerApp {
    pub(super) fn delete_selected_item(&mut self) {
        let Some(selection) = self.selection else {
            self.set_error("Select an item to delete");
            return;
        };
        self.record_undo();
        let Some(band) = self
            .report
            .pages
            .first_mut()
            .and_then(|page| page.bands.get_mut(selection.band))
        else {
            self.undo_stack.pop();
            return;
        };
        if selection.top_index() >= band.items.len() {
            self.undo_stack.pop();
            return;
        }
        if !remove_item_at_path(&mut band.items, selection.indices()) {
            self.undo_stack.pop();
            return;
        }
        self.selection = None;
        self.selected_items.clear();
        self.active_band = Some(selection.band);
        self.geometry_inputs = GeometryInputs::default();
        self.text_inputs = TextInputs::default();
        self.mark_dirty();
    }

    pub(super) fn copy_selected_item(&mut self) {
        let Some(selection) = self.selection else {
            self.set_error("Select an item to copy");
            return;
        };
        let Some(item) = item_at_selection(&self.report, selection) else {
            self.set_error("The selected item is no longer available");
            return;
        };
        self.clipboard_item = Some(item.clone());
        self.error_message = None;
        self.status = format!("Copied {}", item_name(item));
    }

    pub(super) fn select_all_in_active_band(&mut self) {
        let Some(band_index) = self.active_band else {
            self.set_error("Select a report band first");
            return;
        };
        let Some(band) = self
            .report
            .pages
            .first()
            .and_then(|page| page.bands.get(band_index))
        else {
            self.set_error("The active report band is no longer available");
            return;
        };
        self.selected_items = (0..band.items.len())
            .map(|item| Selection::top_level(band_index, item))
            .collect();
        self.selection = self.selected_items.last().copied();
        if let Some(selection) = self.selection {
            self.sync_geometry_inputs(selection);
            self.error_message = None;
            self.status = format!("Selected {} items", self.selected_items.len());
        } else {
            self.geometry_inputs = GeometryInputs::default();
            self.text_inputs = TextInputs::default();
            self.status = "The active band is empty".to_string();
        }
    }

    pub(super) fn cut_selected_item(&mut self) {
        let Some(selection) = self.selection else {
            self.set_error("Select an item to cut");
            return;
        };
        let Some(item) = item_at_selection(&self.report, selection).cloned() else {
            self.set_error("The selected item is no longer available");
            return;
        };
        self.clipboard_item = Some(item);
        self.delete_selected_item();
    }

    pub(super) fn paste_clipboard_item(&mut self) {
        let Some(mut item) = self.clipboard_item.clone() else {
            self.set_error("Copy or cut an item before pasting");
            return;
        };
        let Some(band_index) = self
            .active_band
            .or_else(|| self.selection.map(|item| item.band))
        else {
            self.set_error("Select a report band before pasting");
            return;
        };
        let Some((x, y)) = self.report.pages.first().and_then(|page| {
            let band = page.bands.get(band_index)?;
            find_free_item_position(&item, &band.items, page.printable_width().0, band.height.0)
        }) else {
            self.set_error("No free space is available in the selected band");
            return;
        };

        self.record_undo();
        set_item_origin(&mut item, x, y);
        let Some(band) = self
            .report
            .pages
            .first_mut()
            .and_then(|page| page.bands.get_mut(band_index))
        else {
            self.undo_stack.pop();
            return;
        };
        band.items.push(item);
        let item_index = band.items.len() - 1;
        ensure_unique_item_names(&mut self.report);
        if let Some(item) = self
            .report
            .pages
            .first_mut()
            .and_then(|page| page.bands.get_mut(band_index))
            .and_then(|band| band.items.get_mut(item_index))
        {
            apply_pasted_text(item);
        }
        let selection = Selection::top_level(band_index, item_index);
        self.selection = Some(selection);
        self.selected_items = vec![selection];
        self.active_band = Some(band_index);
        self.sync_geometry_inputs(selection);
        self.error_message = None;
        self.mark_dirty();
    }
}
