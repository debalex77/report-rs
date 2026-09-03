use std::collections::HashMap;
use std::path::Path;

use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, OpenFlags};

use super::context::{Row, Value};
use super::provider::{DataProvider, DataSourceError};

/// SQLite-backed runtime data provider.
pub struct SqliteDataProvider {
    connection: Connection,
}

impl SqliteDataProvider {
    pub fn open(source: &str, path: &Path) -> Result<Self, DataSourceError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|error| DataSourceError::Sqlite {
            data_source: source.to_string(),
            error,
        })?;
        Ok(Self { connection })
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn database_path() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "report-rs-sqlite-{}-{nonce}.db",
            std::process::id()
        ))
    }

    #[test]
    fn sqlite_query_converts_supported_values() {
        let path = database_path();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE items (name TEXT, quantity INTEGER, price REAL, note TEXT);\
                 INSERT INTO items VALUES ('Coffee', 3, 12.5, NULL);",
            )
            .unwrap();
        drop(connection);
        let provider = SqliteDataProvider::open("main", &path).unwrap();

        let rows = provider
            .query(
                "main",
                "items",
                "SELECT name, quantity, price, note FROM items",
            )
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0].get("name"), Some(Value::String(value)) if value == "Coffee"));
        assert!(matches!(rows[0].get("quantity"), Some(Value::Number(value)) if *value == 3.0));
        assert!(matches!(rows[0].get("price"), Some(Value::Number(value)) if *value == 12.5));
        assert!(matches!(rows[0].get("note"), Some(Value::Null)));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn sqlite_query_preserves_blob_bytes() {
        let path = database_path();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute("CREATE TABLE images (content BLOB)", [])
            .unwrap();
        connection
            .execute("INSERT INTO images VALUES (?1)", [vec![1_u8, 2, 3, 4]])
            .unwrap();
        drop(connection);
        let provider = SqliteDataProvider::open("main", &path).unwrap();

        let rows = provider
            .query("main", "images", "SELECT content FROM images")
            .unwrap();

        assert!(
            matches!(rows[0].get("content"), Some(Value::Blob(value)) if value == &[1, 2, 3, 4])
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn sqlite_provider_does_not_create_missing_database() {
        let path = database_path();

        assert!(SqliteDataProvider::open("missing", &path).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn sqlite_fields_are_available_for_an_empty_result() {
        let path = database_path();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE items (id INTEGER, name TEXT);")
            .unwrap();
        drop(connection);
        let provider = SqliteDataProvider::open("main", &path).unwrap();

        let fields = provider
            .fields("main", "items", "SELECT id, name FROM items")
            .unwrap();

        assert_eq!(fields, vec!["id", "name"]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn sqlite_query_binds_named_report_parameters() {
        let path = database_path();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE items (name TEXT, quantity INTEGER);\
                 INSERT INTO items VALUES ('Coffee', 3), ('Tea', 1);",
            )
            .unwrap();
        drop(connection);
        let provider = SqliteDataProvider::open("main", &path).unwrap();
        let parameters = HashMap::from([("minimum".to_string(), Value::Number(2.0))]);

        let rows = provider
            .query_with_parameters(
                "main",
                "items",
                "SELECT name FROM items WHERE quantity >= :minimum",
                &parameters,
            )
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0].get("name"), Some(Value::String(value)) if value == "Coffee"));
        std::fs::remove_file(path).unwrap();
    }
}

impl DataProvider for SqliteDataProvider {
    fn fields(
        &self,
        source: &str,
        _query_name: &str,
        sql: &str,
    ) -> Result<Vec<String>, DataSourceError> {
        let statement = self
            .connection
            .prepare(sql)
            .map_err(|error| DataSourceError::Sqlite {
                data_source: source.to_string(),
                error,
            })?;
        Ok(statement
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect())
    }

    fn query(
        &self,
        source: &str,
        _query_name: &str,
        sql: &str,
    ) -> Result<Vec<Row>, DataSourceError> {
        let mut statement =
            self.connection
                .prepare(sql)
                .map_err(|error| DataSourceError::Sqlite {
                    data_source: source.to_string(),
                    error,
                })?;
        let columns = statement
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut result = Vec::new();
        let mut rows = statement
            .query([])
            .map_err(|error| DataSourceError::Sqlite {
                data_source: source.to_string(),
                error,
            })?;
        while let Some(row) = rows.next().map_err(|error| DataSourceError::Sqlite {
            data_source: source.to_string(),
            error,
        })? {
            let mut values = Row::new();
            for (index, column) in columns.iter().enumerate() {
                let value = match row
                    .get_ref(index)
                    .map_err(|error| DataSourceError::Sqlite {
                        data_source: source.to_string(),
                        error,
                    })? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(value) => Value::Number(value as f64),
                    ValueRef::Real(value) => Value::Number(value),
                    ValueRef::Text(value) => {
                        Value::String(String::from_utf8_lossy(value).into_owned())
                    }
                    ValueRef::Blob(value) => Value::Blob(value.to_vec()),
                };
                values.insert(column.clone(), value);
            }
            result.push(values);
        }
        Ok(result)
    }

    fn query_with_parameters(
        &self,
        source: &str,
        _query_name: &str,
        sql: &str,
        parameters: &HashMap<String, Value>,
    ) -> Result<Vec<Row>, DataSourceError> {
        let mut statement =
            self.connection
                .prepare(sql)
                .map_err(|error| DataSourceError::Sqlite {
                    data_source: source.to_string(),
                    error,
                })?;
        for index in 1..=statement.parameter_count() {
            let sql_name = statement.parameter_name(index).unwrap_or("").to_string();
            let name = sql_name.strip_prefix([':', '@', '$']).unwrap_or(&sql_name);
            let Some(value) = parameters.get(name) else {
                return Err(DataSourceError::MissingParameter {
                    parameter: name.to_string(),
                });
            };
            let value = match value {
                Value::String(value) => SqlValue::Text(value.clone()),
                Value::Number(value) => SqlValue::Real(*value),
                Value::Bool(value) => SqlValue::Integer(i64::from(*value)),
                Value::Blob(value) => SqlValue::Blob(value.clone()),
                Value::Null => SqlValue::Null,
            };
            statement
                .raw_bind_parameter(index, value)
                .map_err(|error| DataSourceError::Sqlite {
                    data_source: source.to_string(),
                    error,
                })?;
        }
        let columns = statement
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut result = Vec::new();
        let mut rows = statement.raw_query();
        while let Some(row) = rows.next().map_err(|error| DataSourceError::Sqlite {
            data_source: source.to_string(),
            error,
        })? {
            let mut values = Row::new();
            for (index, column) in columns.iter().enumerate() {
                let value = match row
                    .get_ref(index)
                    .map_err(|error| DataSourceError::Sqlite {
                        data_source: source.to_string(),
                        error,
                    })? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(value) => Value::Number(value as f64),
                    ValueRef::Real(value) => Value::Number(value),
                    ValueRef::Text(value) => {
                        Value::String(String::from_utf8_lossy(value).into_owned())
                    }
                    ValueRef::Blob(value) => Value::Blob(value.to_vec()),
                };
                values.insert(column.clone(), value);
            }
            result.push(values);
        }
        Ok(result)
    }
}
