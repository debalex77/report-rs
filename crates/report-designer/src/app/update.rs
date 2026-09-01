use super::*;

impl DesignerApp {
    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        if message_closes_app_menu(&message) {
            self.open_menu = None;
            self.recent_reports_expanded = false;
        }
        if message_closes_context_menu(&message) {
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
            Message::Preview => match launch_preview(&self.report, self.path.as_deref()) {
                Ok(()) => {
                    self.error_message = None;
                    self.status = "Preview opened".to_string();
                }
                Err(error) => self.set_error(format!("Cannot open Preview: {error}")),
            },
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
                        let band_height = band.height.0 + dy.max(0.0);
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
                    grow_band_to_fit_items(&mut self.report, selection.band);
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
                        let band_height = band.height.0 + dy.max(0.0);
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
                    grow_band_to_fit_items(&mut self.report, selection.band);
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
                if self.update_selected_text(|item| item.text = value.clone()) {
                    self.mark_dirty();
                }
            }
            Message::ValueTypeChanged(value) => {
                let value_type = match value.as_str() {
                    "Integer" => ValueType::Integer,
                    "Double" => ValueType::Double,
                    "Boolean" => ValueType::Boolean,
                    "Date" => ValueType::Date,
                    "DateTime" => ValueType::DateTime,
                    "Expression" => ValueType::Expression,
                    _ => ValueType::Text,
                };
                self.record_undo();
                if self.update_selected_text(|item| {
                    item.value_type = value_type;
                    if value_type == ValueType::Expression {
                        item.field = None;
                    }
                }) {
                    self.mark_dirty();
                } else {
                    self.undo_stack.pop();
                }
            }
            Message::QuerySourceChanged(value) => {
                let source = if value == "Main Query" {
                    QuerySource::Main
                } else {
                    QuerySource::Named(value)
                };
                self.record_undo();
                if self.update_selected_text(|item| {
                    item.query_source = source.clone();
                    item.field = None;
                }) {
                    self.mark_dirty();
                } else {
                    self.undo_stack.pop();
                }
            }
            Message::QueryFieldChanged(field) => {
                self.record_undo();
                let field = (!field.trim().is_empty()).then_some(field);
                if self.update_selected_text(|item| item.field = field.clone()) {
                    self.mark_dirty();
                } else {
                    self.undo_stack.pop();
                }
            }
            Message::OpenQueryFieldPicker => {
                let Some(selection) = self.selection else {
                    return Task::none();
                };
                let Some(text_item) =
                    item_at_selection(&self.report, selection).and_then(|item| match item {
                        Item::Text(text) => Some(text),
                        _ => None,
                    })
                else {
                    return Task::none();
                };
                let query_name = match &text_item.query_source {
                    QuerySource::Main => self
                        .report
                        .pages
                        .first()
                        .and_then(|page| page.bands.get(selection.band))
                        .and_then(|band| match &band.kind {
                            BandKind::Data { source } if !source.is_empty() => Some(source.clone()),
                            _ => None,
                        }),
                    QuerySource::Named(name) => Some(name.clone()),
                };
                let Some(query_name) = query_name else {
                    self.set_error("Select a query before choosing a field");
                    return Task::none();
                };
                let query_definition = self.report.data_sources.iter().find_map(|source| {
                    source
                        .queries
                        .iter()
                        .find(|query| query.name == query_name)
                        .map(|query| {
                            (
                                source.name.clone(),
                                source.connection.clone(),
                                query.sql.clone(),
                            )
                        })
                });
                let Some((source_name, DataConnection::Sqlite { path }, sql)) = query_definition
                else {
                    self.set_error(format!("Query `{query_name}` was not found"));
                    return Task::none();
                };
                let path = PathBuf::from(path);
                let path = if path.is_absolute() {
                    path
                } else {
                    report_directory(self.path.as_deref()).join(path)
                };
                match SqliteDataProvider::open(&source_name, &path)
                    .and_then(|provider| provider.fields(&source_name, &query_name, &sql))
                {
                    Ok(fields) if fields.is_empty() => {
                        self.set_error(format!("Query `{query_name}` does not return fields"));
                    }
                    Ok(fields) => {
                        self.query_fields.insert(query_name.clone(), fields.clone());
                        self.query_field_picker = Some(QueryFieldPicker { query_name, fields });
                    }
                    Err(error) => self.set_error(format!("Cannot read query fields: {error}")),
                }
            }
            Message::SelectQueryField(field) => {
                let Some(selection) = self.selection else {
                    self.query_field_picker = None;
                    return Task::none();
                };
                let expression =
                    item_at_selection(&self.report, selection).and_then(|item| match item {
                        Item::Text(text) => Some(match &text.query_source {
                            QuerySource::Main => format!("${{{field}}}"),
                            QuerySource::Named(query) => format!("${{{query}.{field}}}"),
                        }),
                        _ => None,
                    });
                let Some(expression) = expression else {
                    self.query_field_picker = None;
                    return Task::none();
                };
                self.record_undo();
                self.text_inputs
                    .text
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        std::sync::Arc::new(expression),
                    )));
                let value = self.text_inputs.text.text();
                if self.update_selected_text(|item| {
                    item.text = value.clone();
                    item.field = None;
                }) {
                    self.mark_dirty();
                } else {
                    self.undo_stack.pop();
                }
                self.query_field_picker = None;
            }
            Message::CloseQueryFieldPicker => self.query_field_picker = None,
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
                let Some(item) =
                    item_at_selection(&self.report, selection).and_then(first_text_item)
                else {
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
                if self.update_selected_text(|item| item.font_family = value.clone()) {
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
                        .and_then(first_text_item)
                        .is_some_and(|text| text.border.is_some());
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
                    .and_then(first_text_item)
                    .is_some_and(|text| text.border.is_some());
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
            Message::FitActiveBandToContents => {
                self.context_menu_position = None;
                let Some(band) = self.active_band else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                let changed = self
                    .report
                    .pages
                    .first_mut()
                    .is_some_and(|page| fit_band_to_contents(page, band));
                if changed {
                    self.record_undo();
                    if let Some(snapshot) = self.undo_stack.last_mut() {
                        *snapshot = previous_report;
                    }
                    self.sync_band_inputs(band);
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
            Message::DataHeaderRepeatChanged(repeat) => {
                self.record_undo();
                if self.update_active_data_header_repeat(repeat) {
                    self.mark_dirty();
                } else {
                    self.undo_stack.pop();
                }
            }
            Message::MoveBandUp(from) | Message::MoveBandDown(from) => {
                let to = if matches!(message, Message::MoveBandUp(_)) {
                    from.checked_sub(1)
                } else {
                    from.checked_add(1)
                };
                let Some(to) = to else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                let changed = self
                    .report
                    .pages
                    .first_mut()
                    .is_some_and(|page| move_band(page, from, to));
                if changed {
                    self.record_undo();
                    if let Some(snapshot) = self.undo_stack.last_mut() {
                        *snapshot = previous_report;
                    }
                    self.active_band = Some(to);
                    self.selection = None;
                    self.selected_items.clear();
                    self.collapsed_structure_layouts.clear();
                    self.sync_band_inputs(to);
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
            Message::ShowSidebarTab(tab) => self.sidebar_tab = tab,
            Message::NewDataSource => {
                self.data_source_editor = Some(DataSourceEditor::new());
            }
            Message::EditDataSource(index) => {
                if let Some(source) = self.report.data_sources.get(index) {
                    self.data_source_editor = Some(DataSourceEditor::from_source(index, source));
                }
            }
            Message::DataSourceNameChanged(name) => {
                if let Some(editor) = &mut self.data_source_editor {
                    editor.name = name;
                    editor.test_result = None;
                }
            }
            Message::DataSourcePathChanged(path) => {
                if let Some(editor) = &mut self.data_source_editor {
                    editor.path = path;
                    editor.test_result = None;
                }
            }
            Message::BrowseDataSourcePath => match select_sqlite_file() {
                Ok(Some(path)) => {
                    let base = report_directory(self.path.as_deref());
                    let path = path.strip_prefix(&base).map(PathBuf::from).unwrap_or(path);
                    if let Some(editor) = &mut self.data_source_editor {
                        editor.path = path.display().to_string();
                        editor.test_result = None;
                    }
                }
                Ok(None) => {}
                Err(error) => self.set_error(format!("SQLite file dialog failed: {error}")),
            },
            Message::TestDataSourceConnection => {
                let Some(editor) = &self.data_source_editor else {
                    return Task::none();
                };
                let path = PathBuf::from(editor.path.trim());
                let path = if path.is_absolute() {
                    path
                } else {
                    report_directory(self.path.as_deref()).join(path)
                };
                match SqliteDataProvider::open(editor.name.trim(), &path) {
                    Ok(_) => {
                        if let Some(editor) = &mut self.data_source_editor {
                            editor.test_result =
                                Some((true, format!("Connection successful: {}", path.display())));
                        }
                    }
                    Err(error) => {
                        if let Some(editor) = &mut self.data_source_editor {
                            editor.test_result =
                                Some((false, format!("Connection failed: {error}")));
                        }
                    }
                }
            }
            Message::SaveDataSource => {
                let Some(editor) = self.data_source_editor.clone() else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                match save_data_source(&mut self.report, &editor) {
                    Ok(()) => {
                        self.record_undo();
                        if let Some(snapshot) = self.undo_stack.last_mut() {
                            *snapshot = previous_report;
                        }
                        self.data_source_editor = None;
                        self.error_message = None;
                        self.mark_dirty();
                    }
                    Err(error) => self.set_error(error),
                }
            }
            Message::CancelDataSourceEdit => self.data_source_editor = None,
            Message::NewDataQuery(source) => {
                if source < self.report.data_sources.len() {
                    self.data_query_editor = Some(DataQueryEditor::new(source));
                }
            }
            Message::EditDataQuery { source, query } => {
                if let Some(data_query) = self
                    .report
                    .data_sources
                    .get(source)
                    .and_then(|data_source| data_source.queries.get(query))
                {
                    self.data_query_editor =
                        Some(DataQueryEditor::from_query(source, query, data_query));
                }
            }
            Message::DataQueryNameChanged(name) => {
                if let Some(editor) = &mut self.data_query_editor {
                    editor.name = name;
                }
            }
            Message::DataQuerySqlEdited(action) => {
                if let Some(editor) = &mut self.data_query_editor {
                    editor.sql.perform(action);
                }
            }
            Message::SaveDataQuery => {
                let Some(editor) = self.data_query_editor.take() else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                match save_data_query(&mut self.report, &editor) {
                    Ok(()) => {
                        self.record_undo();
                        if let Some(snapshot) = self.undo_stack.last_mut() {
                            *snapshot = previous_report;
                        }
                        self.error_message = None;
                        self.mark_dirty();
                    }
                    Err(error) => {
                        self.data_query_editor = Some(editor);
                        self.set_error(error);
                    }
                }
            }
            Message::CancelDataQueryEdit => self.data_query_editor = None,
            Message::ToggleDataQueryFields { source, query } => {
                let Some(query_name) = self
                    .report
                    .data_sources
                    .get(source)
                    .and_then(|source| source.queries.get(query))
                    .map(|query| query.name.clone())
                else {
                    return Task::none();
                };
                if !self.expanded_data_queries.remove(&query_name) {
                    match self.load_query_field_names(source, query) {
                        Ok((query_name, fields)) => {
                            self.query_fields.insert(query_name.clone(), fields);
                            self.expanded_data_queries.insert(query_name);
                            self.error_message = None;
                        }
                        Err(error) => self.set_error(format!("Cannot read query fields: {error}")),
                    }
                }
            }
            Message::ToggleDataField { query, field } => {
                let key = (query, field);
                if !self.selected_data_fields.remove(&key) {
                    self.selected_data_fields.insert(key);
                }
            }
            Message::BeginDataFieldDrag { query, field } => {
                let key = (query.clone(), field.clone());
                if !self.selected_data_fields.contains(&key) {
                    self.selected_data_fields.clear();
                    self.selected_data_fields.insert(key);
                }
                let fields = self
                    .query_fields
                    .get(&query)
                    .map(|fields| {
                        fields
                            .iter()
                            .filter(|field| {
                                self.selected_data_fields
                                    .contains(&(query.clone(), (*field).clone()))
                            })
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .filter(|fields| !fields.is_empty())
                    .unwrap_or_else(|| vec![field]);
                self.data_field_drag = Some(DataFieldDrag { query, fields });
            }
            Message::DropDataFields(band) => {
                let Some(drag) = self.data_field_drag.take() else {
                    return Task::none();
                };
                let valid_target = self
                    .report
                    .pages
                    .first()
                    .and_then(|page| page.bands.get(band))
                    .is_some_and(|band| {
                        matches!(
                            band.kind,
                            BandKind::Data { .. } | BandKind::DataHeader { .. }
                        )
                    });
                if valid_target {
                    let printable_width = self
                        .report
                        .pages
                        .first()
                        .map(|page| page.printable_width().0)
                        .unwrap_or(190.0);
                    let column_width = printable_width / drag.fields.len() as f32;
                    let field_count = drag.fields.len();
                    let mut allocated_width = 0.0;
                    self.pending_data_field_drop = Some(PendingDataFieldDrop {
                        band,
                        query: drag.query,
                        columns: drag
                            .fields
                            .into_iter()
                            .enumerate()
                            .map(|(index, field)| {
                                let width = if index + 1 == field_count {
                                    printable_width - allocated_width
                                } else {
                                    let width = (column_width * 100.0).round() / 100.0;
                                    allocated_width += width;
                                    width
                                };
                                TableColumnSpec {
                                    title: field.clone(),
                                    field,
                                    width: format_mm(width),
                                    alignment: HorizontalAlign::Left,
                                }
                            })
                            .collect(),
                        center_table: true,
                        template_name: String::new(),
                    });
                    self.error_message = None;
                } else {
                    self.set_error("Drop query fields on a DataBand or DataHeader");
                }
            }
            Message::CancelDataFieldDrag => self.data_field_drag = None,
            Message::DroppedColumnTitleChanged(index, title) => {
                if let Some(column) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.title = title;
                }
            }
            Message::DroppedColumnWidthChanged(index, width) => {
                if let Some(column) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.width = width;
                }
            }
            Message::DroppedColumnAlignmentChanged(index, alignment) => {
                if let Some(column) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.alignment = alignment;
                }
            }
            Message::MoveDroppedColumnUp(index) => {
                if index > 0
                    && let Some(drop) = &mut self.pending_data_field_drop
                    && index < drop.columns.len()
                {
                    drop.columns.swap(index, index - 1);
                }
            }
            Message::MoveDroppedColumnDown(index) => {
                if let Some(drop) = &mut self.pending_data_field_drop
                    && index + 1 < drop.columns.len()
                {
                    drop.columns.swap(index, index + 1);
                }
            }
            Message::CenterDroppedTableChanged(center) => {
                if let Some(drop) = &mut self.pending_data_field_drop {
                    drop.center_table = center;
                }
            }
            Message::TableTemplateNameChanged(name) => {
                if let Some(drop) = &mut self.pending_data_field_drop {
                    drop.template_name = name;
                }
            }
            Message::SaveTableTemplate => {
                let Some(drop) = &self.pending_data_field_drop else {
                    return Task::none();
                };
                let name = drop.template_name.trim().to_string();
                if name.is_empty() {
                    self.set_error("Enter a template name");
                    return Task::none();
                }
                let template = TableTemplate {
                    name: name.clone(),
                    center_table: drop.center_table,
                    columns: drop
                        .columns
                        .iter()
                        .map(|column| table_templates::TableTemplateColumn {
                            field: column.field.clone(),
                            title: column.title.clone(),
                            width: column.width.clone(),
                            alignment: table_templates::alignment_name(&column.alignment)
                                .to_string(),
                        })
                        .collect(),
                };
                if let Some(existing) = self
                    .table_templates
                    .iter_mut()
                    .find(|existing| existing.name == name)
                {
                    *existing = template;
                } else {
                    self.table_templates.push(template);
                }
                match save_table_templates(&self.table_templates) {
                    Ok(()) => {
                        self.error_message = None;
                        self.status = format!("Saved table template {name}");
                    }
                    Err(error) => self.set_error(error),
                }
            }
            Message::ApplyTableTemplate(index) => {
                let Some(template) = self.table_templates.get(index).cloned() else {
                    return Task::none();
                };
                let Some(drop) = &mut self.pending_data_field_drop else {
                    return Task::none();
                };
                let current_fields = drop
                    .columns
                    .iter()
                    .map(|column| column.field.as_str())
                    .collect::<HashSet<_>>();
                let template_fields = template
                    .columns
                    .iter()
                    .map(|column| column.field.as_str())
                    .collect::<HashSet<_>>();
                if current_fields != template_fields {
                    self.set_error("The template fields do not match the selected query fields");
                    return Task::none();
                }
                drop.columns = template
                    .columns
                    .into_iter()
                    .map(|column| TableColumnSpec {
                        field: column.field,
                        title: column.title,
                        width: column.width,
                        alignment: table_templates::parse_alignment(&column.alignment),
                    })
                    .collect();
                drop.center_table = template.center_table;
                drop.template_name = template.name;
                self.error_message = None;
            }
            Message::DeleteTableTemplate(index) => {
                if index < self.table_templates.len() {
                    self.table_templates.remove(index);
                    if let Err(error) = save_table_templates(&self.table_templates) {
                        self.set_error(error);
                    }
                }
            }
            Message::CreateDroppedDataFields(include_header) => {
                let Some(drop) = self.pending_data_field_drop.take() else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                let font_family = self
                    .report
                    .pages
                    .first()
                    .map(page_font_family)
                    .unwrap_or_else(|| "Sans".to_string());
                match create_query_table(
                    &mut self.report,
                    drop.band,
                    &drop.query,
                    &drop.columns,
                    include_header,
                    drop.center_table,
                    font_family,
                ) {
                    Ok(()) => {
                        self.record_undo();
                        if let Some(snapshot) = self.undo_stack.last_mut() {
                            *snapshot = previous_report;
                        }
                        self.selection = None;
                        self.selected_items.clear();
                        self.selected_data_fields.clear();
                        self.error_message = None;
                        self.mark_dirty();
                    }
                    Err(error) => {
                        self.report = previous_report;
                        self.pending_data_field_drop = Some(drop);
                        self.set_error(error);
                    }
                }
            }
            Message::CancelDataFieldDrop => self.pending_data_field_drop = None,
            Message::SelectStructureBand(band) => {
                self.selection = None;
                self.selected_items.clear();
                self.active_band = Some(band);
                self.sync_band_inputs(band);
            }
            Message::ToggleStructureLayout(selection) => {
                if !self.collapsed_structure_layouts.remove(&selection) {
                    self.collapsed_structure_layouts.insert(selection);
                }
            }
            Message::BeginStructureDrag(selection) => {
                if report_contains_selection(&self.report, selection) {
                    if self.keyboard_modifiers.shift() {
                        if let Some(anchor) = self.structure_selection_anchor
                            && anchor.band == selection.band
                            && anchor.parent_indices() == selection.parent_indices()
                        {
                            let start = anchor.item_index().min(selection.item_index());
                            let end = anchor.item_index().max(selection.item_index());
                            self.selected_items = (start..=end)
                                .map(|index| selection.with_item_index(index))
                                .collect();
                        } else {
                            self.selected_items = vec![selection];
                        }
                    } else if self.keyboard_modifiers.control() {
                        if let Some(index) = self
                            .selected_items
                            .iter()
                            .position(|selected| *selected == selection)
                        {
                            self.selected_items.remove(index);
                            self.structure_drag = None;
                            self.selection = self.selected_items.last().copied();
                            return Task::none();
                        }
                        self.selected_items.push(selection);
                        self.structure_selection_anchor = Some(selection);
                    } else if !self.selected_items.contains(&selection) {
                        self.selected_items = vec![selection];
                        self.structure_selection_anchor = Some(selection);
                    }
                    self.structure_drag = Some(selection);
                    self.structure_drop_target = Some(StructureDropTarget::Item(selection));
                    self.selection = Some(selection);
                    self.active_band = Some(selection.band);
                    self.sync_geometry_inputs(selection);
                }
            }
            Message::HoverStructureDrop(target) => {
                self.structure_drop_target = self.structure_drag.and_then(|source| {
                    (source.band == target.band
                        && source.parent_indices() == target.parent_indices()
                        || source != target && !source.is_ancestor_of(target))
                    .then_some(StructureDropTarget::Item(target))
                });
            }
            Message::HoverStructureBand(band) => {
                self.structure_drop_target = self.structure_drag.and(
                    report_contains_band(&self.report, band)
                        .then_some(StructureDropTarget::Band(band)),
                );
            }
            Message::DropStructureItem => {
                let Some(source) = self.structure_drag.take() else {
                    return Task::none();
                };
                let Some(target) = self.structure_drop_target.take() else {
                    return Task::none();
                };
                if self.selected_items.len() > 1 && self.selected_items.contains(&source) {
                    let previous_report = self.report.clone();
                    let selections = match target {
                        StructureDropTarget::Band(band) => {
                            move_items_to_band(&mut self.report, &self.selected_items, band)
                        }
                        StructureDropTarget::Item(target) => reorder_items_same_parent(
                            &mut self.report,
                            &self.selected_items,
                            target,
                        )
                        .or_else(|| {
                            item_at_selection(&self.report, target)
                                .is_some_and(|item| item_layout(item).is_some())
                                .then(|| {
                                    move_items_into_layout(
                                        &mut self.report,
                                        &self.selected_items,
                                        target,
                                    )
                                })
                                .flatten()
                        })
                        .or_else(|| {
                            (target.band != source.band && target.is_top_level())
                                .then(|| {
                                    move_items_to_band(
                                        &mut self.report,
                                        &self.selected_items,
                                        target.band,
                                    )
                                })
                                .flatten()
                        }),
                    };
                    if let Some(selections) = selections {
                        self.record_undo();
                        if let Some(snapshot) = self.undo_stack.last_mut() {
                            *snapshot = previous_report;
                        }
                        self.selection = selections.first().copied();
                        self.selected_items = selections;
                        if let Some(selection) = self.selection {
                            self.active_band = Some(selection.band);
                            self.sync_geometry_inputs(selection);
                        }
                        self.mark_dirty();
                    }
                    return Task::none();
                }
                let target_band = match target {
                    StructureDropTarget::Band(band) => Some(band),
                    StructureDropTarget::Item(target) if target.band != source.band => {
                        Some(target.band)
                    }
                    StructureDropTarget::Item(_) => None,
                };
                if let (Some(layout), Some(_)) = (source.parent(), target_band) {
                    self.pending_layout_move = Some(PendingLayoutMove {
                        source,
                        layout,
                        target,
                    });
                    return Task::none();
                }
                let previous_report = self.report.clone();
                let selection = match target {
                    StructureDropTarget::Item(target) => {
                        reorder_item_same_parent(&mut self.report, source, target).or_else(|| {
                            if item_at_selection(&self.report, target)
                                .is_some_and(|item| item_layout(item).is_some())
                            {
                                move_item_into_layout(&mut self.report, source, target)
                            } else {
                                move_item_before(&mut self.report, source, target)
                            }
                        })
                    }
                    StructureDropTarget::Band(band) => {
                        move_item_to_band(&mut self.report, source, band)
                    }
                };
                if let Some(selection) = selection {
                    self.record_undo();
                    if let Some(snapshot) = self.undo_stack.last_mut() {
                        *snapshot = previous_report;
                    }
                    self.selection = Some(selection);
                    self.selected_items = vec![selection];
                    self.active_band = Some(selection.band);
                    self.sync_geometry_inputs(selection);
                    self.mark_dirty();
                }
            }
            Message::MoveEntireLayout => {
                let Some(pending) = self.pending_layout_move.take() else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                let selection = match pending.target {
                    StructureDropTarget::Band(band) => {
                        move_item_to_band(&mut self.report, pending.layout, band)
                    }
                    StructureDropTarget::Item(target) => {
                        move_item_into_layout(&mut self.report, pending.layout, target)
                            .or_else(|| move_item_before(&mut self.report, pending.layout, target))
                    }
                };
                if let Some(selection) = selection {
                    self.record_undo();
                    if let Some(snapshot) = self.undo_stack.last_mut() {
                        *snapshot = previous_report;
                    }
                    self.selection = Some(selection);
                    self.selected_items = vec![selection];
                    self.active_band = Some(selection.band);
                    self.sync_geometry_inputs(selection);
                    self.mark_dirty();
                }
            }
            Message::DismantleLayoutAndMoveItem => {
                let Some(pending) = self.pending_layout_move.take() else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                let child_index = pending.source.item_index();
                let selection = dismantle_layout(&mut self.report, pending.layout)
                    .and_then(|selections| selections.get(child_index).copied())
                    .and_then(|selection| match pending.target {
                        StructureDropTarget::Band(band) => {
                            move_item_to_band(&mut self.report, selection, band)
                        }
                        StructureDropTarget::Item(target) => {
                            let adjusted_target = target.adjusted_after_removal(pending.layout)?;
                            move_item_into_layout(&mut self.report, selection, adjusted_target)
                                .or_else(|| {
                                    move_item_before(&mut self.report, selection, adjusted_target)
                                })
                        }
                    });
                if let Some(selection) = selection {
                    self.record_undo();
                    if let Some(snapshot) = self.undo_stack.last_mut() {
                        *snapshot = previous_report;
                    }
                    self.selection = Some(selection);
                    self.selected_items = vec![selection];
                    self.active_band = Some(selection.band);
                    self.sync_geometry_inputs(selection);
                    self.mark_dirty();
                } else {
                    self.report = previous_report;
                }
            }
            Message::CancelLayoutMove => self.pending_layout_move = None,
            Message::DismantleSelectedLayout => {
                self.context_menu_position = None;
                let Some(selection) = self.selection else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                if let Some(selections) = dismantle_layout(&mut self.report, selection) {
                    self.record_undo();
                    if let Some(snapshot) = self.undo_stack.last_mut() {
                        *snapshot = previous_report;
                    }
                    self.selection = selections.first().copied();
                    self.selected_items = selections;
                    self.active_band = Some(selection.band);
                    if let Some(selection) = self.selection {
                        self.sync_geometry_inputs(selection);
                    }
                    self.mark_dirty();
                }
            }
            Message::BeginStructureRename(selection) => {
                let Some(item) = item_at_selection(&self.report, selection) else {
                    return Task::none();
                };
                self.structure_rename = Some(selection);
                self.structure_name_input = item_name_storage(item).clone();
                self.structure_drag = None;
                self.structure_drop_target = None;
                self.selection = Some(selection);
                self.selected_items = vec![selection];
                self.active_band = Some(selection.band);
                return iced::widget::operation::focus(iced::widget::Id::new(
                    "structure-item-name",
                ));
            }
            Message::BeginSelectedStructureRename => {
                let Some(selection) = self.selection else {
                    return Task::none();
                };
                let Some(item) = item_at_selection(&self.report, selection) else {
                    return Task::none();
                };
                self.structure_rename = Some(selection);
                self.structure_name_input = item_name_storage(item).clone();
                self.properties_visible = true;
                self.sidebar_tab = SidebarTab::Structure;
                return iced::widget::operation::focus(iced::widget::Id::new(
                    "structure-item-name",
                ));
            }
            Message::StructureNameChanged(name) => self.structure_name_input = name,
            Message::CommitStructureRename => {
                let Some(selection) = self.structure_rename else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                match rename_report_item(&mut self.report, selection, &self.structure_name_input) {
                    Ok(true) => {
                        self.record_undo();
                        if let Some(snapshot) = self.undo_stack.last_mut() {
                            *snapshot = previous_report;
                        }
                        self.structure_rename = None;
                        self.error_message = None;
                        self.mark_dirty();
                    }
                    Ok(false) => self.structure_rename = None,
                    Err(error) => self.set_error(error),
                }
            }
            Message::CancelStructureRename => self.structure_rename = None,
            Message::ModifiersChanged(modifiers) => self.keyboard_modifiers = modifiers,
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

pub(crate) fn message_closes_context_menu(message: &Message) -> bool {
    matches!(
        message,
        Message::Copy
            | Message::Paste
            | Message::Cut
            | Message::Delete
            | Message::SelectAll
            | Message::DismantleSelectedLayout
            | Message::FitActiveBandToContents
    )
}

pub(crate) fn message_closes_app_menu(message: &Message) -> bool {
    matches!(
        message,
        Message::NewReport
            | Message::Load
            | Message::Save
            | Message::SaveAs
            | Message::Reload
            | Message::OpenRecentReport(_)
            | Message::Undo
            | Message::Redo
            | Message::Copy
            | Message::Paste
            | Message::Cut
            | Message::Delete
            | Message::SelectAll
            | Message::OpenSettings
            | Message::OpenAbout
    )
}
