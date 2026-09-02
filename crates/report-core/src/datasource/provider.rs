use std::cmp::Ordering;
use std::path::Path;

use crate::model::{
    DataConnection, DataQuery, FilterOperator, QueryFilter, QuerySort, Report, SortDirection,
};

use super::context::{ReportContext, Row, Value};
use super::sqlite::SqliteDataProvider;

/// Error returned while opening or querying a declarative report data source.
#[derive(Debug, thiserror::Error)]
pub enum DataSourceError {
    #[error("SQLite data source `{data_source}` failed: {error}")]
    Sqlite {
        data_source: String,
        #[source]
        error: rusqlite::Error,
    },
    #[error(
        "query `{query}` from data source `{data_source}` returned an unsupported BLOB value in column `{column}`"
    )]
    UnsupportedBlob {
        data_source: String,
        query: String,
        column: String,
    },
}

/// Runtime adapter capable of executing a query and returning report rows.
pub trait DataProvider {
    /// Returns the column names produced by a query, even when it has no rows.
    fn fields(
        &self,
        source: &str,
        query_name: &str,
        sql: &str,
    ) -> Result<Vec<String>, DataSourceError>;

    fn query(&self, source: &str, query_name: &str, sql: &str)
    -> Result<Vec<Row>, DataSourceError>;
}

/// Executes every declarative query and adds its result to the report context.
///
/// Relative SQLite paths are resolved against `base_dir`, normally the
/// directory containing the `.report.json` file.
pub fn load_report_data_sources(
    report: &Report,
    base_dir: &Path,
    context: &mut ReportContext,
) -> Result<(), DataSourceError> {
    for source in &report.data_sources {
        match &source.connection {
            DataConnection::Sqlite { path } => {
                let path = Path::new(path);
                let resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    base_dir.join(path)
                };
                let provider = SqliteDataProvider::open(&source.name, &resolved)?;
                for query in &source.queries {
                    let mut rows = provider.query(&source.name, &query.name, &query.sql)?;
                    apply_query_transformations(&mut rows, query);
                    context.add_table(&query.name, rows);
                }
            }
        }
    }
    Ok(())
}

pub fn apply_query_transformations(rows: &mut Vec<Row>, query: &DataQuery) {
    rows.retain(|row| {
        query
            .filters
            .iter()
            .all(|filter| matches_filter(row, filter))
    });
    if !query.sorts.is_empty() {
        rows.sort_by(|left, right| compare_rows(left, right, &query.sorts));
    }
}

fn matches_filter(row: &Row, filter: &QueryFilter) -> bool {
    let value = row.get(&filter.field);
    match filter.operator {
        FilterOperator::IsNull => value.is_none_or(|value| matches!(value, Value::Null)),
        FilterOperator::IsNotNull => value.is_some_and(|value| !matches!(value, Value::Null)),
        FilterOperator::Equal => compare_to_filter(value, filter) == Some(Ordering::Equal),
        FilterOperator::NotEqual => compare_to_filter(value, filter) != Some(Ordering::Equal),
        FilterOperator::Greater => compare_to_filter(value, filter) == Some(Ordering::Greater),
        FilterOperator::GreaterOrEqual => matches!(
            compare_to_filter(value, filter),
            Some(Ordering::Greater | Ordering::Equal)
        ),
        FilterOperator::Less => compare_to_filter(value, filter) == Some(Ordering::Less),
        FilterOperator::LessOrEqual => matches!(
            compare_to_filter(value, filter),
            Some(Ordering::Less | Ordering::Equal)
        ),
        FilterOperator::Contains | FilterOperator::StartsWith => {
            let Some(Value::String(value)) = value else {
                return false;
            };
            let (value, expected) = if filter.case_sensitive {
                (value.clone(), filter.value.clone())
            } else {
                (value.to_lowercase(), filter.value.to_lowercase())
            };
            if filter.operator == FilterOperator::Contains {
                value.contains(&expected)
            } else {
                value.starts_with(&expected)
            }
        }
    }
}

