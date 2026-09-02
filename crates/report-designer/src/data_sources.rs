use super::*;
use std::path::Path;

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
    pub(crate) tab: DataQueryTab,
    pub(crate) parameters: Vec<ReportParameter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataQueryTab {
    Sql,
    Parameters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueryTextTarget {
    Name,
    Sql,
}

impl DataQueryEditor {
    pub(crate) fn new(source_index: usize, report_parameters: &[ReportParameter]) -> Self {
        let mut editor = Self {
            source_index,
            query_index: None,
            name: String::new(),
            sql: text_editor::Content::new(),
            tab: DataQueryTab::Sql,
            parameters: report_parameters.to_vec(),
        };
        editor.sync_parameters();
        editor
    }

    pub(crate) fn from_query(
        source_index: usize,
        query_index: usize,
        query: &DataQuery,
        report_parameters: &[ReportParameter],
    ) -> Self {
        let mut editor = Self {
            source_index,
            query_index: Some(query_index),
            name: query.name.clone(),
            sql: text_editor::Content::with_text(&query.sql),
            tab: DataQueryTab::Sql,
            parameters: report_parameters.to_vec(),
        };
        editor.sync_parameters();
        editor
    }

    pub(crate) fn sync_parameters(&mut self) {
        let names = sql_parameter_names(&self.sql.text());
        let previous = std::mem::take(&mut self.parameters);
        self.parameters = names
            .into_iter()
            .map(|name| {
                previous
                    .iter()
                    .find(|parameter| parameter.name == name)
                    .cloned()
                    .unwrap_or(ReportParameter {
                        name,
                        value_type: ReportParameterType::Text,
                        default_value: None,
                        required: false,
                    })
            })
            .collect();
    }
}

fn sql_parameter_names(sql: &str) -> Vec<String> {
    let chars = sql.chars().collect::<Vec<_>>();
    let mut names = Vec::new();
    let mut index = 0;
    let mut quote = None;
    while index < chars.len() {
        let ch = chars[index];
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            }
            index += 1;
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            index += 1;
            continue;
        }
        if ch == ':'
            && chars
                .get(index + 1)
                .is_some_and(|ch| ch.is_ascii_alphabetic() || *ch == '_')
        {
            let start = index + 1;
            index = start + 1;
            while chars
                .get(index)
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            {
                index += 1;
            }
            let name = chars[start..index].iter().collect::<String>();
            if !names.contains(&name) {
                names.push(name);
            }
            continue;
        }
        index += 1;
    }
    names
}

