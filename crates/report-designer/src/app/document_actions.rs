use super::*;

impl DesignerApp {
    pub(super) fn mark_dirty(&mut self) {
        self.dirty = true;
        self.status = "Unsaved changes".to_string();
    }

    pub(super) fn new_report(&mut self) {
        if self.dirty && !self.new_report_confirmation_pending {
            self.new_report_confirmation_pending = true;
            self.set_error("Unsaved changes: choose New report again to discard them");
            return;
        }
        self.new_report_confirmation_pending = false;
        self.report = blank_report();
        self.path = None;
        self.images.clear();
        self.selection = None;
        self.selected_items.clear();
        self.active_band = None;
        self.geometry_inputs = GeometryInputs::default();
        self.text_inputs = TextInputs::default();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.data_source_editor = None;
        self.data_query_editor = None;
        self.query_fields.clear();
        self.expanded_data_queries.clear();
        self.selected_data_fields.clear();
        self.data_field_drag = None;
        self.pending_data_field_drop = None;
        self.query_field_picker = None;
        self.error_message = None;
        self.status = "New blank report".to_string();
    }

    pub(super) fn remember_recent_report(&mut self, path: &PathBuf) {
        self.recent_reports.retain(|recent| recent != path);
        self.recent_reports.insert(0, path.clone());
        self.recent_reports.truncate(8);
    }

    pub(super) fn use_tool(&mut self, tool: DesignerTool) {
        match tool {
            DesignerTool::ReportHeader => self.add_band(BandKind::ReportHeader),
            DesignerTool::DataHeader => {
                let source = self
                    .report
                    .data_sources
                    .iter()
                    .flat_map(|source| &source.queries)
                    .next()
                    .map(|query| query.name.clone())
                    .unwrap_or_default();
                self.add_band(BandKind::DataHeader {
                    source,
                    repeat_on_each_page: true,
                });
            }
            DesignerTool::DataBand => self.add_band(BandKind::Data {
                source: self
                    .report
                    .data_sources
                    .iter()
                    .flat_map(|source| &source.queries)
                    .next()
                    .map(|query| query.name.clone())
                    .unwrap_or_default(),
            }),
            DesignerTool::ReportFooter => self.add_band(BandKind::ReportFooter),
            DesignerTool::Text => {
                let font_family = self
                    .report
                    .pages
                    .first()
                    .map(page_font_family)
                    .unwrap_or_else(report_core::model::default_font_family);
                self.add_item(new_text_item(font_family));
            }
            DesignerTool::Image => self.add_item(new_image_item()),
            DesignerTool::Shape => self.add_item(new_shape_item()),
            DesignerTool::HorizontalLayout => self.create_layout_from_selection(true),
            DesignerTool::VerticalLayout => self.create_layout_from_selection(false),
            DesignerTool::Delete => self.delete_selected_item(),
        }
    }

    pub(super) fn add_band(&mut self, kind: BandKind) {
        let Some(page) = self.report.pages.first() else {
            self.set_error("The report does not contain any pages");
            return;
        };
        if page
            .bands
            .iter()
            .any(|band| same_band_kind(&band.kind, &kind))
        {
            self.set_error(format!("{} already exists", band_name(&kind)));
            return;
        }
        self.record_undo();
        let page = &mut self.report.pages[0];
        page.bands.push(Band {
            kind,
            height: Mm(30.0),
            items: Vec::new(),
        });
        let band_index = page.bands.len() - 1;
        self.selection = None;
        self.selected_items.clear();
        self.active_band = Some(band_index);
        self.sync_band_inputs(band_index);
        self.mark_dirty();
    }

    pub(super) fn add_item(&mut self, mut item: Item) {
        let Some(band_index) = self
            .selection
            .map(|selection| selection.band)
            .or(self.active_band)
        else {
            self.set_error("Select a report band or an item inside the target band first");
            return;
        };
        let (band_width, band_height) = match self.report.pages.first() {
            Some(page) => (page.printable_width().0, page.bands[band_index].height.0),
            None => return,
        };
        assign_unique_item_name_in_report(&mut item, &self.report);
        apply_generated_text(&mut item);
        self.record_undo();
        let Some(band) = self
            .report
            .pages
            .first_mut()
            .and_then(|page| page.bands.get_mut(band_index))
        else {
            self.undo_stack.pop();
            self.set_error("The selected band is no longer available");
            return;
        };
        let Some((x, y)) = find_free_item_position(&item, &band.items, band_width, band_height)
        else {
            self.undo_stack.pop();
            self.set_error("No free space is available in the selected band");
            return;
        };
        set_item_origin(&mut item, x, y);
        band.items.push(item);
        self.selection = Some(Selection::top_level(band_index, band.items.len() - 1));
        self.selected_items = vec![self.selection.unwrap()];
        self.active_band = Some(band_index);
        self.sync_geometry_inputs(self.selection.unwrap());
        self.mark_dirty();
    }

