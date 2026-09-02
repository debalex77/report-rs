use super::*;

#[derive(Clone)]
pub(crate) struct TableColumnSpec {
    pub(crate) field: String,
    pub(crate) title: String,
    pub(crate) width: String,
    pub(crate) alignment: HorizontalAlign,
    pub(crate) value_type: ValueType,
    pub(crate) decimal_places: String,
    pub(crate) date_pattern: String,
    pub(crate) prefix: String,
    pub(crate) suffix: String,
    pub(crate) grouping: bool,
}

pub(crate) fn create_query_table(
    report: &mut Report,
    target_band: usize,
    query: &str,
    columns: &[TableColumnSpec],
    include_header: bool,
    center_table: bool,
    font_family: String,
) -> Result<(), String> {
    if columns.is_empty() {
        return Err("Select at least one query field".to_string());
    }
    let page = report
        .pages
        .first_mut()
        .ok_or_else(|| "The report does not contain a page".to_string())?;
    let target_kind = page
        .bands
        .get(target_band)
        .map(|band| &band.kind)
        .ok_or_else(|| "The target band no longer exists".to_string())?;
    if !matches!(
        target_kind,
        BandKind::Data { .. } | BandKind::DataHeader { .. }
    ) {
        return Err("Drop query fields on a DataBand or DataHeader".to_string());
    }

    let widths = columns
        .iter()
        .map(|column| {
            column
                .width
                .trim()
                .parse::<f32>()
                .map_err(|_| format!("Invalid width for column {}", column.title))
                .and_then(|width| {
                    if width > 0.0 {
                        Ok(width)
                    } else {
                        Err(format!(
                            "Width must be positive for column {}",
                            column.title
                        ))
                    }
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total_width: f32 = widths.iter().sum();
    if total_width > page.printable_width().0 + 0.01 {
        return Err(format!(
            "Column widths ({total_width:.2} mm) exceed the printable page width ({:.2} mm)",
            page.printable_width().0
        ));
    }
    let table_x = if center_table {
        (page.printable_width().0 - total_width) / 2.0
    } else {
        0.0
    };

    let mut data_index = if matches!(target_kind, BandKind::Data { .. }) {
        target_band
    } else if let Some(index) = page
        .bands
        .iter()
        .position(|band| matches!(band.kind, BandKind::Data { .. }))
    {
        index
    } else {
        page.bands.insert(
            target_band + 1,
            Band {
                kind: BandKind::Data {
                    source: query.to_string(),
                },
                height: Mm(8.0),
                items: Vec::new(),
            },
        );
        target_band + 1
    };
    if !page.bands[data_index].items.is_empty() {
        return Err("The target DataBand must be empty".to_string());
    }
    page.bands[data_index].kind = BandKind::Data {
        source: query.to_string(),
    };

    if include_header {
        let header_index = page
            .bands
            .iter()
            .position(|band| matches!(band.kind, BandKind::DataHeader { .. }));
        let header_index = if let Some(index) = header_index {
            index
        } else {
            page.bands.insert(
                data_index,
                Band {
                    kind: BandKind::DataHeader {
                        source: query.to_string(),
                        repeat_on_each_page: true,
                    },
                    height: Mm(8.0),
                    items: Vec::new(),
                },
            );
            data_index += 1;
            data_index - 1
        };
        if !page.bands[header_index].items.is_empty() {
            return Err("The DataHeader must be empty".to_string());
        }
        page.bands[header_index].kind = BandKind::DataHeader {
            source: query.to_string(),
            repeat_on_each_page: true,
        };
        page.bands[header_index].height = Mm(8.0);
        page.bands[header_index].items =
            vec![table_layout(columns, &widths, &font_family, true, table_x)];
    }

    page.bands[data_index].height = Mm(8.0);
    page.bands[data_index].items =
        vec![table_layout(columns, &widths, &font_family, false, table_x)];
    ensure_unique_item_names(report);
    Ok(())
}

fn table_layout(
    columns: &[TableColumnSpec],
    widths: &[f32],
    font_family: &str,
    header: bool,
    layout_x: f32,
) -> Item {
    let mut x = 0.0;
    let items = columns
        .iter()
        .zip(widths)
        .map(|(column, &cell_width)| {
            let mut item = new_text_item(font_family.to_string());
            let Item::Text(text) = &mut item else {
                unreachable!();
            };
            text.x = Mm(x);
            text.y = Mm(0.0);
            text.width = Mm(cell_width);
            text.height = Mm(8.0);
            text.text = if header {
                column.title.clone()
            } else {
                format!("${{{}}}", column.field)
            };
            text.value_type = if header {
                ValueType::Text
            } else {
                column.value_type
            };
            if !header {
                text.value_format = ValueFormat {
                    decimal_places: parse_decimal_places(&column.decimal_places).flatten(),
                    date_pattern: (!column.date_pattern.trim().is_empty())
                        .then(|| column.date_pattern.trim().to_string()),
                    prefix: column.prefix.clone(),
                    suffix: column.suffix.clone(),
                    grouping: column.grouping,
                };
            }
            text.query_source = QuerySource::Main;
            text.bold = header;
            text.horizontal_align = column.alignment;
            text.vertical_align = VerticalAlign::Center;
            text.padding = Padding {
                left: Mm(1.0),
                top: Mm(0.5),
                right: Mm(1.0),
                bottom: Mm(0.5),
            };
            text.background = header.then_some(ReportColor::rgb(225, 228, 234));
            text.border = Some(Border {
                left: true,
                top: true,
                right: true,
                bottom: true,
                width: 0.3,
            });
            x += cell_width;
            item
        })
        .collect();
    Item::HorizontalLayout(LayoutItem {
        name: String::new(),
        x: Mm(layout_x),
        y: Mm(0.0),
        width: Mm(widths.iter().sum()),
        height: Mm(8.0),
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_with_data_band() -> Report {
        let mut report = blank_report();
        report.pages[0].bands.push(Band {
            kind: BandKind::Data {
                source: String::new(),
            },
            height: Mm(8.0),
            items: Vec::new(),
        });
        report
    }

    fn columns(fields: &[&str]) -> Vec<TableColumnSpec> {
        fields
            .iter()
            .map(|field| TableColumnSpec {
                field: (*field).to_string(),
                title: (*field).to_string(),
                width: "40.00".to_string(),
                alignment: HorizontalAlign::Left,
                value_type: ValueType::Expression,
                decimal_places: String::new(),
                date_pattern: String::new(),
                prefix: String::new(),
                suffix: String::new(),
                grouping: false,
            })
            .collect()
    }

    #[test]
    fn creates_header_and_repeating_query_row() {
        let mut report = report_with_data_band();
        let mut fields = columns(&["name", "age"]);
        fields[1].value_type = ValueType::Double;
        fields[1].decimal_places = "2".to_string();
        fields[1].suffix = " MDL".to_string();
        fields[1].grouping = true;

        create_query_table(
            &mut report,
            0,
            "Patients",
            &fields,
            true,
            true,
            "Sans".into(),
        )
        .unwrap();

        let page = &report.pages[0];
        assert_eq!(page.bands.len(), 2);
        assert!(matches!(
            &page.bands[0].kind,
            BandKind::DataHeader {
                source,
                repeat_on_each_page: true
            } if source == "Patients"
        ));
        assert!(matches!(
            &page.bands[1].kind,
            BandKind::Data { source } if source == "Patients"
        ));

        let Item::HorizontalLayout(header) = &page.bands[0].items[0] else {
            panic!("expected header layout");
        };
        let Item::HorizontalLayout(data) = &page.bands[1].items[0] else {
            panic!("expected data layout");
        };
        assert_eq!(header.items.len(), 2);
        assert_eq!(data.items.len(), 2);
        assert!((header.x.0 - 55.0).abs() < 0.01);
        assert!((data.x.0 - 55.0).abs() < 0.01);
        let Item::Text(header_name) = &header.items[0] else {
            panic!("expected header text");
        };
        let Item::Text(data_name) = &data.items[0] else {
            panic!("expected data text");
        };
        let Item::Text(data_age) = &data.items[1] else {
            panic!("expected formatted data text");
        };
        assert_eq!(header_name.text, "name");
        assert!(header_name.bold);
        assert_eq!(data_name.text, "${name}");
        assert_eq!(data_name.value_type, ValueType::Expression);
        assert!(data_name.border.is_some());
        assert_eq!(data_age.value_type, ValueType::Double);
        assert_eq!(data_age.value_format.decimal_places, Some(2));
        assert_eq!(data_age.value_format.suffix, " MDL");
        assert!(data_age.value_format.grouping);
    }

    #[test]
    fn refuses_to_overwrite_non_empty_data_band() {
        let mut report = report_with_data_band();
        report.pages[0].bands[0]
            .items
            .push(new_text_item("Sans".into()));

        let error = create_query_table(
            &mut report,
            0,
            "Patients",
            &columns(&["name"]),
            false,
            false,
            "Sans".into(),
        )
        .unwrap_err();

        assert_eq!(error, "The target DataBand must be empty");
    }
}