fn compare_to_filter(value: Option<&Value>, filter: &QueryFilter) -> Option<Ordering> {
    match value? {
        Value::Number(value) => value.partial_cmp(&filter.value.parse::<f64>().ok()?),
        Value::Bool(value) => Some(value.cmp(&filter.value.parse::<bool>().ok()?)),
        Value::String(value) => {
            if filter.case_sensitive {
                Some(value.as_str().cmp(filter.value.as_str()))
            } else {
                Some(value.to_lowercase().cmp(&filter.value.to_lowercase()))
            }
        }
        Value::Null => None,
    }
}

fn compare_rows(left: &Row, right: &Row, sorts: &[QuerySort]) -> Ordering {
    for sort in sorts {
        let ordering = compare_values(left.get(&sort.field), right.get(&sort.field));
        let ordering = if sort.direction == SortDirection::Descending {
            ordering.reverse()
        } else {
            ordering
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

fn compare_values(left: Option<&Value>, right: Option<&Value>) -> Ordering {
    match (left, right) {
        (Some(Value::Number(left)), Some(Value::Number(right))) => left.total_cmp(right),
        (Some(Value::Bool(left)), Some(Value::Bool(right))) => left.cmp(right),
        (Some(Value::String(left)), Some(Value::String(right))) => left.cmp(right),
        (Some(Value::Null) | None, Some(Value::Null) | None) => Ordering::Equal,
        (Some(Value::Null) | None, _) => Ordering::Less,
        (_, Some(Value::Null) | None) => Ordering::Greater,
        (Some(left), Some(right)) => left.as_string().cmp(&right.as_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::datasource::{ReportContext, Value};
    use crate::model::{DataConnection, DataQuery, DataSourceDefinition, Report};

    use super::*;

    #[test]
    fn filters_and_sorts_rows_before_they_reach_layout() {
        let mut rows = Vec::new();
        for (name, total) in [("Tea", 8.0), ("Coffee", 15.0), ("Coffee XL", 22.0)] {
            let mut row = Row::new();
            row.insert("name".into(), Value::String(name.into()));
            row.insert("total".into(), Value::Number(total));
            rows.push(row);
        }
        let query = DataQuery {
            name: "orders".into(),
            sql: "SELECT name, total FROM orders".into(),
            filters: vec![
                QueryFilter {
                    field: "name".into(),
                    operator: FilterOperator::Contains,
                    value: "coffee".into(),
                    case_sensitive: false,
                },
                QueryFilter {
                    field: "total".into(),
                    operator: FilterOperator::Greater,
                    value: "10".into(),
                    case_sensitive: false,
                },
            ],
            sorts: vec![QuerySort {
                field: "total".into(),
                direction: SortDirection::Descending,
            }],
        };

        apply_query_transformations(&mut rows, &query);

        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0].get("total"), Some(Value::Number(22.0))));
        assert!(matches!(rows[1].get("total"), Some(Value::Number(15.0))));
    }

    #[test]
    fn declarative_sqlite_query_populates_named_context_table() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "report-rs-data-source-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let database = directory.join("orders.sqlite");
        let connection = rusqlite::Connection::open(&database).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE orders (id INTEGER, total REAL);\
                     INSERT INTO orders VALUES (1, 42.75);",
            )
            .unwrap();
        drop(connection);
        let report = Report {
            name: "Database report".to_string(),
            data_sources: vec![DataSourceDefinition {
                name: "main".to_string(),
                connection: DataConnection::Sqlite {
                    path: "orders.sqlite".to_string(),
                },
                queries: vec![DataQuery {
                    name: "orders".to_string(),
                    sql: "SELECT id, total FROM orders".to_string(),
                    filters: Vec::new(),
                    sorts: Vec::new(),
                }],
            }],
            pages: Vec::new(),
        };
        let mut context = ReportContext::new();

        load_report_data_sources(&report, &directory, &mut context).unwrap();

        let orders = context.table("orders").unwrap();
        assert_eq!(orders.len(), 1);
        assert!(matches!(orders[0].get("id"), Some(Value::Number(value)) if *value == 1.0));
        assert!(matches!(orders[0].get("total"), Some(Value::Number(value)) if *value == 42.75));

        std::fs::remove_file(database).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
