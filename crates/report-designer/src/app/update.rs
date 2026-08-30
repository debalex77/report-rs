use super::*;

impl DesignerApp {
    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        if !matches!(
            &message,
            Message::ToggleMenu(_) | Message::CloseMenu | Message::ToggleRecentReports
        ) {
            self.open_menu = None;
            self.recent_reports_expanded = false;
        }
        if !matches!(&message, Message::OpenContextMenu { .. }) {
            self.context_menu_position = None;
        }
        match message {
            Message::Load => match select_report_file() {
                Ok(Some(path)) => self.open_report(path),
                Ok(None) => self.status = "Load cancelled".to_string(),
                Err(error) => self.set_error(format!("File dialog failed: {error}")),
            },
            Message::Reload => {
                if let Some(path) = self.path.clone() {
                    self.open_report(path);
                }
            }
            Message::Save => {
                if let Some(path) = self.path.clone() {
                    self.save_report_to(path);
                } else {
                    self.save_report_as();
                }
            }
            Message::SaveAs => self.save_report_as(),
            Message::ZoomIn => self.zoom = (self.zoom + 0.1).min(2.0),
            Message::ZoomOut => self.zoom = (self.zoom - 0.1).max(0.5),
            Message::ZoomReset => self.zoom = 1.0,
            Message::BeginCanvasInteraction {
                selection,
                band,
                additive,
            } => {
                if additive {
                    if let Some(selection) = selection {
                        if let Some(index) = self
                            .selected_items
                            .iter()
                            .position(|selected| *selected == selection)
                        {
                            self.selected_items.remove(index);
                            self.selection = self.selected_items.last().copied();
                        } else {
                            self.selected_items.push(selection);
                            self.selection = Some(selection);
                        }
                    }
                } else {
                    self.selection = selection;
                    self.selected_items = selection.into_iter().collect();
                }
                self.active_band = self.selection.map(|selection| selection.band).or(band);
                if let Some(selection) = self.selection {
                    self.sync_geometry_inputs(selection);
                    self.canvas_interaction_active = false;
                    return self.load_selected_font();
                } else if let Some(band) = self.active_band {
                    self.sync_band_inputs(band);
                }
            }
            Message::EndCanvasInteraction => self.canvas_interaction_active = false,
            Message::MoveItem { selection, dx, dy } => {
                if !self.canvas_interaction_active {
                    self.record_undo();
                    self.canvas_interaction_active = true;
                }
                let mut changed = false;
                if let Some(page) = self.report.pages.first_mut() {
                    let band_width = page.printable_width().0;
                    if let Some(band) = page.bands.get_mut(selection.band) {
                        let band_height = band.height.0;
                        if let Some(item) = band.items.get_mut(selection.top_index()) {
                            move_item(item, dx, dy, band_width, band_height);
                            self.selection = Some(selection);
                            self.dirty = true;
                            self.status = "Unsaved changes".to_string();
                            changed = true;
                        }
                    }
                }
                if changed {
                    self.sync_geometry_inputs(selection);
                }
            }
            Message::ResizeItem {
                selection,
                handle,
                dx,
                dy,
            } => {
                if !self.canvas_interaction_active {
                    self.record_undo();
                    self.canvas_interaction_active = true;
                }
                let mut changed = false;
                if let Some(page) = self.report.pages.first_mut() {
                    let band_width = page.printable_width().0;
                    if let Some(band) = page.bands.get_mut(selection.band) {
                        let band_height = band.height.0;
                        if let Some(item) = band.items.get_mut(selection.top_index()) {
                            resize_item(item, handle, dx, dy, band_width, band_height);
                            self.selection = Some(selection);
                            self.dirty = true;
                            self.status = "Unsaved changes".to_string();
                            changed = true;
                        }
                    }
                }
                if changed {
                    self.sync_geometry_inputs(selection);
                }
            }
            Message::ResizeBand { band, dy } => {
                if !self.canvas_interaction_active {
                    self.record_undo();
                    self.canvas_interaction_active = true;
                }
                if let Some(page) = self.report.pages.first_mut()
                    && resize_band_height(page, band, dy)
                {
                    self.active_band = Some(band);
                    self.selection = None;
                    self.selected_items.clear();
                    self.sync_band_inputs(band);
                    self.mark_dirty();
                }
            }
            Message::ResizeLayoutDivider {
                selection,
                divider,
                horizontal,
                delta,
            } => {
                if !self.canvas_interaction_active {
                    self.record_undo();
                    self.canvas_interaction_active = true;
                }
                let changed = item_at_selection_mut(&mut self.report, selection)
                    .is_some_and(|item| resize_layout_divider(item, divider, horizontal, delta));
                if changed {
                    self.selection = Some(selection);
                    self.mark_dirty();
                }
            }
            Message::GeometryChanged(field, value) => {
                self.geometry_inputs.set(field, value.clone());
                if let (Some(selection), Ok(value)) = (self.selection, value.parse::<f32>())
                    && value.is_finite()
                {
                    self.record_undo();
                    if !self.set_geometry(selection, field, value) {
                        return Task::none();
                    }
                    if let Some(actual) = self.geometry_value(selection, field)
                        && (actual - value).abs() > f32::EPSILON
                    {
                        self.geometry_inputs.set(field, format_mm(actual));
                    }
                    self.dirty = true;
                    self.status = "Unsaved changes".to_string();
                }
            }
            Message::GeometryStep(field, delta) => {
                if let Some(selection) = self.selection
                    && let Some(value) = self.geometry_value(selection, field)
                {
                    self.record_undo();
                    if self.set_geometry(selection, field, value + delta) {
                        self.sync_geometry_inputs(selection);
                        self.mark_dirty();
                    }
                }
            }
            Message::ItemNameChanged(name) => {
                let Some(selection) = self.selection else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                match rename_report_item(&mut self.report, selection, &name) {
                    Ok(true) => {
                        self.record_undo();
                        if let Some(snapshot) = self.undo_stack.last_mut() {
                            *snapshot = previous_report;
                        }
                        self.error_message = None;
                        self.mark_dirty();
                    }
                    Ok(false) => {}
                    Err(error) => {
                        self.set_error(error);
                    }
                }
            }
            Message::TextEdited(action) => {
                self.record_undo();
                self.text_inputs.text.perform(action);
                let value = self.text_inputs.text.text();
                if self.update_selected_text(|item| item.text = value) {
                    self.mark_dirty();
                }
            }
            Message::FontSizeChanged(value) => {
                self.text_inputs.font_size.clone_from(&value);
                if let Ok(font_size) = value.parse::<f32>()
                    && font_size.is_finite()
                {
                    let font_size = font_size.clamp(1.0, 200.0);
                    self.record_undo();
                    if self.update_selected_text(|item| item.font_size = font_size) {
                        if (font_size - value.parse::<f32>().unwrap_or(font_size)).abs()
                            > f32::EPSILON
                        {
                            self.text_inputs.font_size = format_pt(font_size);
                        }
                        self.mark_dirty();
                    }
                }
            }
            Message::FontSizeStep(delta) => {
                let Some(selection) = self.selection else {
                    return Task::none();
                };
                let Some(Item::Text(item)) = item_at_selection(&self.report, selection) else {
                    return Task::none();
                };
                let font_size = (item.font_size + delta).clamp(1.0, 200.0);
                self.record_undo();
                if self.update_selected_text(|item| item.font_size = font_size) {
                    self.text_inputs.font_size = format_pt(font_size);
                    self.mark_dirty();
                }
            }
            Message::ImageSourceChanged(source) => {
                self.record_undo();
                if self.update_selected_image(|item| item.source = source) {
                    self.refresh_images();
                    self.mark_dirty();
                }
            }
            Message::BrowseImageSource => match select_image_file() {
                Ok(Some(path)) => {
                    self.record_undo();
                    let source = path.to_string_lossy().into_owned();
                    if self.update_selected_image(|item| item.source = source) {
                        self.refresh_images();
                        self.error_message = None;
                        self.mark_dirty();
                    }
                }
                Ok(None) => self.status = "Image selection cancelled".to_string(),
                Err(error) => self.set_error(format!("Image dialog failed: {error}")),
            },
            Message::ImageFitChanged(fit) => {
                self.record_undo();
                if self.update_selected_image(|item| item.fit = fit) {
                    self.mark_dirty();
                }
            }
            Message::HorizontalAlignChanged(alignment) => {
                self.record_undo();
                if self.update_selected_text(|item| item.horizontal_align = alignment) {
                    self.mark_dirty();
                }
            }
            Message::VerticalAlignChanged(alignment) => {
                self.record_undo();
                if self.update_selected_text(|item| item.vertical_align = alignment) {
                    self.mark_dirty();
                }
            }
            Message::FontFamilyChanged(value) => {
                self.text_inputs.font_family.clone_from(&value);
                self.record_undo();
                if self.update_selected_text(|item| item.font_family = value) {
                    self.mark_dirty();
                    return self.load_selected_font();
                }
            }
            Message::BoldChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|item| item.bold = value) {
                    self.mark_dirty();
                    return self.load_selected_font();
                }
            }
            Message::ItalicChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|item| item.italic = value) {
                    self.mark_dirty();
                    return self.load_selected_font();
                }
            }
            Message::WordWrapChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|item| item.word_wrap = value) {
                    self.mark_dirty();
                }
            }
            Message::AutoHeightChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|item| item.auto_height = value) {
                    self.mark_dirty();
                }
            }
            Message::PaddingChanged(field, value) => {
                self.text_inputs.set_padding(field, value.clone());
                if let Ok(value) = value.parse::<f32>()
                    && value.is_finite()
                {
                    let value = value.clamp(0.0, 100.0);
                    self.record_undo();
                    if self.update_selected_text(|item| match field {
                        PaddingField::Left => item.padding.left = Mm(value),
                        PaddingField::Top => item.padding.top = Mm(value),
                        PaddingField::Right => item.padding.right = Mm(value),
                        PaddingField::Bottom => item.padding.bottom = Mm(value),
                    }) {
                        self.text_inputs.set_padding(field, format_mm(value));
                        self.mark_dirty();
                    }
                }
            }
            Message::PaddingStep(field, delta) => {
                let value = self
                    .text_inputs
                    .padding(field)
                    .parse::<f32>()
                    .unwrap_or(0.0);
                let value = (value + delta).clamp(0.0, 100.0);
                self.record_undo();
                if self.update_selected_text(|item| match field {
                    PaddingField::Left => item.padding.left = Mm(value),
                    PaddingField::Top => item.padding.top = Mm(value),
                    PaddingField::Right => item.padding.right = Mm(value),
                    PaddingField::Bottom => item.padding.bottom = Mm(value),
                }) {
                    self.text_inputs.set_padding(field, format_mm(value));
                    self.mark_dirty();
                }
            }
            Message::BackgroundEnabled(enabled) => {
                self.record_undo();
                let color =
                    parse_report_color(&self.text_inputs.background).unwrap_or(ReportColor::WHITE);
                if self.update_selected_text(|item| {
                    item.background = enabled.then_some(color);
                }) {
                    self.text_inputs.background = if enabled {
                        format_report_color(color)
                    } else {
                        String::new()
                    };
                    self.mark_dirty();
                }
            }
            Message::BackgroundColorChanged(value) => {
                self.text_inputs.background.clone_from(&value);
                if let Some(color) = parse_report_color(&value) {
                    self.record_undo();
                    if self.update_selected_text(|item| item.background = Some(color)) {
                        self.mark_dirty();
                    }
                }
            }
            Message::BackgroundColorSelected(color) => {
                self.text_inputs.background = format_report_color(color);
                self.record_undo();
                if self.update_selected_text(|item| item.background = Some(color)) {
                    self.mark_dirty();
                }
            }
            Message::BorderEnabled(enabled) => {
                self.record_undo();
                let width = self
                    .text_inputs
                    .border_width
                    .parse::<f32>()
                    .unwrap_or(0.5)
                    .clamp(0.1, 10.0);
                if self.update_selected_text(|item| {
                    item.border = enabled.then_some(Border {
                        left: true,
                        top: true,
                        right: true,
                        bottom: true,
                        width,
                    });
                }) {
                    self.text_inputs.border_width = format_mm(width);
                    self.mark_dirty();
                }
            }
            Message::BorderSideChanged(side, enabled) => {
                self.record_undo();
                if self.update_selected_text(|item| {
                    if let Some(border) = &mut item.border {
                        match side {
                            BorderSide::Left => border.left = enabled,
                            BorderSide::Top => border.top = enabled,
                            BorderSide::Right => border.right = enabled,
                            BorderSide::Bottom => border.bottom = enabled,
                        }
                    }
                }) {
                    self.mark_dirty();
                }
            }
            Message::BorderWidthChanged(value) => {
                self.text_inputs.border_width.clone_from(&value);
                if let Ok(width) = value.parse::<f32>()
                    && width.is_finite()
                {
                    let width = width.clamp(0.1, 10.0);
                    self.text_inputs.border_width = format_mm(width);
                    let enabled = self
                        .selection
                        .and_then(|selection| item_at_selection(&self.report, selection))
                        .is_some_and(
                            |item| matches!(item, Item::Text(text) if text.border.is_some()),
                        );
                    if enabled {
                        self.record_undo();
                        if self.update_selected_text(|item| {
                            if let Some(border) = &mut item.border {
                                border.width = width;
                            }
                        }) {
                            self.mark_dirty();
                        }
                    }
                }
            }
            Message::BorderWidthStep(delta) => {
                let width = self.text_inputs.border_width.parse::<f32>().unwrap_or(0.5);
                let width = (width + delta).clamp(0.1, 10.0);
                self.text_inputs.border_width = format_mm(width);
                let enabled = self
                    .selection
                    .and_then(|selection| item_at_selection(&self.report, selection))
                    .is_some_and(|item| matches!(item, Item::Text(text) if text.border.is_some()));
                if enabled {
                    self.record_undo();
                    if self.update_selected_text(|item| {
                        if let Some(border) = &mut item.border {
                            border.width = width;
                        }
                    }) {
                        self.mark_dirty();
                    }
                }
            }
            Message::ShapeBorderWidthChanged(value) => {
                self.shape_inputs.border_width.clone_from(&value);
                if let Ok(width) = value.parse::<f32>()
                    && width.is_finite()
                {
                    let width = width.clamp(0.1, 10.0);
                    self.record_undo();
                    if self.update_selected_shape(|item| item.border_width = Mm(width)) {
                        self.shape_inputs.border_width = format_mm(width);
                        self.mark_dirty();
                    }
                }
            }
            Message::ShapeBorderWidthStep(delta) => {
                let width = self.shape_inputs.border_width.parse::<f32>().unwrap_or(0.5);
                let width = (width + delta).clamp(0.1, 10.0);
                self.record_undo();
                if self.update_selected_shape(|item| item.border_width = Mm(width)) {
                    self.shape_inputs.border_width = format_mm(width);
                    self.mark_dirty();
                }
            }
            Message::BandHeightChanged(value) => {
                self.band_inputs.height.clone_from(&value);
                if let Ok(height) = value.parse::<f32>()
                    && height.is_finite()
                {
                    self.record_undo();
                    if self.set_active_band_height(height) {
                        if let Some(band) = self.active_band {
                            self.sync_band_inputs(band);
                        }
                        self.mark_dirty();
                    }
                }
            }
            Message::BandHeightStep(delta) => {
                let height = self.band_inputs.height.parse::<f32>().unwrap_or(5.0);
                self.record_undo();
                if self.set_active_band_height(height + delta) {
                    if let Some(band) = self.active_band {
                        self.sync_band_inputs(band);
                    }
                    self.mark_dirty();
                }
            }
            Message::BandDataSourceChanged(source) => {
                self.band_inputs.data_source.clone_from(&source);
                self.record_undo();
                if self.update_active_data_source(source) {
                    self.mark_dirty();
                }
            }
            Message::EqualizeLayoutChildren => {
                let Some(selection) = self.selection else {
                    return Task::none();
                };
                self.record_undo();
                let changed = item_at_selection_mut(&mut self.report, selection)
                    .is_some_and(equalize_layout_children);
                if changed {
                    self.sync_geometry_inputs(selection);
                    self.mark_dirty();
                } else {
                    self.undo_stack.pop();
                }
            }
            Message::TextColorChanged(value) => {
                self.text_inputs.text_color.clone_from(&value);
                if let Some(color) = parse_report_color(&value) {
                    self.record_undo();
                    if !self.update_selected_text(|item| item.text_color = color) {
                        return Task::none();
                    }
                    self.mark_dirty();
                }
            }
            Message::TextColorSelected(color) => {
                self.text_inputs.text_color = format_report_color(color);
                self.record_undo();
                if self.update_selected_text(|item| item.text_color = color) {
                    self.mark_dirty();
                }
            }
            Message::ToggleGroup(group) => self.collapsed_groups.toggle(group),
            Message::FontLoaded => {}
            Message::Undo => self.undo(),
            Message::Redo => self.redo(),
            Message::Copy => self.copy_selected_item(),
            Message::Paste => self.paste_clipboard_item(),
            Message::Cut => self.cut_selected_item(),
            Message::Delete => self.delete_selected_item(),
            Message::SelectAll => self.select_all_in_active_band(),
            Message::OpenContextMenu {
                selection,
                band,
                position,
            } => {
                if let Some(selection) = selection {
                    self.selection = Some(selection);
                    self.selected_items = vec![selection];
                    self.active_band = Some(selection.band);
                    self.sync_geometry_inputs(selection);
                } else {
                    self.selection = None;
                    self.selected_items.clear();
                    if let Some(band) = band {
                        self.active_band = Some(band);
                    }
                }
                self.open_menu = None;
                self.context_menu_position = Some(position);
            }
            Message::CloseContextMenu => self.context_menu_position = None,
            Message::ToggleProperties => self.properties_visible = !self.properties_visible,
            Message::ResizeProperties(dx) => {
                self.properties_width =
                    (self.properties_width - dx).clamp(MIN_INSPECTOR_WIDTH, MAX_INSPECTOR_WIDTH);
            }
            Message::ToggleGuides => self.guides_visible = !self.guides_visible,
            Message::DismissError => self.error_message = None,
            Message::OpenSettings => {
                if let Some(page) = self.report.pages.first() {
                    self.settings = Some(DesignerSettings::from_page(page, page_font_family(page)));
                }
            }
            Message::CloseSettings => self.settings = None,
            Message::SettingsOrientationChanged(orientation) => {
                if let Some(settings) = &mut self.settings {
                    settings.orientation = orientation;
                }
            }
            Message::SettingsMarginChanged(field, value) => {
                if let Some(settings) = &mut self.settings {
                    settings.set_margin(field, value);
                }
            }
            Message::SettingsMarginStep(field, delta) => {
                if let Some(settings) = &mut self.settings {
                    settings.step_margin(field, delta);
                }
            }
            Message::SettingsFontChanged(value) => {
                if let Some(settings) = &mut self.settings {
                    settings.font_family = value;
                }
            }
            Message::ApplySettings => {
                let Some(settings) = self.settings.clone() else {
                    return Task::none();
                };
                self.record_undo();
                let result = self
                    .report
                    .pages
                    .first_mut()
                    .ok_or_else(|| "The report does not contain any pages".to_string())
                    .and_then(|page| settings.apply(page));
                match result {
                    Ok(()) => {
                        self.settings = None;
                        self.error_message = None;
                        if let Some(selection) = self.selection {
                            self.sync_geometry_inputs(selection);
                        }
                        self.mark_dirty();
                        return self.load_font_family(&settings.font_family);
                    }
                    Err(error) => {
                        self.undo_stack.pop();
                        self.set_error(error);
                    }
                }
            }
            Message::ToggleMenu(menu) => {
                self.open_menu = (self.open_menu != Some(menu)).then_some(menu);
                if self.open_menu != Some(AppMenu::File) {
                    self.recent_reports_expanded = false;
                }
            }
            Message::CloseMenu => self.open_menu = None,
            Message::OpenAbout => {
                self.settings = None;
                self.about_visible = true;
            }
            Message::CloseAbout => self.about_visible = false,
            Message::ToggleToolbox => self.toolbox_visible = !self.toolbox_visible,
            Message::UseTool(tool) => self.use_tool(tool),
            Message::NewReport => self.new_report(),
            Message::ToggleRecentReports => {
                self.recent_reports_expanded = !self.recent_reports_expanded;
            }
            Message::OpenRecentReport(path) => self.open_report(path),
        }

        Task::none()
    }
}
