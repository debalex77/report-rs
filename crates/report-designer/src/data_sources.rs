use super::*;

#[derive(Debug, Clone)]
pub(crate) struct DataSourceEditor {
    pub(crate) index: Option<usize>,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) test_result: Option<(bool, String)>,
}

pub(crate) struct DataQueryEditor {
    pub(crate) source_index: usize,
    pub(crate) query_index: Option<usize>,
    pub(crate) name: String,
    pub(crate) sql: text_editor::Content,
}

impl DataQueryEditor {
    pub(crate) fn new(source_index: usize) -> Self {
        Self {
            source_index,
            query_index: None,
            name: String::new(),
            sql: text_editor::Content::new(),
        }
    }

    pub(crate) fn from_query(source_index: usize, query_index: usize, query: &DataQuery) -> Self {
        Self {
            source_index,
            query_index: Some(query_index),
            name: query.name.clone(),
            sql: text_editor::Content::with_text(&query.sql),
        }
    }
}

impl DataSourceEditor {
    pub(crate) fn new() -> Self {
        Self {
            index: None,
            name: String::new(),
            path: String::new(),
            test_result: None,
        }
    }

    pub(crate) fn from_source(index: usize, source: &DataSourceDefinition) -> Self {
        let DataConnection::Sqlite { path } = &source.connection;
        Self {
            index: Some(index),
            name: source.name.clone(),
            path: path.clone(),
            test_result: None,
        }
    }
}

pub(crate) fn save_data_source(
    report: &mut Report,
    editor: &DataSourceEditor,
) -> Result<(), String> {
    let name = editor.name.trim();
    let path = editor.path.trim();
    if name.is_empty() {
        return Err("Data source name cannot be empty".to_string());
    }
    if path.is_empty() {
        return Err("SQLite database path cannot be empty".to_string());
    }
    if report
        .data_sources
        .iter()
        .enumerate()
        .any(|(index, source)| Some(index) != editor.index && source.name == name)
    {
        return Err("Another data source already uses this name".to_string());
    }
    let queries = editor
        .index
        .and_then(|index| report.data_sources.get(index))
        .map(|source| source.queries.clone())
        .unwrap_or_default();
    let source = DataSourceDefinition {
        name: name.to_string(),
        connection: DataConnection::Sqlite {
            path: path.to_string(),
        },
        queries,
    };
    if let Some(index) = editor.index {
        let target = report
            .data_sources
            .get_mut(index)
            .ok_or_else(|| "The data source no longer exists".to_string())?;
        *target = source;
    } else {
        report.data_sources.push(source);
    }
    Ok(())
}

pub(crate) fn save_data_query(report: &mut Report, editor: &DataQueryEditor) -> Result<(), String> {
    let name = editor.name.trim();
    let sql = editor.sql.text();
    let sql = sql.trim();
    if name.is_empty() {
        return Err("Query name cannot be empty".to_string());
    }
    if sql.is_empty() {
        return Err("SQL cannot be empty".to_string());
    }
    if report
        .data_sources
        .iter()
        .enumerate()
        .any(|(source_index, source)| {
            source
                .queries
                .iter()
                .enumerate()
                .any(|(query_index, query)| {
                    (source_index != editor.source_index || Some(query_index) != editor.query_index)
                        && query.name == name
                })
        })
    {
        return Err("Another query already uses this name".to_string());
    }
    let source = report
        .data_sources
        .get_mut(editor.source_index)
        .ok_or_else(|| "The data source no longer exists".to_string())?;
    let query = DataQuery {
        name: name.to_string(),
        sql: sql.to_string(),
    };
    if let Some(index) = editor.query_index {
        *source
            .queries
            .get_mut(index)
            .ok_or_else(|| "The query no longer exists".to_string())? = query;
    } else {
        source.queries.push(query);
    }
    Ok(())
}

