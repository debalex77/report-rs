use std::collections::HashMap;

/// Represents a value that can be stored in a report data source
/// or used as a report variable.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

/// Represents a single data row.
///
/// Each field is stored by name and associated with a [`Value`].
pub type Row = HashMap<String, Value>;

/// Runtime data available while a report is being rendered.
///
/// A context can contain:
/// - named tables used by data bands;
/// - global variables used in text expressions;
/// - input parameters supplied by the calling application.
#[derive(Debug, Default)]
pub struct ReportContext {
    tables: HashMap<String, Vec<Row>>,
    variables: HashMap<String, Value>,
    parameters: HashMap<String, Value>,
}

impl ReportContext {
    /// Creates an empty report context.
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            variables: HashMap::new(),
            parameters: HashMap::new(),
        }
    }

    /// Adds or replaces a global report variable.
    pub fn set_variable(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// Returns a global variable by name.
    pub fn variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Returns all global report variables.
    pub fn variables(&self) -> &HashMap<String, Value> {
        &self.variables
    }

    /// Adds or replaces an input parameter.
    pub fn set_parameter(&mut self, name: &str, value: Value) {
        self.parameters.insert(name.to_string(), value);
    }

    /// Returns an input parameter by name.
    pub fn parameter(&self, name: &str) -> Option<&Value> {
        self.parameters.get(name)
    }

    /// Returns all input parameters.
    pub fn parameters(&self) -> &HashMap<String, Value> {
        &self.parameters
    }

    /// Adds or replaces a named table.
    ///
    /// Each row is represented as a map of field names to values.
    pub fn add_table(&mut self, name: &str, rows: Vec<Row>) {
        self.tables.insert(name.to_string(), rows);
    }

    /// Returns a table by name.
    pub fn table(&self, name: &str) -> Option<&Vec<Row>> {
        self.tables.get(name)
    }
}

impl Value {
    /// Converts the value to the textual representation used
    /// when inserting data into report text.
    pub fn as_string(&self) -> String {
        match self {
            Value::String(value) => value.clone(),
            Value::Number(value) => value.to_string(),
            Value::Bool(value) => value.to_string(),
            // Null values are rendered as empty text.
            Value::Null => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get_table() {
        let mut row = Row::new();

        row.insert("name".to_string(), Value::String("Ion Popescu".to_string()));
        row.insert("age".to_string(), Value::Number(42.0));

        let mut context = ReportContext::new();

        context.add_table("patients", vec![row]);

        let patients = context.table("patients").unwrap();

        assert_eq!(patients.len(), 1);
    }

    #[test]
    fn set_and_get_parameter() {
        let mut context = ReportContext::new();

        context.set_parameter("clinic", Value::String("Clinica Centrală".to_string()));

        match context.parameter("clinic") {
            Some(Value::String(value)) => assert_eq!(value, "Clinica Centrală"),
            _ => panic!("expected a string parameter"),
        }
    }
}
