use crate::datasource::{ReportContext, Row, Value};
use crate::model::{ValueFormat, ValueType};
use chrono::{NaiveDate, NaiveDateTime};

/// Resolves `${...}` references in a report text expression.
///
/// Supported references:
/// - `${field}`: field from the current DataBand row, then a global variable;
/// - `${Query.field}`: field from the first row of a named query;
/// - `${parameter.name}`: caller-supplied report parameter.
///
/// Unknown or malformed references are preserved in the output so mistakes are
/// visible in Preview instead of silently becoming empty text.
pub fn evaluate(template: &str, row: Option<&Row>, context: &ReportContext) -> String {
    evaluate_for_query(template, row, context, None)
}

/// Resolves an expression while identifying the query represented by the
/// current DataBand row. A qualified reference to that same query uses the
/// current row instead of always reading the query's first row.
pub fn evaluate_for_query(
    template: &str,
    row: Option<&Row>,
    context: &ReportContext,
    current_query: Option<&str>,
) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("${") {
        result.push_str(&remaining[..start]);
        let expression = &remaining[start + 2..];
        let Some(end) = expression.find('}') else {
            result.push_str(&remaining[start..]);
            return result;
        };
        let reference = &expression[..end];
        if let Some(value) = resolve_reference(reference, row, context, current_query) {
            result.push_str(&value.as_string());
        } else {
            result.push_str("${");
            result.push_str(reference);
            result.push('}');
        }
        remaining = &expression[end + 1..];
    }

    result.push_str(remaining);
    result
}

/// Resolves an expression and applies the display format owned by a TextItem.
pub fn evaluate_formatted_for_query(
    template: &str,
    row: Option<&Row>,
    context: &ReportContext,
    current_query: Option<&str>,
    value_type: ValueType,
    format: &ValueFormat,
) -> String {
    let trimmed = template.trim();
    let value = trimmed
        .strip_prefix("${")
        .and_then(|reference| reference.strip_suffix('}'))
        .filter(|reference| !reference.contains("${"))
        .and_then(|reference| resolve_reference(reference, row, context, current_query));

    let raw = value
        .map(Value::as_string)
        .unwrap_or_else(|| evaluate_for_query(template, row, context, current_query));
    format_value(value, &raw, value_type, format)
}

fn format_value(
    value: Option<&Value>,
    raw: &str,
    value_type: ValueType,
    format: &ValueFormat,
) -> String {
    if format.is_default() {
        return raw.to_string();
    }
    let formatted = match value_type {
        ValueType::Integer | ValueType::Double => {
            let number = match value {
                Some(Value::Number(number)) => Some(*number),
                Some(Value::String(number)) => number.parse().ok(),
                _ => raw.parse().ok(),
            };
            number
                .map(|number| format_number(number, value_type, format))
                .unwrap_or_else(|| raw.to_string())
        }
        ValueType::Boolean => match value {
            Some(Value::Bool(value)) => value.to_string(),
            _ => raw.to_string(),
        },
        ValueType::Date | ValueType::DateTime => {
            format_date(raw, value_type, format).unwrap_or_else(|| raw.to_string())
        }
        ValueType::Text | ValueType::Expression => raw.to_string(),
    };
    format!("{}{}{}", format.prefix, formatted, format.suffix)
}

fn format_number(number: f64, value_type: ValueType, format: &ValueFormat) -> String {
    let mut value = if let Some(decimals) = format.decimal_places {
        format!("{number:.decimals$}", decimals = decimals as usize)
    } else if value_type == ValueType::Integer {
        format!("{number:.0}")
    } else {
        number.to_string()
    };
    if format.grouping {
        let (integer, fraction) = value.split_once('.').unwrap_or((&value, ""));
        let (sign, digits) = integer
            .strip_prefix('-')
            .map(|digits| ("-", digits))
            .unwrap_or(("", integer));
        let mut grouped = String::with_capacity(integer.len() + integer.len() / 3);
        grouped.push_str(sign);
        for (index, character) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                grouped.push(' ');
            }
            grouped.push(character);
        }
        if !fraction.is_empty() {
            grouped.push('.');
            grouped.push_str(fraction);
        }
        value = grouped;
    }
    value
}