impl DesignerApp {
    pub(super) fn load_query_field_names(
        &self,
        source_index: usize,
        query_index: usize,
    ) -> Result<(String, Vec<String>), String> {
        let source = self
            .report
            .data_sources
            .get(source_index)
            .ok_or_else(|| "The data source no longer exists".to_string())?;
        let query = source
            .queries
            .get(query_index)
            .ok_or_else(|| "The query no longer exists".to_string())?;
        let DataConnection::Sqlite { path } = &source.connection;
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            report_directory(self.path.as_deref()).join(path)
        };
        let fields = SqliteDataProvider::open(&source.name, &path)
            .and_then(|provider| provider.fields(&source.name, &query.name, &query.sql))
            .map_err(|error| error.to_string())?;
        Ok((query.name.clone(), fields))
    }

    pub(super) fn data_source_dialog(&self) -> Element<'_, Message> {
        let Some(editor) = &self.data_source_editor else {
            return Space::new().into();
        };
        let mut content = iced::widget::column![
            text(if editor.index.is_some() {
                "Edit SQLite connection"
            } else {
                "New SQLite connection"
            })
            .size(20),
            text("Name").size(12),
            text_input("main", &editor.name)
                .size(12)
                .padding(6)
                .on_input(Message::DataSourceNameChanged),
            text("Database file").size(12),
        ]
        .spacing(8);
        content = content
            .push(
                row![
                    text_input("data/report.sqlite", &editor.path)
                        .width(Fill)
                        .size(12)
                        .padding(6)
                        .on_input(Message::DataSourcePathChanged),
                    button(text("Browse").size(12))
                        .style(common::style_button(6.0))
                        .on_press(Message::BrowseDataSourcePath),
                ]
                .spacing(6),
            )
            .push(
                text("The database is opened read-only. Relative paths are resolved from the report file.")
                    .size(11),
            );
        if let Some((success, message)) = &editor.test_result {
            content = content.push(text(message).size(11).color(if *success {
                Color::from_rgb8(70, 180, 105)
            } else {
                Color::from_rgb8(225, 90, 80)
            }));
        }
        content = content.push(
            row![
                button(text("Test connection").size(12))
                    .style(common::style_button(6.0))
                    .on_press(Message::TestDataSourceConnection),
                Space::new().width(Fill),
                button(text("Cancel").size(12))
                    .style(common::style_button(6.0))
                    .on_press(Message::CancelDataSourceEdit),
                button(text("Save").size(12))
                    .style(button::primary)
                    .on_press(Message::SaveDataSource),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );
        dialog_container(content.padding(18), 520.0)
    }

    pub(super) fn data_query_dialog(&self) -> Element<'_, Message> {
        let Some(editor) = &self.data_query_editor else {
            return Space::new().into();
        };
        let source_name = self
            .report
            .data_sources
            .get(editor.source_index)
            .map(|source| source.name.as_str())
            .unwrap_or("Missing source");
        let content = iced::widget::column![
            text(if editor.query_index.is_some() {
                "Edit SQLite query"
            } else {
                "New SQLite query"
            })
            .size(20),
            text(format!("Connection: {source_name}")).size(11),
            text("Query name / DataBand source").size(12),
            text_input("orders", &editor.name)
                .size(12)
                .padding(6)
                .on_input(Message::DataQueryNameChanged),
            text("SQL").size(12),
            text_editor(&editor.sql)
                .height(330)
                .padding(7)
                .on_action(Message::DataQuerySqlEdited),
        ]
        .spacing(7);
        let content = content.push(
            row![
                Space::new().width(Fill),
                button(text("Cancel").size(12))
                    .style(common::style_button(6.0))
                    .on_press(Message::CancelDataQueryEdit),
                button(text("Save").size(12))
                    .style(button::primary)
                    .on_press(Message::SaveDataQuery),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );
        dialog_container(scrollable(content.padding(18)).height(560), 620.0)
    }

    pub(super) fn query_field_dialog(&self) -> Element<'_, Message> {
        let Some(picker) = &self.query_field_picker else {
            return Space::new().into();
        };
        let mut fields = iced::widget::column![
            text("Select query field").size(20),
            text(format!("Query: {}", picker.query_name)).size(11),
        ]
        .spacing(7);
        for field in &picker.fields {
            fields = fields.push(
                button(text(field).size(12))
                    .width(Fill)
                    .padding(6)
                    .style(common::style_button(6.0))
                    .on_press(Message::SelectQueryField(field.clone())),
            );
        }
        fields = fields.push(
            row![
                Space::new().width(Fill),
                button(text("Cancel").size(12))
                    .style(common::style_button(6.0))
                    .on_press(Message::CloseQueryFieldPicker),
            ]
            .spacing(6),
        );
        dialog_container(scrollable(fields.padding(18)).height(420), 420.0)
    }

    pub(super) fn data_field_drop_dialog(&self) -> Element<'_, Message> {
        let Some(drop) = &self.pending_data_field_drop else {
            return Space::new().into();
        };
        let mut content = iced::widget::column![
            text("Create table from query").size(20),
            text(format!("Query: {}", drop.query)).size(12),
            row![
                text("Field").size(10).width(110),
                text("Column title").size(10).width(Fill),
                text("Width (mm)").size(10).width(82),
                text("Align").size(10).width(92),
                text("Order").size(10).width(58),
            ]
            .spacing(5),
        ]
        .spacing(9);
        if !self.table_templates.is_empty() {
            let mut templates = row![text("Templates").size(11)].spacing(5);
            for (index, template) in self.table_templates.iter().enumerate() {
                templates = templates
                    .push(
                        button(text(&template.name).size(10))
                            .padding([4, 7])
                            .style(common::style_button(5.0))
                            .on_press(Message::ApplyTableTemplate(index)),
                    )
                    .push(
                        button(text("×").size(10))
                            .width(22)
                            .height(22)
                            .padding(0)
                            .style(button::danger)
                            .on_press(Message::DeleteTableTemplate(index)),
                    );
            }
            content = content.push(templates.align_y(iced::Alignment::Center));
        }
        for (index, column) in drop.columns.iter().enumerate() {
            let align_button = |label, alignment| {
                let selected =
                    std::mem::discriminant(&column.alignment) == std::mem::discriminant(&alignment);
                button(text(if selected { label } else { label }).size(10))
                    .width(26)
                    .height(24)
                    .padding(0)
                    .style(if selected {
                        button::primary
                    } else {
                        button::secondary
                    })
                    .on_press(Message::DroppedColumnAlignmentChanged(index, alignment))
            };
            content = content.push(
                row![
                    text(&column.field).size(11).width(110),
                    text_input("Title", &column.title)
                        .size(11)
                        .padding(5)
                        .width(Fill)
                        .on_input(move |value| Message::DroppedColumnTitleChanged(index, value)),
                    text_input("0.00", &column.width)
                        .size(11)
                        .padding(5)
                        .width(82)
                        .on_input(move |value| Message::DroppedColumnWidthChanged(index, value)),
                    row![
                        align_button("L", HorizontalAlign::Left),
                        align_button("C", HorizontalAlign::Center),
                        align_button("R", HorizontalAlign::Right),
                    ]
                    .spacing(3)
                    .width(92),
                    row![
                        button(text("↑").size(11))
                            .width(26)
                            .height(24)
                            .padding(0)
                            .style(common::style_button(4.0))
                            .on_press_maybe(
                                (index > 0).then_some(Message::MoveDroppedColumnUp(index)),
                            ),
                        button(text("↓").size(11))
                            .width(26)
                            .height(24)
                            .padding(0)
                            .style(common::style_button(4.0))
                            .on_press_maybe(
                                (index + 1 < drop.columns.len())
                                    .then_some(Message::MoveDroppedColumnDown(index)),
                            ),
                    ]
                    .spacing(3)
                    .width(58),
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center),
            );
        }
        content = content
            .push(
                text("Widths are in millimetres and must fit inside the printable page width.")
                    .size(10),
            )
            .push(
                toggler(drop.center_table)
                    .label("Center table in printable area")
                    .text_size(11)
                    .size(16)
                    .spacing(7)
                    .on_toggle(Message::CenterDroppedTableChanged),
            )
            .push(
                row![
                    text_input("Template name", &drop.template_name)
                        .size(11)
                        .padding(5)
                        .width(Fill)
                        .on_input(Message::TableTemplateNameChanged),
                    button(text("Save template").size(11))
                        .style(common::style_button(6.0))
                        .on_press(Message::SaveTableTemplate),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
            .push(
                row![
                    Space::new().width(Fill),
                    button(text("Cancel").size(12))
                        .style(common::style_button(6.0))
                        .on_press(Message::CancelDataFieldDrop),
                    button(text("Data only").size(12))
                        .style(common::style_button(6.0))
                        .on_press(Message::CreateDroppedDataFields(false)),
                    button(text("Header + Data").size(12))
                        .style(button::primary)
                        .on_press(Message::CreateDroppedDataFields(true)),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            );
        dialog_container(scrollable(content.padding(18)).height(520), 720.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_new_sqlite_data_source() {
        let mut report = blank_report();
        let editor = DataSourceEditor {
            index: None,
            name: "main".to_string(),
            path: "data/orders.sqlite".to_string(),
            test_result: None,
        };

        save_data_source(&mut report, &editor).unwrap();

        assert_eq!(report.data_sources.len(), 1);
        assert_eq!(report.data_sources[0].name, "main");
        assert!(matches!(
            &report.data_sources[0].connection,
            DataConnection::Sqlite { path } if path == "data/orders.sqlite"
        ));
    }

    #[test]
    fn rejects_duplicate_data_source_name() {
        let mut report = blank_report();
        save_data_source(
            &mut report,
            &DataSourceEditor {
                index: None,
                name: "main".to_string(),
                path: "first.sqlite".to_string(),
                test_result: None,
            },
        )
        .unwrap();

        let error = save_data_source(
            &mut report,
            &DataSourceEditor {
                index: None,
                name: "main".to_string(),
                path: "second.sqlite".to_string(),
                test_result: None,
            },
        )
        .unwrap_err();

        assert_eq!(error, "Another data source already uses this name");
    }

    #[test]
    fn saves_query_in_selected_data_source() {
        let mut report = blank_report();
        save_data_source(
            &mut report,
            &DataSourceEditor {
                index: None,
                name: "main".to_string(),
                path: "orders.sqlite".to_string(),
                test_result: None,
            },
        )
        .unwrap();
        let editor = DataQueryEditor {
            source_index: 0,
            query_index: None,
            name: "orders".to_string(),
            sql: text_editor::Content::with_text("SELECT id, total FROM orders"),
        };

        save_data_query(&mut report, &editor).unwrap();

        assert_eq!(report.data_sources[0].queries.len(), 1);
        assert_eq!(report.data_sources[0].queries[0].name, "orders");
        assert_eq!(
            report.data_sources[0].queries[0].sql,
            "SELECT id, total FROM orders"
        );
    }

    #[test]
    fn query_names_are_unique_across_data_sources() {
        let mut report = blank_report();
        for (name, path) in [("main", "first.sqlite"), ("archive", "second.sqlite")] {
            save_data_source(
                &mut report,
                &DataSourceEditor {
                    index: None,
                    name: name.to_string(),
                    path: path.to_string(),
                    test_result: None,
                },
            )
            .unwrap();
        }
        save_data_query(
            &mut report,
            &DataQueryEditor {
                source_index: 0,
                query_index: None,
                name: "orders".to_string(),
                sql: text_editor::Content::with_text("SELECT 1"),
            },
        )
        .unwrap();

        let error = save_data_query(
            &mut report,
            &DataQueryEditor {
                source_index: 1,
                query_index: None,
                name: "orders".to_string(),
                sql: text_editor::Content::with_text("SELECT 2"),
            },
        )
        .unwrap_err();

        assert_eq!(error, "Another query already uses this name");
    }
}
