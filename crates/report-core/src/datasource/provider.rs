use std::path::Path;

use crate::model::{DataConnection, Report};

use super::context::{ReportContext, Row};
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
                    let rows = provider.query(&source.name, &query.name, &query.sql)?;
                    context.add_table(&query.name, rows);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::datasource::{ReportContext, Value};
    use crate::model::{DataConnection, DataQuery, DataSourceDefinition, Report};

    use super::*;

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