impl DataSourceEditor {
    pub(crate) fn new() -> Self {
        Self {
            index: None,
            name: "main".to_string(),
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
    for parameter in &editor.parameters {
        if let Some(existing) = report
            .parameters
            .iter_mut()
            .find(|existing| existing.name == parameter.name)
        {
            *existing = parameter.clone();
        } else {
            report.parameters.push(parameter.clone());
        }
    }
    let source = report
        .data_sources
        .get_mut(editor.source_index)
        .ok_or_else(|| "The data source no longer exists".to_string())?;
    let query = DataQuery {
        name: name.to_string(),
        sql: sql.to_string(),
        filters: editor
            .query_index
            .and_then(|index| source.queries.get(index))
            .map(|query| query.filters.clone())
            .unwrap_or_default(),
        sorts: editor
            .query_index
            .and_then(|index| source.queries.get(index))
            .map(|query| query.sorts.clone())
            .unwrap_or_default(),
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

fn infer_query_value_type(field: &str, value: Option<&Value>) -> ValueType {
    let name = field.to_ascii_lowercase();
    if name.starts_with("is_") || name.starts_with("has_") || name.starts_with("can_") {
        return ValueType::Boolean;
    }
    match value {
        Some(Value::Number(number)) => {
            if number.fract() == 0.0 {
                ValueType::Integer
            } else {
                ValueType::Double
            }
        }
        Some(Value::Bool(_)) => ValueType::Boolean,
        Some(Value::String(value)) if looks_like_iso_datetime(value) => ValueType::DateTime,
        Some(Value::String(value)) if looks_like_iso_date(value) => ValueType::Date,
        Some(Value::String(_)) => ValueType::Text,
        Some(Value::Null) | None
            if name.contains("date")
                || name.contains("birth")
                || name.ends_with("_at")
                || name.contains("time") =>
        {
            ValueType::Date
        }
        Some(Value::Null) | None => ValueType::Expression,
    }
}

fn looks_like_iso_date(value: &str) -> bool {
    value.len() >= 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value[..4]
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn looks_like_iso_datetime(value: &str) -> bool {
    looks_like_iso_date(value)
        && value.len() >= 16
        && matches!(value.as_bytes().get(10), Some(b'T' | b' '))
}

pub(super) fn parse_filter_operator(value: &str) -> FilterOperator {
    match value {
        "Not equal" => FilterOperator::NotEqual,
        "Contains" => FilterOperator::Contains,
        "Starts with" => FilterOperator::StartsWith,
        "Greater" => FilterOperator::Greater,
        "Greater or equal" => FilterOperator::GreaterOrEqual,
        "Less" => FilterOperator::Less,
        "Less or equal" => FilterOperator::LessOrEqual,
        "Is null" => FilterOperator::IsNull,
        "Is not null" => FilterOperator::IsNotNull,
        _ => FilterOperator::Equal,
    }
}

fn filter_operator_name(operator: FilterOperator) -> &'static str {
    match operator {
        FilterOperator::Equal => "Equal",
        FilterOperator::NotEqual => "Not equal",
        FilterOperator::Contains => "Contains",
        FilterOperator::StartsWith => "Starts with",
        FilterOperator::Greater => "Greater",
        FilterOperator::GreaterOrEqual => "Greater or equal",
        FilterOperator::Less => "Less",
        FilterOperator::LessOrEqual => "Less or equal",
        FilterOperator::IsNull => "Is null",
        FilterOperator::IsNotNull => "Is not null",
    }
}

pub(super) fn load_query_rules_preview(
    report: &Report,
    report_path: Option<&Path>,
    editor: &QueryRulesEditor,
) -> Result<QueryRulesPreview, String> {
    let source = report
        .data_sources
        .get(editor.source_index)
        .ok_or_else(|| "The data source no longer exists".to_string())?;
    let query = source
        .queries
        .get(editor.query_index)
        .ok_or_else(|| "The query no longer exists".to_string())?;
    let DataConnection::Sqlite { path } = &source.connection;
    let path = PathBuf::from(path);
    let path = if path.is_absolute() {
        path
    } else {
        report_directory(report_path).join(path)
    };
    let provider =
        SqliteDataProvider::open(&source.name, &path).map_err(|error| error.to_string())?;
    let mut rows = provider
        .query(&source.name, &query.name, &query.sql)
        .map_err(|error| error.to_string())?;
    let total_rows = rows.len();
    let draft = DataQuery {
        name: query.name.clone(),
        sql: query.sql.clone(),
        filters: editor.filters.clone(),
        sorts: editor.sorts.clone(),
    };
    apply_query_transformations(&mut rows, &draft);
    let filtered_rows = rows.len();
    let preview_rows = rows
        .into_iter()
        .take(8)
        .map(|row| {
            editor
                .fields
                .iter()
                .map(|field| row.get(field).map(Value::as_string).unwrap_or_default())
                .collect()
        })
        .collect();
    Ok(QueryRulesPreview {
        total_rows,
        filtered_rows,
        fields: editor.fields.clone(),
        rows: preview_rows,
    })
}

impl DesignerApp {
    pub(super) fn load_query_field_names(
        &self,
        source_index: usize,
        query_index: usize,
    ) -> Result<(String, Vec<String>, HashMap<String, ValueType>), String> {
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
        let provider =
            SqliteDataProvider::open(&source.name, &path).map_err(|error| error.to_string())?;
        let fields = provider
            .fields(&source.name, &query.name, &query.sql)
            .map_err(|error| error.to_string())?;
        let sql = query.sql.trim().trim_end_matches(';');
        let sample_sql = format!("SELECT * FROM ({sql}) AS report_rs_sample LIMIT 1");
        let sample = provider
            .query(&source.name, &query.name, &sample_sql)
            .ok()
            .and_then(|rows| rows.into_iter().next());
        let types = fields
            .iter()
            .map(|field| {
                let value = sample.as_ref().and_then(|row| row.get(field));
                (field.clone(), infer_query_value_type(field, value))
            })
            .collect();
        Ok((query.name.clone(), fields, types))
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
        let tabs = row![
            button(container(text("SQL").size(12)).center(Fill))
                .width(Fill)
                .height(28)
                .padding(0)
                .style(if editor.tab == DataQueryTab::Sql {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::ShowDataQueryTab(DataQueryTab::Sql)),
            button(container(text("Parameters").size(12)).center(Fill))
                .width(Fill)
                .height(28)
                .padding(0)
                .style(if editor.tab == DataQueryTab::Parameters {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::ShowDataQueryTab(DataQueryTab::Parameters)),
        ]
        .spacing(4);
        let editor_body: Element<'_, Message> = if editor.tab == DataQueryTab::Sql {
            mouse_area(
                text_editor(&editor.sql)
                    .height(330)
                    .padding(7)
                    .on_action(Message::DataQuerySqlEdited),
            )
            .on_right_press(Message::OpenQueryTextMenu(QueryTextTarget::Sql))
            .into()
        } else {
            let types = ["Text", "Integer", "Double", "Boolean", "Date", "DateTime"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            let mut parameters = iced::widget::column![
                text("Parameters detected automatically from :name occurrences in SQL.").size(10)
            ]
            .spacing(7);
            if editor.parameters.is_empty() {
                parameters = parameters.push(text("No SQL parameters detected.").size(11));
            }
            for (index, parameter) in editor.parameters.iter().enumerate() {
                parameters = parameters.push(
                    container(
                        iced::widget::column![
                            row![
                                text("Name").size(11).width(62),
                                text(format!(":{}", parameter.name)).size(12),
                            ]
                            .spacing(5),
                            row![
                                text("Type").size(11).width(62),
                                pick_list(
                                    types.clone(),
                                    Some(parameter_type_name(parameter.value_type).to_string()),
                                    move |value| Message::ReportParameterTypeChanged(index, value),
                                )
                                .width(Fill)
                                .text_size(11)
                                .padding(4),
                            ]
                            .spacing(5)
                            .align_y(iced::Alignment::Center),
                            row![
                                text("Default").size(11).width(62),
                                text_input(
                                    "Optional value",
                                    parameter.default_value.as_deref().unwrap_or(""),
                                )
                                .width(Fill)
                                .size(11)
                                .padding(5)
                                .on_input(move |value| {
                                    Message::ReportParameterDefaultChanged(index, value)
                                }),
                            ]
                            .spacing(5)
                            .align_y(iced::Alignment::Center),
                            toggler(parameter.required)
                                .label("Required")
                                .text_size(11)
                                .size(16)
                                .spacing(7)
                                .on_toggle(move |value| Message::ReportParameterRequiredChanged(
                                    index, value
                                )),
                        ]
                        .spacing(6),
                    )
                    .padding(7)
                    .width(Fill)
                    .style(|theme: &Theme| container::Style {
                        border: iced::Border {
                            color: theme.extended_palette().background.strong.color,
                            width: 1.0,
                            radius: iced::border::radius(7),
                        },
                        ..Default::default()
                    }),
                );
            }
            scrollable(parameters).height(330).into()
        };
        let content = iced::widget::column![
            text(if editor.query_index.is_some() {
                "Edit SQLite query"
            } else {
                "New SQLite query"
            })
            .size(20),
            text(format!("Connection: {source_name}")).size(11),
            text("Query name / DataBand source").size(12),
            mouse_area(
                text_input("orders", &editor.name)
                    .id(self.query_name_input_id.clone())
                    .size(12)
                    .padding(6)
                    .on_input(Message::DataQueryNameChanged)
            )
            .on_right_press(Message::OpenQueryTextMenu(QueryTextTarget::Name)),
            tabs,
            editor_body,
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

    pub(super) fn query_rules_dialog(&self) -> Element<'_, Message> {
        let Some(editor) = &self.query_rules_editor else {
            return Space::new().into();
        };
        let operators = [
            "Equal",
            "Not equal",
            "Contains",
            "Starts with",
            "Greater",
            "Greater or equal",
            "Less",
            "Less or equal",
            "Is null",
            "Is not null",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let mut content = iced::widget::column![
            text("Query filters and sorting").size(20),
            text(format!("Query: {}", editor.query_name)).size(11),
            row![
                text("Filters").size(15),
                Space::new().width(Fill),
                button(text("+ Filter").size(11))
                    .style(common::style_button(6.0))
                    .on_press_maybe((!editor.fields.is_empty()).then_some(Message::AddQueryFilter)),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8);
        if editor.filters.is_empty() {
            content = content.push(text("No filters. All query rows are included.").size(10));
        }
        for (index, filter) in editor.filters.iter().enumerate() {
            content = content.push(
                row![
                    pick_list(
                        editor.fields.clone(),
                        Some(filter.field.clone()),
                        move |value| Message::QueryFilterFieldChanged(index, value),
                    )
                    .width(140)
                    .text_size(11)
                    .padding(4),
                    pick_list(
                        operators.clone(),
                        Some(filter_operator_name(filter.operator).to_string()),
                        move |value| Message::QueryFilterOperatorChanged(index, value),
                    )
                    .width(145)
                    .text_size(11)
                    .padding(4),
                    text_input("Value", &filter.value)
                        .width(Fill)
                        .size(11)
                        .padding(4)
                        .on_input(move |value| Message::QueryFilterValueChanged(index, value)),
                    toggler(filter.case_sensitive)
                        .label("Aa")
                        .text_size(10)
                        .size(14)
                        .spacing(3)
                        .on_toggle(move |value| Message::QueryFilterCaseChanged(index, value)),
                    button(container(text("×").size(11)).center(Fill))
                        .width(26)
                        .height(26)
                        .padding(0)
                        .style(common::style_button(5.0))
                        .on_press(Message::RemoveQueryFilter(index)),
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center),
            );
        }
        content = content.push(rule::horizontal(1)).push(
            row![
                text("Sorting").size(15),
                Space::new().width(Fill),
                button(text("+ Sort").size(11))
                    .style(common::style_button(6.0))
                    .on_press_maybe((!editor.fields.is_empty()).then_some(Message::AddQuerySort)),
            ]
            .align_y(iced::Alignment::Center),
        );
        if editor.sorts.is_empty() {
            content = content.push(text("No sorting. Query order is preserved.").size(10));
        }
        for (index, sort) in editor.sorts.iter().enumerate() {
            let direction = match sort.direction {
                SortDirection::Ascending => "Ascending",
                SortDirection::Descending => "Descending",
            };
            content = content.push(
                row![
                    pick_list(
                        editor.fields.clone(),
                        Some(sort.field.clone()),
                        move |value| Message::QuerySortFieldChanged(index, value),
                    )
                    .width(Fill)
                    .text_size(11)
                    .padding(4),
                    pick_list(
                        vec!["Ascending".to_string(), "Descending".to_string()],
                        Some(direction.to_string()),
                        move |value| Message::QuerySortDirectionChanged(index, value),
                    )
                    .width(135)
                    .text_size(11)
                    .padding(4),
                    button(container(text("↑").size(11)).center(Fill))
                        .width(26)
                        .height(26)
                        .padding(0)
                        .style(common::style_button(4.0))
                        .on_press_maybe((index > 0).then_some(Message::MoveQuerySortUp(index))),
                    button(container(text("↓").size(11)).center(Fill))
                        .width(26)
                        .height(26)
                        .padding(0)
                        .style(common::style_button(4.0))
                        .on_press_maybe(
                            (index + 1 < editor.sorts.len())
                                .then_some(Message::MoveQuerySortDown(index)),
                        ),
                    button(container(text("×").size(11)).center(Fill))
                        .width(26)
                        .height(26)
                        .padding(0)
                        .style(common::style_button(5.0))
                        .on_press(Message::RemoveQuerySort(index)),
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center),
            );
        }
        content = content.push(rule::horizontal(1)).push(
            row![
                text("Result preview").size(15),
                Space::new().width(Fill),
                button(text("Preview rules").size(11))
                    .style(common::style_button(6.0))
                    .on_press(Message::PreviewQueryRules),
            ]
            .align_y(iced::Alignment::Center),
        );
        if let Some(preview) = &editor.preview {
            content = content.push(
                text(format!(
                    "Rows: {} before filters → {} after filters",
                    preview.total_rows, preview.filtered_rows
                ))
                .size(11),
            );
            let header = preview
                .fields
                .iter()
                .map(|field| truncate(field, 18))
                .collect::<Vec<_>>()
                .join("  |  ");
            let mut result = iced::widget::column![text(header).size(10)].spacing(3);
            for row in &preview.rows {
                let row = row
                    .iter()
                    .map(|value| truncate(value, 18))
                    .collect::<Vec<_>>()
                    .join("  |  ");
                result = result.push(text(row).size(10));
            }
            if preview.rows.is_empty() {
                result = result.push(text("No rows match the current filters.").size(10));
            }
            content = content.push(container(result.padding(7)).width(Fill).style(
                |theme: &Theme| container::Style {
                    background: Some(Background::Color(
                        theme.extended_palette().background.weak.color,
                    )),
                    border: iced::Border {
                        color: theme.extended_palette().background.strong.color,
                        width: 1.0,
                        radius: iced::border::radius(6),
                    },
                    ..Default::default()
                },
            ));
        }
        content = content.push(
            row![
                Space::new().width(Fill),
                button(text("Cancel").size(12))
                    .style(common::style_button(6.0))
                    .on_press(Message::CancelQueryRules),
                button(text("Save").size(12))
                    .style(button::primary)
                    .on_press(Message::SaveQueryRules),
            ]
            .spacing(6),
        );
        dialog_container(scrollable(content.padding(18)).height(560), 720.0)
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
                container(text("Align").size(10)).width(92).center_x(92),
                container(text("Order").size(10)).width(58).center_x(58),
            ]
            .spacing(5),
        ]
        .spacing(9);
        if !self.table_templates.is_empty() {
            let mut templates = row![text("Templates").size(11)].spacing(5);
            for (index, template) in self.table_templates.iter().enumerate() {
                templates = templates.push(
                    container(
                        row![
                            button(text(&template.name).size(10))
                                .height(24)
                                .padding([4, 7])
                                .style(button::text)
                                .on_press(Message::ApplyTableTemplate(index)),
                            button(container(text("×").size(10)).center(Fill))
                                .width(22)
                                .height(24)
                                .padding(0)
                                .style(button::text)
                                .on_press(Message::DeleteTableTemplate(index)),
                        ]
                        .spacing(0)
                        .align_y(iced::Alignment::Center),
                    )
                    .style(|theme: &Theme| container::Style {
                        border: iced::Border {
                            color: theme.extended_palette().background.strong.color,
                            width: 1.0,
                            radius: iced::border::radius(5),
                        },
                        ..Default::default()
                    }),
                );
            }
            content = content.push(templates.align_y(iced::Alignment::Center));
        }
        for (index, column) in drop.columns.iter().enumerate() {
            let align_button = |icon: &'static [u8], alignment| {
                let selected =
                    std::mem::discriminant(&column.alignment) == std::mem::discriminant(&alignment);
                button(
                    container(
                        svg(svg::Handle::from_memory(icon))
                            .width(14)
                            .height(14)
                            .style(move |theme: &Theme, _status: svg::Status| svg::Style {
                                color: Some(if selected {
                                    Color::WHITE
                                } else {
                                    theme.palette().text
                                }),
                            }),
                    )
                    .center(Fill),
                )
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
                        align_button(
                            include_bytes!("../../../assets/format-justify-left-symbolic.svg"),
                            HorizontalAlign::Left
                        ),
                        align_button(
                            include_bytes!("../../../assets/format-justify-center-symbolic.svg"),
                            HorizontalAlign::Center
                        ),
                        align_button(
                            include_bytes!("../../../assets/format-justify-right-symbolic.svg"),
                            HorizontalAlign::Right
                        ),
                    ]
                    .spacing(3)
                    .width(92),
                    row![
                        button(container(text("↑").size(11)).center(Fill))
                            .width(26)
                            .height(24)
                            .padding(0)
                            .style(common::style_button(4.0))
                            .on_press_maybe(
                                (index > 0).then_some(Message::MoveDroppedColumnUp(index)),
                            ),
                        button(container(text("↓").size(11)).center(Fill))
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
            let value_types = [
                "Expression",
                "Text",
                "Integer",
                "Double",
                "Boolean",
                "Date",
                "DateTime",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
            let mut format_row = row![
                Space::new().width(110),
                pick_list(
                    value_types,
                    Some(table_templates::value_type_name(column.value_type).to_string()),
                    move |value| Message::DroppedColumnValueTypeChanged(index, value),
                )
                .width(105)
                .text_size(10)
                .padding(4),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center);
            if matches!(column.value_type, ValueType::Integer | ValueType::Double) {
                format_row = format_row
                    .push(
                        text_input("Decimals", &column.decimal_places)
                            .width(72)
                            .size(10)
                            .padding(4)
                            .on_input(move |value| {
                                Message::DroppedColumnDecimalsChanged(index, value)
                            }),
                    )
                    .push(
                        toggler(column.grouping)
                            .label("Group")
                            .text_size(10)
                            .size(14)
                            .spacing(4)
                            .on_toggle(move |value| {
                                Message::DroppedColumnGroupingChanged(index, value)
                            }),
                    );
            }
            if matches!(column.value_type, ValueType::Date | ValueType::DateTime) {
                format_row = format_row.push(
                    text_input("dd.MM.yyyy", &column.date_pattern)
                        .width(112)
                        .size(10)
                        .padding(4)
                        .on_input(move |value| {
                            Message::DroppedColumnDatePatternChanged(index, value)
                        }),
                );
            }
            format_row = format_row
                .push(
                    text_input("Prefix", &column.prefix)
                        .width(78)
                        .size(10)
                        .padding(4)
                        .on_input(move |value| Message::DroppedColumnPrefixChanged(index, value)),
                )
                .push(
                    text_input("Suffix", &column.suffix)
                        .width(78)
                        .size(10)
                        .padding(4)
                        .on_input(move |value| Message::DroppedColumnSuffixChanged(index, value)),
                );
            content = content.push(format_row);
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
            );
        let actions = row![
            Space::new().width(Fill),
            button(container(text("Cancel").size(12)).center(Fill))
                .width(82)
                .height(30)
                .padding(0)
                .style(common::style_button(6.0))
                .on_press(Message::CancelDataFieldDrop),
            button(container(text("Data only").size(12)).center(Fill))
                .width(92)
                .height(30)
                .padding(0)
                .style(common::style_button(6.0))
                .on_press(Message::CreateDroppedDataFields(false)),
            button(container(text("Header + Data").size(12)).center(Fill))
                .width(112)
                .height(30)
                .padding(0)
                .style(common::style_button(6.0))
                .on_press(Message::CreateDroppedDataFields(true)),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        dialog_container(
            iced::widget::column![
                scrollable(content.padding(18)).height(Fill),
                container(actions).padding([8, 18]).width(Fill),
            ]
            .height(560),
            820.0,
        )
    }
}

fn parameter_type_name(value_type: ReportParameterType) -> &'static str {
    match value_type {
        ReportParameterType::Text => "Text",
        ReportParameterType::Integer => "Integer",
        ReportParameterType::Double => "Double",
        ReportParameterType::Boolean => "Boolean",
        ReportParameterType::Date => "Date",
        ReportParameterType::DateTime => "DateTime",
    }
}

pub(super) fn query_text_context_popup(position: Point) -> Element<'static, Message> {
    let action = |label, message| {
        button(text(label).size(12))
            .height(28)
            .padding([5, 10])
            .style(button::text)
            .on_press(message)
    };
    let actions = iced::widget::column![
        action("Copy", Message::CopyQueryText),
        action("Paste", Message::PasteQueryText),
        action("Cut", Message::CutQueryText),
        popup_menu_separator(),
        action("Select all", Message::SelectAllQueryText),
    ]
    .spacing(1)
    .width(iced::Shrink);
    let popup = container(opaque(
        container(actions).padding(4).style(popup_menu_style),
    ))
    .padding(iced::Padding {
        top: position.y.max(0.0),
        right: 0.0,
        bottom: 0.0,
        left: position.x.max(0.0),
    })
    .align_x(iced::alignment::Horizontal::Left)
    .align_y(iced::alignment::Vertical::Top)
    .width(Fill)
    .height(Fill);
    stack![
        mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::CloseQueryTextMenu),
        popup,
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_query_value_types_and_date_names() {
        assert_eq!(
            infer_query_value_type("quantity", Some(&Value::Number(3.0))),
            ValueType::Integer
        );
        assert_eq!(
            infer_query_value_type("price", Some(&Value::Number(12.5))),
            ValueType::Double
        );
        assert_eq!(
            infer_query_value_type("birthday", Some(&Value::String("2026-09-01".into()))),
            ValueType::Date
        );
        assert_eq!(
            infer_query_value_type(
                "created_at",
                Some(&Value::String("2026-09-01 14:30:00".into()))
            ),
            ValueType::DateTime
        );
        assert_eq!(
            infer_query_value_type("is_active", Some(&Value::Number(1.0))),
            ValueType::Boolean
        );
    }

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
            tab: DataQueryTab::Sql,
            parameters: Vec::new(),
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
                tab: DataQueryTab::Sql,
                parameters: Vec::new(),
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
                tab: DataQueryTab::Sql,
                parameters: Vec::new(),
            },
        )
        .unwrap_err();

        assert_eq!(error, "Another query already uses this name");
    }

    #[test]
    fn detects_unique_named_parameters_outside_sql_literals() {
        let mut editor = DataQueryEditor::new(0, &[]);
        editor.sql = text_editor::Content::with_text(
            "SELECT * FROM visits WHERE date >= :date_from AND doctor = :doctor AND note = ':ignored' AND date <= :date_from",
        );

        editor.sync_parameters();

        assert_eq!(
            editor
                .parameters
                .iter()
                .map(|parameter| parameter.name.as_str())
                .collect::<Vec<_>>(),
            vec!["date_from", "doctor"]
        );
    }
}
