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
            Message::Preview => {
                if self.preview_loading {
                    return Task::none();
                }
                self.preview_loading = true;
                self.preview_progress = 0.0;
                self.preview_stage = "Starting Preview…".to_string();
                self.status = "Opening Preview…".to_string();
                let report = self.report.clone();
                let path = self.path.clone();
                return Task::perform(
                    async move { launch_preview(&report, path.as_deref()) },
                    Message::PreviewLaunched,
                );
            }
            Message::PreviewLaunched(result) => match result {
                Ok((ready_path, started_at)) => {
                    self.error_message = None;
                    self.preview_ready_path = Some(ready_path);
                    self.preview_started_at = Some(started_at);
                }
                Err(error) => {
                    self.preview_loading = false;
                    self.set_error(format!("Cannot open Preview: {error}"));
                }
            },
            Message::PreviewProgressTick => {
                if let Some(path) = self.preview_ready_path.clone()
                    && let Ok(contents) = std::fs::read_to_string(&path)
                {
                    let summary = contents.trim();
                    if summary.is_empty() {
                        return Task::none();
                    }
                    if let Some(progress) = summary.strip_prefix("PROGRESS:") {
                        if let Some((percent, stage)) = progress.split_once(':') {
                            if let Ok(percent) = percent.parse::<f32>() {
                                self.preview_progress = percent.clamp(0.0, 100.0);
                            }
                            self.preview_stage = stage.to_string();
                        }
                        return Task::none();
                    }
                    let _ = std::fs::remove_file(&path);
                    self.preview_ready_path = None;
                    self.preview_loading = false;
                    if let Some(error) = summary.strip_prefix("ERROR: ") {
                        self.set_error(format!("Cannot open Preview: {error}"));
                    } else {
                        let elapsed = self
                            .preview_started_at
                            .take()
                            .map(|started| started.elapsed())
                            .unwrap_or_default();
                        let elapsed = if elapsed.as_secs() >= 60 {
                            format!(
                                "{} min {} sec",
                                elapsed.as_secs() / 60,
                                elapsed.as_secs() % 60
                            )
                        } else {
                            format!("{:.1} sec", elapsed.as_secs_f32())
                        };
                        self.status = if summary.is_empty() {
                            format!("Preview opened in {elapsed}")
                        } else {
                            format!("Preview opened: {summary} in {elapsed}")
                        };
                    }
                } else {
                    self.preview_progress = (self.preview_progress + 3.0) % 200.0;
                }
            }
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
                    "Function" => ValueType::Function,
                    _ => ValueType::Text,
                };
                self.record_undo();
                if self.update_selected_text(|item| {
                    item.value_type = value_type;
                    if matches!(value_type, ValueType::Expression | ValueType::Function) {
                        item.field = None;
                    }
                }) {
                    self.mark_dirty();
                    if value_type == ValueType::Function {
                        self.load_function_query_fields();
                        self.function_picker_visible = true;
                    }
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
                    QuerySource::Main => {
                        let band_query = self
                            .report
                            .pages
                            .first()
                            .and_then(|page| page.bands.get(selection.band))
                            .and_then(|band| match &band.kind {
                                BandKind::Data { source } if !source.is_empty() => {
                                    Some(source.clone())
                                }
                                _ => None,
                            });
                        band_query.or_else(|| {
                            let mut queries = self
                                .report
                                .data_sources
                                .iter()
                                .flat_map(|source| source.queries.iter());
                            let first = queries.next()?;
                            queries.next().is_none().then(|| first.name.clone())
                        })
                    }
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
            Message::OpenFunctionPicker => {
                self.load_function_query_fields();
                self.function_picker_visible = true;
            }
            Message::SelectFunction(function) => {
                self.record_undo();
                self.text_inputs
                    .text
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        std::sync::Arc::new(function),
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
                self.function_picker_visible = false;
            }
            Message::CloseFunctionPicker => self.function_picker_visible = false,
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
            Message::ImageSourceTypeChanged(source_type) => {
                let source_type = if source_type == "Database BLOB" {
                    ImageSourceType::Database
                } else {
                    ImageSourceType::File
                };
                self.record_undo();
                if self.update_selected_image(|item| item.source_type = source_type) {
                    if source_type == ImageSourceType::Database {
                        self.load_function_query_fields();
                    }
                    self.refresh_images();
                    self.mark_dirty();
                }
            }
            Message::ImageQuerySourceChanged(query) => {
                let query_source = if query == "Main Query" {
                    QuerySource::Main
                } else {
                    QuerySource::Named(query)
                };
                self.record_undo();
                if self.update_selected_image(|item| {
                    item.query_source = query_source;
                    item.field = None;
                }) {
                    self.mark_dirty();
                }
            }
            Message::ImageFieldChanged(field) => {
                self.record_undo();
                if self.update_selected_image(|item| item.field = Some(field)) {
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
            Message::UnderlineChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|text| text.underline = value) {
                    self.mark_dirty();
                }
            }
            Message::StrikeoutChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|text| text.strikeout = value) {
                    self.mark_dirty();
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
            Message::ValueFormatDecimalChanged(value) => {
                self.text_inputs.decimal_places.clone_from(&value);
                if let Some(decimals) = parse_decimal_places(&value) {
                    self.record_undo();
                    if self.update_selected_text(|text| text.value_format.decimal_places = decimals)
                    {
                        self.mark_dirty();
                    }
                }
            }
            Message::ValueFormatDatePatternChanged(value) => {
                self.text_inputs.date_pattern.clone_from(&value);
                self.record_undo();
                if self.update_selected_text(|text| {
                    text.value_format.date_pattern = (!value.is_empty()).then(|| value.clone())
                }) {
                    self.mark_dirty();
                }
            }
            Message::ValueFormatPrefixChanged(value) => {
                self.text_inputs.value_prefix.clone_from(&value);
                self.record_undo();
                if self.update_selected_text(|text| text.value_format.prefix = value.clone()) {
                    self.mark_dirty();
                }
            }
            Message::ValueFormatSuffixChanged(value) => {
                self.text_inputs.value_suffix.clone_from(&value);
                self.record_undo();
                if self.update_selected_text(|text| text.value_format.suffix = value.clone()) {
                    self.mark_dirty();
                }
            }
            Message::ValueFormatGroupingChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|text| text.value_format.grouping = value) {
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
            Message::GroupFieldChanged(field) => {
                self.record_undo();
                if self.update_active_group_field(field.clone()) {
                    self.band_inputs.group_field = field;
                    self.mark_dirty();
                } else {
                    self.undo_stack.pop();
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
            Message::GroupHeaderRepeatChanged(repeat) => {
                self.record_undo();
                if self.update_active_group_header_repeat(repeat) {
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
            Message::ShowDataQueryTab(tab) => {
                if let Some(editor) = &mut self.data_query_editor {
                    editor.tab = tab;
                }
            }
            Message::ReportParameterTypeChanged(index, value) => {
                let value_type = match value.as_str() {
                    "Integer" => ReportParameterType::Integer,
                    "Double" => ReportParameterType::Double,
                    "Boolean" => ReportParameterType::Boolean,
                    "Date" => ReportParameterType::Date,
                    "DateTime" => ReportParameterType::DateTime,
                    _ => ReportParameterType::Text,
                };
                if let Some(parameter) = self
                    .data_query_editor
                    .as_mut()
                    .and_then(|editor| editor.parameters.get_mut(index))
                {
                    parameter.value_type = value_type;
                }
            }
            Message::ReportParameterDefaultChanged(index, value) => {
                if let Some(parameter) = self
                    .data_query_editor
                    .as_mut()
                    .and_then(|editor| editor.parameters.get_mut(index))
                {
                    parameter.default_value = (!value.is_empty()).then_some(value);
                }
            }
            Message::ReportParameterRequiredChanged(index, required) => {
                if let Some(parameter) = self
                    .data_query_editor
                    .as_mut()
                    .and_then(|editor| editor.parameters.get_mut(index))
                {
                    parameter.required = required;
                }
            }
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
                    self.data_query_editor =
                        Some(DataQueryEditor::new(source, &self.report.parameters));
                }
            }
            Message::EditDataQuery { source, query } => {
                if let Some(data_query) = self
                    .report
                    .data_sources
                    .get(source)
                    .and_then(|data_source| data_source.queries.get(query))
                {
                    self.data_query_editor = Some(DataQueryEditor::from_query(
                        source,
                        query,
                        data_query,
                        &self.report.parameters,
                    ));
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
                    editor.sync_parameters();
                }
            }
            Message::OpenQueryTextMenu(target) => {
                self.query_text_menu = Some(target);
                self.query_text_menu_position = Some(self.cursor_position);
            }
            Message::CloseQueryTextMenu => {
                self.query_text_menu = None;
                self.query_text_menu_position = None;
            }
            Message::CopyQueryText => {
                let target = self.query_text_menu.unwrap_or(QueryTextTarget::Sql);
                let text = match target {
                    QueryTextTarget::ItemText => self
                        .text_inputs
                        .text
                        .selection()
                        .unwrap_or_else(|| self.text_inputs.text.text()),
                    QueryTextTarget::ItemName
                    | QueryTextTarget::DatePattern
                    | QueryTextTarget::ValuePrefix
                    | QueryTextTarget::ValueSuffix
                    | QueryTextTarget::TextColor => context_text_value(self, target),
                    target => self
                        .data_query_editor
                        .as_ref()
                        .map(|editor| match target {
                            QueryTextTarget::Name => editor.name.clone(),
                            QueryTextTarget::Sql => {
                                editor.sql.selection().unwrap_or_else(|| editor.sql.text())
                            }
                            QueryTextTarget::ItemText => unreachable!(),
                            _ => unreachable!(),
                        })
                        .unwrap_or_default(),
                };
                self.query_text_menu = None;
                self.query_text_menu_position = None;
                return iced::clipboard::write(text);
            }
            Message::CutQueryText => {
                let target = self.query_text_menu.unwrap_or(QueryTextTarget::Sql);
                let text = if target == QueryTextTarget::ItemText {
                    let text = self
                        .text_inputs
                        .text
                        .selection()
                        .unwrap_or_else(|| self.text_inputs.text.text());
                    self.record_undo();
                    self.text_inputs
                        .text
                        .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
                    let value = self.text_inputs.text.text();
                    if self.update_selected_text(|item| item.text = value.clone()) {
                        self.mark_dirty();
                    }
                    text
                } else if let Some(message) = context_text_change_message(target, String::new()) {
                    let text = context_text_value(self, target);
                    let _ = self.update(message);
                    text
                } else {
                    let Some(editor) = &mut self.data_query_editor else {
                        return Task::none();
                    };
                    match target {
                        QueryTextTarget::Name => std::mem::take(&mut editor.name),
                        QueryTextTarget::Sql => {
                            let text = editor.sql.selection().unwrap_or_else(|| editor.sql.text());
                            editor
                                .sql
                                .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
                            text
                        }
                        QueryTextTarget::ItemText => unreachable!(),
                        _ => unreachable!(),
                    }
                };
                self.query_text_menu = None;
                self.query_text_menu_position = None;
                return iced::clipboard::write(text);
            }
            Message::PasteQueryText => {
                return iced::clipboard::read().map(Message::QueryTextPasted);
            }
            Message::SelectAllQueryText => {
                if self.query_text_menu == Some(QueryTextTarget::Sql) {
                    if let Some(editor) = &mut self.data_query_editor {
                        editor.sql.perform(text_editor::Action::SelectAll);
                    }
                } else if self.query_text_menu == Some(QueryTextTarget::ItemText) {
                    self.text_inputs
                        .text
                        .perform(text_editor::Action::SelectAll);
                } else if self.query_text_menu == Some(QueryTextTarget::Name) {
                    self.query_text_menu = None;
                    self.query_text_menu_position = None;
                    return iced::advanced::widget::operate(
                        iced::advanced::widget::operation::text_input::select_all(
                            self.query_name_input_id.clone(),
                        ),
                    );
                } else if let Some(id) = match self.query_text_menu {
                    Some(QueryTextTarget::ItemName) => Some(self.item_name_input_id.clone()),
                    Some(QueryTextTarget::DatePattern) => Some(self.date_pattern_input_id.clone()),
                    Some(QueryTextTarget::ValuePrefix) => Some(self.value_prefix_input_id.clone()),
                    Some(QueryTextTarget::ValueSuffix) => Some(self.value_suffix_input_id.clone()),
                    Some(QueryTextTarget::TextColor) => Some(self.text_color_input_id.clone()),
                    _ => None,
                } {
                    self.query_text_menu = None;
                    self.query_text_menu_position = None;
                    return iced::advanced::widget::operate(
                        iced::advanced::widget::operation::text_input::select_all(id),
                    );
                }
                self.query_text_menu = None;
                self.query_text_menu_position = None;
            }
            Message::QueryTextPasted(value) => {
                if let Some(value) = value {
                    if self.query_text_menu == Some(QueryTextTarget::ItemText) {
                        self.record_undo();
                        self.text_inputs.text.perform(text_editor::Action::Edit(
                            text_editor::Edit::Paste(std::sync::Arc::new(value)),
                        ));
                        let text = self.text_inputs.text.text();
                        if self.update_selected_text(|item| item.text = text.clone()) {
                            self.mark_dirty();
                        }
                    } else if let Some(message) = self
                        .query_text_menu
                        .and_then(|target| context_text_change_message(target, value.clone()))
                    {
                        let _ = self.update(message);
                    } else if let Some(editor) = &mut self.data_query_editor {
                        match self.query_text_menu.unwrap_or(QueryTextTarget::Sql) {
                            QueryTextTarget::Name => editor.name = value,
                            QueryTextTarget::Sql => editor.sql.perform(text_editor::Action::Edit(
                                text_editor::Edit::Paste(std::sync::Arc::new(value)),
                            )),
                            QueryTextTarget::ItemText => unreachable!(),
                            _ => unreachable!(),
                        }
                    }
                }
                self.query_text_menu = None;
                self.query_text_menu_position = None;
            }
            Message::SaveDataQuery => {
                let Some(editor) = self.data_query_editor.take() else {
                    return Task::none();
                };
                let source_index = editor.source_index;
                let query_index = editor.query_index;
                let old_name = query_index.and_then(|index| {
                    self.report
                        .data_sources
                        .get(source_index)
                        .and_then(|source| source.queries.get(index))
                        .map(|query| query.name.clone())
                });
                let was_expanded = old_name
                    .as_ref()
                    .is_some_and(|name| self.expanded_data_queries.contains(name));
                let previous_report = self.report.clone();
                match save_data_query(&mut self.report, &editor) {
                    Ok(()) => {
                        self.record_undo();
                        if let Some(snapshot) = self.undo_stack.last_mut() {
                            *snapshot = previous_report;
                        }
                        if let Some(old_name) = old_name {
                            self.expanded_data_queries.remove(&old_name);
                            self.query_fields.remove(&old_name);
                            self.query_field_types
                                .retain(|(query, _), _| query != &old_name);
                            self.selected_data_fields
                                .retain(|(query, _)| query != &old_name);
                        }
                        let mut refresh_error = None;
                        if was_expanded {
                            let saved_index = query_index.unwrap_or_else(|| {
                                self.report.data_sources[source_index].queries.len() - 1
                            });
                            match self.load_query_field_names(source_index, saved_index) {
                                Ok((query_name, fields, types)) => {
                                    self.query_field_types.extend(types.into_iter().map(
                                        |(field, value_type)| {
                                            ((query_name.clone(), field), value_type)
                                        },
                                    ));
                                    self.query_fields.insert(query_name.clone(), fields);
                                    self.expanded_data_queries.insert(query_name);
                                }
                                Err(error) => {
                                    refresh_error = Some(format!(
                                        "Query saved, but its fields could not be refreshed: {error}"
                                    ));
                                }
                            }
                        }
                        self.error_message = refresh_error;
                        self.mark_dirty();
                    }
                    Err(error) => {
                        self.data_query_editor = Some(editor);
                        self.set_error(error);
                    }
                }
            }
            Message::CancelDataQueryEdit => self.data_query_editor = None,
            Message::OpenQueryRules { source, query } => {
                let Some(definition) = self
                    .report
                    .data_sources
                    .get(source)
                    .and_then(|source| source.queries.get(query))
                    .cloned()
                else {
                    self.set_error("The query no longer exists");
                    return Task::none();
                };
                match self.load_query_field_names(source, query) {
                    Ok((query_name, fields, types)) => {
                        self.query_field_types.extend(
                            types.into_iter().map(|(field, value_type)| {
                                ((query_name.clone(), field), value_type)
                            }),
                        );
                        self.query_fields.insert(query_name.clone(), fields.clone());
                        self.query_rules_editor = Some(QueryRulesEditor {
                            source_index: source,
                            query_index: query,
                            query_name,
                            fields,
                            filters: definition.filters,
                            sorts: definition.sorts,
                            preview: None,
                        });
                        self.error_message = None;
                    }
                    Err(error) => self.set_error(format!("Cannot read query fields: {error}")),
                }
            }
            Message::AddQueryFilter => {
                if let Some(editor) = &mut self.query_rules_editor
                    && let Some(field) = editor.fields.first()
                {
                    editor.filters.push(QueryFilter {
                        field: field.clone(),
                        operator: FilterOperator::Equal,
                        value: String::new(),
                        case_sensitive: false,
                    });
                    editor.preview = None;
                }
            }
            Message::QueryFilterFieldChanged(index, field) => {
                if let Some(filter) = self
                    .query_rules_editor
                    .as_mut()
                    .and_then(|editor| editor.filters.get_mut(index))
                {
                    filter.field = field;
                    if let Some(editor) = &mut self.query_rules_editor {
                        editor.preview = None;
                    }
                }
            }
            Message::QueryFilterOperatorChanged(index, operator) => {
                if let Some(filter) = self
                    .query_rules_editor
                    .as_mut()
                    .and_then(|editor| editor.filters.get_mut(index))
                {
                    filter.operator = parse_filter_operator(&operator);
                    if let Some(editor) = &mut self.query_rules_editor {
                        editor.preview = None;
                    }
                }
            }
            Message::QueryFilterValueChanged(index, value) => {
                if let Some(filter) = self
                    .query_rules_editor
                    .as_mut()
                    .and_then(|editor| editor.filters.get_mut(index))
                {
                    filter.value = value;
                    if let Some(editor) = &mut self.query_rules_editor {
                        editor.preview = None;
                    }
                }
            }
            Message::QueryFilterCaseChanged(index, value) => {
                if let Some(filter) = self
                    .query_rules_editor
                    .as_mut()
                    .and_then(|editor| editor.filters.get_mut(index))
                {
                    filter.case_sensitive = value;
                    if let Some(editor) = &mut self.query_rules_editor {
                        editor.preview = None;
                    }
                }
            }
            Message::RemoveQueryFilter(index) => {
                if let Some(editor) = &mut self.query_rules_editor
                    && index < editor.filters.len()
                {
                    editor.filters.remove(index);
                    editor.preview = None;
                }
            }
            Message::AddQuerySort => {
                if let Some(editor) = &mut self.query_rules_editor
                    && let Some(field) = editor.fields.first()
                {
                    editor.sorts.push(QuerySort {
                        field: field.clone(),
                        direction: SortDirection::Ascending,
                    });
                    editor.preview = None;
                }
            }
            Message::QuerySortFieldChanged(index, field) => {
                if let Some(sort) = self
                    .query_rules_editor
                    .as_mut()
                    .and_then(|editor| editor.sorts.get_mut(index))
                {
                    sort.field = field;
                    if let Some(editor) = &mut self.query_rules_editor {
                        editor.preview = None;
                    }
                }
            }
            Message::QuerySortDirectionChanged(index, direction) => {
                if let Some(sort) = self
                    .query_rules_editor
                    .as_mut()
                    .and_then(|editor| editor.sorts.get_mut(index))
                {
                    sort.direction = if direction == "Descending" {
                        SortDirection::Descending
                    } else {
                        SortDirection::Ascending
                    };
                    if let Some(editor) = &mut self.query_rules_editor {
                        editor.preview = None;
                    }
                }
            }
            Message::MoveQuerySortUp(index) => {
                if index > 0
                    && let Some(editor) = &mut self.query_rules_editor
                    && index < editor.sorts.len()
                {
                    editor.sorts.swap(index, index - 1);
                    editor.preview = None;
                }
            }
            Message::MoveQuerySortDown(index) => {
                if let Some(editor) = &mut self.query_rules_editor
                    && index + 1 < editor.sorts.len()
                {
                    editor.sorts.swap(index, index + 1);
                    editor.preview = None;
                }
            }
            Message::RemoveQuerySort(index) => {
                if let Some(editor) = &mut self.query_rules_editor
                    && index < editor.sorts.len()
                {
                    editor.sorts.remove(index);
                    editor.preview = None;
                }
            }
            Message::PreviewQueryRules => {
                let result = self.query_rules_editor.as_ref().map(|editor| {
                    load_query_rules_preview(&self.report, self.path.as_deref(), editor)
                });
                match result {
                    Some(Ok(preview)) => {
                        if let Some(editor) = &mut self.query_rules_editor {
                            editor.preview = Some(preview);
                        }
                        self.error_message = None;
                    }
                    Some(Err(error)) => self.set_error(format!("Cannot preview query: {error}")),
                    None => {}
                }
            }
            Message::SaveQueryRules => {
                let Some(editor) = self.query_rules_editor.take() else {
                    return Task::none();
                };
                let previous_report = self.report.clone();
                let Some(query) = self
                    .report
                    .data_sources
                    .get_mut(editor.source_index)
                    .and_then(|source| source.queries.get_mut(editor.query_index))
                else {
                    self.set_error("The query no longer exists");
                    return Task::none();
                };
                query.filters = editor.filters;
                query.sorts = editor.sorts;
                self.record_undo();
                if let Some(snapshot) = self.undo_stack.last_mut() {
                    *snapshot = previous_report;
                }
                self.error_message = None;
                self.mark_dirty();
            }
            Message::CancelQueryRules => self.query_rules_editor = None,
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
                        Ok((query_name, fields, types)) => {
                            self.query_field_types.extend(types.into_iter().map(
                                |(field, value_type)| ((query_name.clone(), field), value_type),
                            ));
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
            Message::ToggleDataTemplates(query) => {
                if self.open_data_templates.as_deref() == Some(query.as_str()) {
                    self.open_data_templates = None;
                    self.data_templates_position = None;
                } else {
                    self.open_data_templates = Some(query);
                    self.data_templates_position = Some(Point::new(
                        (self.cursor_position.x - 39.0).max(0.0),
                        self.cursor_position.y + 16.0,
                    ));
                }
            }
            Message::CloseDataTemplates => {
                self.open_data_templates = None;
                self.data_templates_position = None;
            }
            Message::GenerateDataFields(query) => {
                let fields = self
                    .query_fields
                    .get(&query)
                    .map(|fields| {
                        let selected = fields
                            .iter()
                            .filter(|field| {
                                self.selected_data_fields
                                    .contains(&(query.clone(), (*field).clone()))
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        if selected.is_empty() {
                            fields.clone()
                        } else {
                            selected
                        }
                    })
                    .unwrap_or_default();
                if fields.is_empty() {
                    self.set_error("Expand the query and select at least one field first");
                    return Task::none();
                }
                let target =
                    self.active_band
                        .filter(|index| {
                            self.report
                                .pages
                                .first()
                                .and_then(|page| page.bands.get(*index))
                                .is_some_and(|band| {
                                    matches!(
                                        band.kind,
                                        BandKind::Data { .. } | BandKind::DataHeader { .. }
                                    )
                                })
                        })
                        .or_else(|| {
                            self.report.pages.first()?.bands.iter().position(|band| {
                                match &band.kind {
                                    BandKind::Data { source }
                                    | BandKind::DataHeader { source, .. } => source == &query,
                                    _ => false,
                                }
                            })
                        })
                        .or_else(|| {
                            self.report.pages.first()?.bands.iter().position(|band| {
                                matches!(
                                    band.kind,
                                    BandKind::Data { .. } | BandKind::DataHeader { .. }
                                )
                            })
                        });
                let Some(target) = target else {
                    self.set_error("Add a DataBand or DataHeader before generating the table");
                    return Task::none();
                };
                self.open_data_templates = None;
                self.data_templates_position = None;
                self.data_field_drag = Some(DataFieldDrag { query, fields });
                return self.update(Message::DropDataFields(target));
            }
            Message::GenerateDataFieldsWithTemplate(query, index) => {
                let Some(template) = self.table_templates.get(index) else {
                    return Task::none();
                };
                let available = self.query_fields.get(&query).cloned().unwrap_or_default();
                let missing = template
                    .columns
                    .iter()
                    .filter(|column| !available.contains(&column.field))
                    .map(|column| column.field.clone())
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    self.set_error(format!(
                        "The query does not contain the template fields: {}",
                        missing.join(", ")
                    ));
                    return Task::none();
                }
                self.selected_data_fields
                    .retain(|(selected_query, _)| selected_query != &query);
                self.selected_data_fields.extend(
                    template
                        .columns
                        .iter()
                        .map(|column| (query.clone(), column.field.clone())),
                );
                let task = self.update(Message::GenerateDataFields(query));
                if self.pending_data_field_drop.is_some() {
                    return self.update(Message::ApplyTableTemplate(index));
                }
                return task;
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
                    let query_name = drag.query.clone();
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
                                let value_type = self
                                    .query_field_types
                                    .get(&(query_name.clone(), field.clone()))
                                    .copied()
                                    .unwrap_or(ValueType::Expression);
                                let alignment = match value_type {
                                    ValueType::Integer | ValueType::Double => {
                                        HorizontalAlign::Right
                                    }
                                    ValueType::Boolean | ValueType::Date | ValueType::DateTime => {
                                        HorizontalAlign::Center
                                    }
                                    _ => HorizontalAlign::Left,
                                };
                                TableColumnSpec {
                                    title: field.clone(),
                                    field,
                                    width: format_mm(width),
                                    alignment,
                                    value_type,
                                    decimal_places: matches!(value_type, ValueType::Double)
                                        .then_some("2".to_string())
                                        .unwrap_or_default(),
                                    date_pattern: match value_type {
                                        ValueType::Date => "dd.MM.yyyy".to_string(),
                                        ValueType::DateTime => "dd.MM.yyyy HH:mm".to_string(),
                                        _ => String::new(),
                                    },
                                    prefix: String::new(),
                                    suffix: String::new(),
                                    grouping: false,
                                }
                            })
                            .collect(),
                        center_table: true,
                        include_row_number: false,
                        groups: Vec::new(),
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
            Message::DroppedColumnValueTypeChanged(index, value_type) => {
                if let Some(column) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.value_type = table_templates::parse_value_type(&value_type);
                }
            }
            Message::DroppedColumnDecimalsChanged(index, value) => {
                if parse_decimal_places(&value).is_some()
                    && let Some(column) = self
                        .pending_data_field_drop
                        .as_mut()
                        .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.decimal_places = value;
                }
            }
            Message::DroppedColumnDatePatternChanged(index, value) => {
                if let Some(column) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.date_pattern = value;
                }
            }
            Message::DroppedColumnPrefixChanged(index, value) => {
                if let Some(column) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.prefix = value;
                }
            }
            Message::DroppedColumnSuffixChanged(index, value) => {
                if let Some(column) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.suffix = value;
                }
            }
            Message::DroppedColumnGroupingChanged(index, value) => {
                if let Some(column) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.columns.get_mut(index))
                {
                    column.grouping = value;
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
            Message::IncludeRowNumberChanged(include) => {
                if let Some(drop) = &mut self.pending_data_field_drop {
                    drop.include_row_number = include;
                }
            }
            Message::AddGeneratedGroup => {
                if let Some(drop) = &mut self.pending_data_field_drop
                    && let Some(field) = drop
                        .columns
                        .iter()
                        .map(|column| &column.field)
                        .find(|field| !drop.groups.iter().any(|group| &group.field == *field))
                {
                    drop.groups.push(TableGroupSpec {
                        field: field.clone(),
                        include_header: true,
                        include_footer: true,
                    });
                }
            }
            Message::GeneratedGroupFieldChanged(index, field) => {
                if let Some(group) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.groups.get_mut(index))
                {
                    group.field = field;
                }
            }
            Message::GeneratedGroupHeaderChanged(index, include) => {
                if let Some(group) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.groups.get_mut(index))
                {
                    group.include_header = include;
                }
            }
            Message::GeneratedGroupFooterChanged(index, include) => {
                if let Some(group) = self
                    .pending_data_field_drop
                    .as_mut()
                    .and_then(|drop| drop.groups.get_mut(index))
                {
                    group.include_footer = include;
                }
            }
            Message::MoveGeneratedGroupUp(index) => {
                if index > 0
                    && let Some(drop) = &mut self.pending_data_field_drop
                    && index < drop.groups.len()
                {
                    drop.groups.swap(index, index - 1);
                }
            }
            Message::MoveGeneratedGroupDown(index) => {
                if let Some(drop) = &mut self.pending_data_field_drop
                    && index + 1 < drop.groups.len()
                {
                    drop.groups.swap(index, index + 1);
                }
            }
            Message::RemoveGeneratedGroup(index) => {
                if let Some(drop) = &mut self.pending_data_field_drop
                    && index < drop.groups.len()
                {
                    drop.groups.remove(index);
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
                    include_row_number: drop.include_row_number,
                    groups: drop
                        .groups
                        .iter()
                        .map(|group| table_templates::TableTemplateGroup {
                            field: group.field.clone(),
                            include_header: group.include_header,
                            include_footer: group.include_footer,
                        })
                        .collect(),
                    columns: drop
                        .columns
                        .iter()
                        .map(|column| table_templates::TableTemplateColumn {
                            field: column.field.clone(),
                            title: column.title.clone(),
                            width: column.width.clone(),
                            alignment: table_templates::alignment_name(&column.alignment)
                                .to_string(),
                            value_type: table_templates::value_type_name(column.value_type)
                                .to_string(),
                            decimal_places: column.decimal_places.clone(),
                            date_pattern: column.date_pattern.clone(),
                            prefix: column.prefix.clone(),
                            suffix: column.suffix.clone(),
                            grouping: column.grouping,
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
                    .map(|column| column.field.clone())
                    .collect::<HashSet<_>>();
                let template_fields = template
                    .columns
                    .iter()
                    .map(|column| column.field.clone())
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
                        value_type: table_templates::parse_value_type(&column.value_type),
                        decimal_places: column.decimal_places,
                        date_pattern: column.date_pattern,
                        prefix: column.prefix,
                        suffix: column.suffix,
                        grouping: column.grouping,
                    })
                    .collect();
                drop.center_table = template.center_table;
                drop.include_row_number = template.include_row_number;
                drop.groups = template
                    .groups
                    .into_iter()
                    .filter(|group| current_fields.contains(&group.field))
                    .map(|group| TableGroupSpec {
                        field: group.field,
                        include_header: group.include_header,
                        include_footer: group.include_footer,
                    })
                    .collect();
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
                let mut columns = drop.columns.clone();
                if drop.include_row_number {
                    let row_number_width = 12.0;
                    let existing_width = columns
                        .iter()
                        .filter_map(|column| column.width.trim().parse::<f32>().ok())
                        .sum::<f32>();
                    if existing_width > row_number_width {
                        let scale = (existing_width - row_number_width) / existing_width;
                        for column in &mut columns {
                            if let Ok(width) = column.width.trim().parse::<f32>() {
                                column.width = format_mm(width * scale);
                            }
                        }
                    }
                    columns.insert(
                        0,
                        TableColumnSpec {
                            field: "row_number".to_string(),
                            title: "Nr.".to_string(),
                            width: format_mm(row_number_width.min(existing_width)),
                            alignment: HorizontalAlign::Right,
                            value_type: ValueType::Integer,
                            decimal_places: String::new(),
                            date_pattern: String::new(),
                            prefix: String::new(),
                            suffix: String::new(),
                            grouping: false,
                        },
                    );
                }
                match create_query_table(
                    &mut self.report,
                    drop.band,
                    &drop.query,
                    &columns,
                    include_header,
                    drop.center_table,
                    &drop.groups,
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
            Message::CursorMoved(position) => self.cursor_position = position,
            Message::ResizeProperties(dx) => {
                self.properties_width =
                    (self.properties_width - dx).clamp(MIN_INSPECTOR_WIDTH, MAX_INSPECTOR_WIDTH);
            }
            Message::ToggleGuides => self.guides_visible = !self.guides_visible,
            Message::DismissError => self.error_message = None,
            Message::OpenSettings => {
                if let Some(page) = self.report.pages.first() {
                    let mut settings = DesignerSettings::from_page(page, page_font_family(page));
                    settings.auto_close_messages = self.auto_close_messages;
                    self.settings = Some(settings);
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
            Message::SettingsAutoCloseMessagesChanged(value) => {
                if let Some(settings) = &mut self.settings {
                    settings.auto_close_messages = value;
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
                        self.auto_close_messages = settings.auto_close_messages;
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

fn context_text_value(app: &DesignerApp, target: QueryTextTarget) -> String {
    match target {
        QueryTextTarget::ItemName => app
            .selection
            .and_then(|selection| item_at_selection(&app.report, selection))
            .map(item_name)
            .unwrap_or_default()
            .to_string(),
        QueryTextTarget::DatePattern => app.text_inputs.date_pattern.clone(),
        QueryTextTarget::ValuePrefix => app.text_inputs.value_prefix.clone(),
        QueryTextTarget::ValueSuffix => app.text_inputs.value_suffix.clone(),
        QueryTextTarget::TextColor => app.text_inputs.text_color.clone(),
        _ => String::new(),
    }
}

fn context_text_change_message(target: QueryTextTarget, value: String) -> Option<Message> {
    match target {
        QueryTextTarget::ItemName => Some(Message::ItemNameChanged(value)),
        QueryTextTarget::DatePattern => Some(Message::ValueFormatDatePatternChanged(value)),
        QueryTextTarget::ValuePrefix => Some(Message::ValueFormatPrefixChanged(value)),
        QueryTextTarget::ValueSuffix => Some(Message::ValueFormatSuffixChanged(value)),
        QueryTextTarget::TextColor => Some(Message::TextColorChanged(value)),
        _ => None,
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