    pub(super) fn create_layout_from_selection(&mut self, horizontal: bool) {
        if self.selected_items.len() < 2 {
            self.set_error("Select at least two items with Ctrl + click");
            return;
        }
        if self
            .selected_items
            .iter()
            .any(|selection| !selection.is_top_level())
        {
            self.set_error("Nested layout selection is not supported");
            return;
        }
        let band_index = self.selected_items[0].band;
        if self
            .selected_items
            .iter()
            .any(|selection| selection.band != band_index)
        {
            self.set_error("All layout items must belong to the same report band");
            return;
        }
        let mut indices: Vec<usize> = self
            .selected_items
            .iter()
            .map(|item| item.top_index())
            .collect();
        indices.sort_unstable();
        indices.dedup();
        let Some(band) = self
            .report
            .pages
            .first()
            .and_then(|page| page.bands.get(band_index))
        else {
            return;
        };
        if indices.iter().any(|index| *index >= band.items.len()) {
            self.set_error("The selection is no longer valid");
            return;
        }
        let used_names = collect_report_item_names(&self.report);
        self.record_undo();
        let band = &mut self.report.pages[0].bands[band_index];
        let selected: Vec<Item> = indices
            .iter()
            .map(|index| band.items[*index].clone())
            .collect();
        let origin_x = selected
            .iter()
            .map(|item| normalized_geometry(item).0)
            .fold(f32::INFINITY, f32::min);
        let origin_y = selected
            .iter()
            .map(|item| normalized_geometry(item).1)
            .fold(f32::INFINITY, f32::min);
        let (mut children, retained_name) = flatten_matching_layouts(selected, horizontal);
        let (width, height) = if horizontal {
            (
                children
                    .iter()
                    .map(|item| normalized_geometry(item).2)
                    .sum(),
                children
                    .iter()
                    .map(|item| normalized_geometry(item).3)
                    .fold(0.0_f32, f32::max),
            )
        } else {
            (
                children
                    .iter()
                    .map(|item| normalized_geometry(item).2)
                    .fold(0.0_f32, f32::max),
                children
                    .iter()
                    .map(|item| normalized_geometry(item).3)
                    .sum(),
            )
        };
        arrange_layout_children(&mut children, horizontal, width, height);
        let mut layout = LayoutItem {
            name: retained_name.unwrap_or_default(),
            x: Mm(origin_x),
            y: Mm(origin_y),
            width: Mm(width),
            height: Mm(height),
            items: children,
        };
        if layout.name.is_empty() {
            let mut candidate = if horizontal {
                Item::HorizontalLayout(layout.clone())
            } else {
                Item::VerticalLayout(layout.clone())
            };
            assign_unique_item_name_from_used(&mut candidate, &used_names);
            layout.name = item_name(&candidate).to_string();
        }
        let layout_item = if horizontal {
            Item::HorizontalLayout(layout)
        } else {
            Item::VerticalLayout(layout)
        };

        let insert_at = indices[0];
        for index in indices.into_iter().rev() {
            band.items.remove(index);
        }
        band.items.insert(insert_at, layout_item);
        let selection = Selection::top_level(band_index, insert_at);
        self.selection = Some(selection);
        self.selected_items = vec![selection];
        self.active_band = Some(band_index);
        self.sync_geometry_inputs(selection);
        self.mark_dirty();
    }

    pub(super) fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
    }

    pub(super) fn open_report(&mut self, path: PathBuf) {
        match Report::from_file(path.to_string_lossy().as_ref()) {
            Ok(mut report) => {
                ensure_unique_item_names(&mut report);
                self.report = report;
                self.path = Some(path.clone());
                self.refresh_images();
                self.selection = None;
                self.selected_items.clear();
                self.active_band = None;
                self.geometry_inputs = GeometryInputs::default();
                self.text_inputs = TextInputs::default();
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.dirty = false;
                self.error_message = None;
                self.settings = None;
                self.data_source_editor = None;
                self.data_query_editor = None;
                self.query_fields.clear();
                self.expanded_data_queries.clear();
                self.selected_data_fields.clear();
                self.data_field_drag = None;
                self.pending_data_field_drop = None;
                self.query_field_picker = None;
                self.new_report_confirmation_pending = false;
                self.status = format!("Loaded {}", path.display());
                self.remember_recent_report(&path);
            }
            Err(error) => self.set_error(format!("Load failed: {error}")),
        }
    }

    pub(super) fn save_report_as(&mut self) {
        match select_report_save_file(self.path.as_deref()) {
            Ok(Some(path)) => self.save_report_to(ensure_json_extension(path)),
            Ok(None) => self.status = "Save cancelled".to_string(),
            Err(error) => self.set_error(format!("Save dialog failed: {error}")),
        }
    }

    pub(super) fn save_report_to(&mut self, path: PathBuf) {
        match self.report.save_to_file(path.to_string_lossy().as_ref()) {
            Ok(()) => {
                self.path = Some(path.clone());
                self.remember_recent_report(&path);
                self.dirty = false;
                self.error_message = None;
                self.status = format!("Saved {}", path.display());
                self.refresh_images();
            }
            Err(error) => self.set_error(format!("Save failed: {error}")),
        }
    }
}
