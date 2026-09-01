use crate::datasource::{ReportContext, Row, Value};

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
}