fn format_date(raw: &str, value_type: ValueType, format: &ValueFormat) -> Option<String> {
    let pattern = format.date_pattern.as_deref()?;
    let chrono_pattern = pattern
        .replace("yyyy", "%Y")
        .replace("MM", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S");
    if value_type == ValueType::Date {
        let date = NaiveDate::parse_from_str(raw.get(..10)?, "%Y-%m-%d").ok()?;
        Some(date.format(&chrono_pattern).to_string())
    } else {
        let normalized = raw.replace('T', " ");
        let date_time = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"]
            .into_iter()
            .find_map(|input| NaiveDateTime::parse_from_str(&normalized, input).ok())?;
        Some(date_time.format(&chrono_pattern).to_string())
    }
}

fn resolve_reference<'a>(
    reference: &str,
    row: Option<&'a Row>,
    context: &'a ReportContext,
    current_query: Option<&str>,
) -> Option<&'a Value> {
    if let Some(parameter) = reference.strip_prefix("parameter.") {
        return context.parameter(parameter);
    }

    if let Some((query, field)) = reference.split_once('.') {
        if query.is_empty() || field.is_empty() || field.contains('.') {
            return None;
        }
        if current_query == Some(query) {
            return row?.get(field);
        }
        return context.table(query)?.first()?.get(field);
    }

    row.and_then(|row| row.get(reference))
        .or_else(|| context.variable(reference))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_current_row_and_named_query_fields() {
        let mut current = Row::new();
        current.insert("patient".into(), Value::String("Ana".into()));
        let mut lookup = Row::new();
        lookup.insert("doctor".into(), Value::String("Medic șef".into()));
        let mut context = ReportContext::new();
        context.add_table("QueryStaff", vec![lookup]);

        let value = evaluate(
            "Pacient: ${patient}; aprobat: ${QueryStaff.doctor}",
            Some(&current),
            &context,
        );

        assert_eq!(value, "Pacient: Ana; aprobat: Medic șef");
    }

    #[test]
    fn named_query_uses_its_first_row() {
        let mut first = Row::new();
        first.insert("total".into(), Value::Number(10.5));
        let mut second = Row::new();
        second.insert("total".into(), Value::Number(99.0));
        let mut context = ReportContext::new();
        context.add_table("Totals", vec![first, second]);

        assert_eq!(evaluate("${Totals.total}", None, &context), "10.5");
    }

    #[test]
    fn qualified_current_query_uses_the_current_data_band_row() {
        let mut first = Row::new();
        first.insert("patient".into(), Value::String("Ana".into()));
        let mut current = Row::new();
        current.insert("patient".into(), Value::String("Mihai".into()));
        let mut context = ReportContext::new();
        context.add_table("QueryPatients", vec![first]);

        assert_eq!(
            evaluate_for_query(
                "${QueryPatients.patient}",
                Some(&current),
                &context,
                Some("QueryPatients")
            ),
            "Mihai"
        );
    }

    #[test]
    fn resolves_parameters_and_variables() {
        let mut context = ReportContext::new();
        context.set_parameter("subtitle", Value::String("August".into()));
        context.set_variable("page", Value::Number(2.0));

        assert_eq!(
            evaluate("${parameter.subtitle} / ${page}", None, &context),
            "August / 2"
        );
    }

    #[test]
    fn preserves_unknown_and_malformed_references() {
        let context = ReportContext::new();

        assert_eq!(
            evaluate("${Missing.value} ${unknown} ${broken", None, &context),
            "${Missing.value} ${unknown} ${broken"
        );
    }

    #[test]
    fn formats_numeric_query_value() {
        let mut row = Row::new();
        row.insert("total".into(), Value::Number(12345.678));
        let context = ReportContext::new();
        let format = ValueFormat {
            decimal_places: Some(2),
            prefix: "$ ".into(),
            suffix: " MDL".into(),
            grouping: true,
            ..ValueFormat::default()
        };

        assert_eq!(
            evaluate_formatted_for_query(
                "${total}",
                Some(&row),
                &context,
                None,
                ValueType::Double,
                &format,
            ),
            "$ 12 345.68 MDL"
        );
    }

    #[test]
    fn formats_iso_date_with_report_pattern() {
        let mut row = Row::new();
        row.insert("birthday".into(), Value::String("2026-09-01".into()));
        let context = ReportContext::new();
        let format = ValueFormat {
            date_pattern: Some("dd.MM.yyyy".into()),
            ..ValueFormat::default()
        };

        assert_eq!(
            evaluate_formatted_for_query(
                "${birthday}",
                Some(&row),
                &context,
                None,
                ValueType::Date,
                &format,
            ),
            "01.09.2026"
        );
    }

    #[test]
    fn empty_format_preserves_previous_expression_output() {
        let mut row = Row::new();
        row.insert("total".into(), Value::Number(10.5));
        let context = ReportContext::new();

        assert_eq!(
            evaluate_formatted_for_query(
                "${total}",
                Some(&row),
                &context,
                None,
                ValueType::Double,
                &ValueFormat::default(),
            ),
            "10.5"
        );
    }
}
