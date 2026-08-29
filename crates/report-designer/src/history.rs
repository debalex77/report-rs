use super::*;

impl DesignerApp {
    pub(super) fn record_undo(&mut self) {
        const HISTORY_LIMIT: usize = 100;
        if self.undo_stack.len() == HISTORY_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.report.clone());
        self.redo_stack.clear();
    }

    pub(super) fn undo(&mut self) {
        let Some(report) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack
            .push(std::mem::replace(&mut self.report, report));
        self.sync_after_history("Undo");
    }

    pub(super) fn redo(&mut self) {
        let Some(report) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack
            .push(std::mem::replace(&mut self.report, report));
        self.sync_after_history("Redo");
    }

    pub(super) fn sync_after_history(&mut self, action: &str) {
        self.selected_items
            .retain(|selection| report_contains_selection(&self.report, *selection));
        if let Some(selection) = self
            .selection
            .filter(|selection| report_contains_selection(&self.report, *selection))
        {
            self.active_band = Some(selection.band);
            self.sync_geometry_inputs(selection);
        } else {
            self.selection = None;
            self.geometry_inputs = GeometryInputs::default();
            self.text_inputs = TextInputs::default();
            if self
                .active_band
                .is_some_and(|band| !report_contains_band(&self.report, band))
            {
                self.active_band = None;
            }
        }
        self.dirty = true;
        self.status = format!("{action}: unsaved changes");
        self.refresh_images();
    }
}
