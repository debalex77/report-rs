use super::*;

#[test]
fn serialize_report() {
    let report = Report {
        name: "Test report".to_string(),
        data_sources: Vec::new(),

        pages: vec![Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![
                Band {
                    kind: BandKind::ReportHeader,
                    height: Mm(20.0),

                    items: vec![
                        Item::Text(TextItem {
                            name: String::new(),
                            x: Mm(10.0),
                            y: Mm(5.0),
                            width: Mm(190.0),
                            height: Mm(10.0),

                            text: "Raport ecografic".to_string(),

                            value_type: ValueType::Text,

                            query_source: QuerySource::Main,

                            field: None,

                            font_size: 14.0,
                            font_family: default_font_family(),

                            bold: false,
                            italic: false,

                            underline: false,

                            strikeout: false,

                            text_color: default_text_color(),

                            horizontal_align: HorizontalAlign::Center,
                            vertical_align: VerticalAlign::Center,

                            word_wrap: false,
                            auto_height: false,

                            padding: Padding::default(),

                            background: None,
                            border: None,
                        }),
                        Item::Line(LineItem {
                            name: String::new(),
                            x1: Mm(10.0),
                            y1: Mm(18.0),
                            x2: Mm(200.0),
                            y2: Mm(18.0),
                            width: Mm(0.5),
                        }),
                    ],
                },
                Band {
                    kind: BandKind::Data {
                        source: "items".to_string(),
                    },
                    height: Mm(10.0),
                    items: vec![],
                },
            ],
        }],
    };

    let json = serde_json::to_string_pretty(&report).unwrap();

    println!("{json}");

    assert!(json.contains("Test report"));
    assert!(json.contains("Raport ecografic"));
}

#[test]
fn deserialize_report() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/simple.report.json"
    );

    let report = Report::from_file(path).unwrap();

    println!("{:#?}", report);

    assert_eq!(report.name, "Structura operațională HoReCa");
    assert_eq!(report.pages.len(), 1);
}

#[test]
fn legacy_text_item_uses_literal_main_query_defaults() {
    let json = r#"
    {
        "type": "Text",
        "x": 0.0,
        "y": 0.0,
        "width": 40.0,
        "height": 8.0,
        "text": "Legacy text",
        "font_size": 12.0,
        "horizontal_align": "Left",
        "vertical_align": "Center",
        "word_wrap": false,
        "auto_height": false
    }
    "#;

    let item: Item = serde_json::from_str(json).unwrap();
    let Item::Text(text) = item else {
        panic!("expected a text item");
    };

    assert_eq!(text.value_type, ValueType::Text);
    assert_eq!(text.query_source, QuerySource::Main);
    assert_eq!(text.field, None);
}

#[test]
fn query_bound_text_item_json_round_trip() {
    let json = r#"
    {
        "type": "Text",
        "x": 0.0,
        "y": 0.0,
        "width": 40.0,
        "height": 8.0,
        "text": "",
        "value_type": "Double",
        "query_source": { "Named": "totals" },
        "field": "amount",
        "font_size": 12.0,
        "horizontal_align": "Right",
        "vertical_align": "Center",
        "word_wrap": false,
        "auto_height": false
    }
    "#;

    let item: Item = serde_json::from_str(json).unwrap();
    let Item::Text(text) = &item else {
        panic!("expected a text item");
    };

    assert_eq!(text.value_type, ValueType::Double);
    assert_eq!(text.query_source, QuerySource::Named("totals".to_string()));
    assert_eq!(text.field.as_deref(), Some("amount"));

    let serialized = serde_json::to_string(&item).unwrap();
    let _: Item = serde_json::from_str(&serialized).unwrap();
}

#[test]
fn save_report_to_file() {
    let report = Report {
        name: "Saved report".to_string(),
        data_sources: Vec::new(),

        pages: vec![Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,

            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },

            bands: vec![],
        }],
    };

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/saved.report.json"
    );

    report.save_to_file(path).unwrap();
}

#[test]
fn page_dimensions() {
    let page = Page {
        size: PageSize::A4,
        orientation: Orientation::Portrait,

        margins: Margins {
            left: Mm(10.0),
            top: Mm(10.0),
            right: Mm(10.0),
            bottom: Mm(10.0),
        },

        bands: vec![],
    };

    assert_eq!(page.width(), Mm(210.0));
    assert_eq!(page.height(), Mm(297.0));

    assert_eq!(page.printable_width(), Mm(190.0));
    assert_eq!(page.printable_height(), Mm(277.0));
}

#[test]
fn landscape_page_dimensions() {
    let page = Page {
        size: PageSize::A4,
        orientation: Orientation::Landscape,
        margins: Margins {
            left: Mm(10.0),
            top: Mm(10.0),
            right: Mm(10.0),
            bottom: Mm(10.0),
        },
        bands: vec![],
    };

    assert_eq!(page.width(), Mm(297.0));
    assert_eq!(page.height(), Mm(210.0));
    assert_eq!(page.printable_width(), Mm(277.0));
    assert_eq!(page.printable_height(), Mm(190.0));
}

#[test]
fn image_item_json_round_trip() {
    let json = r#"
    {
        "type": "Image",
        "x": 10.0,
        "y": 5.0,
        "width": 40.0,
        "height": 30.0,
        "source": "images/logo.png"
    }
    "#;

    let item: Item = serde_json::from_str(json).unwrap();

    match &item {
        Item::Image(image) => {
            assert_eq!(image.x, Mm(10.0));
            assert_eq!(image.y, Mm(5.0));
            assert_eq!(image.width, Mm(40.0));
            assert_eq!(image.height, Mm(30.0));
            assert_eq!(image.source, "images/logo.png");
            assert_eq!(image.fit, ImageFit::Stretch);
        }
        _ => panic!("expected an image item"),
    }

    let serialized = serde_json::to_string(&item).unwrap();
    assert!(serialized.contains(r#""fit":"Stretch""#));
    let _: Item = serde_json::from_str(&serialized).unwrap();
}

#[test]
fn deserialize_image_item_with_contain_fit() {
    let json = r#"
        {
            "type": "Image",
            "x": 0.0,
            "y": 0.0,
            "width": 100.0,
            "height": 50.0,
            "source": "images/logo.png",
            "fit": "Contain"
        }
        "#;

    let item: Item = serde_json::from_str(json).unwrap();

    match item {
        Item::Image(image) => assert_eq!(image.fit, ImageFit::Contain),
        _ => panic!("expected an image item"),
    }
}

#[test]
fn layout_item_json_round_trip() {
    for item_type in ["HorizontalLayout", "VerticalLayout"] {
        let json = format!(
            r#"{{"type":"{item_type}","name":"layout1","x":0.0,"y":0.0,"width":100.0,"height":20.0,"items":[]}}"#
        );
        let item: Item = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            item,
            Item::HorizontalLayout(_) | Item::VerticalLayout(_)
        ));
        let serialized = serde_json::to_string(&item).unwrap();
        let _: Item = serde_json::from_str(&serialized).unwrap();
    }
}

#[test]
fn report_without_data_sources_remains_compatible() {
    let json = r#"{
            "name": "Legacy report",
            "pages": []
        }"#;

    let report: Report = serde_json::from_str(json).unwrap();

    assert!(report.data_sources.is_empty());
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("data_sources"));
}

#[test]
fn sqlite_data_source_json_round_trip() {
    let report = Report {
        name: "SQLite report".to_string(),
        data_sources: vec![DataSourceDefinition {
            name: "main".to_string(),
            connection: DataConnection::Sqlite {
                path: "data/orders.sqlite".to_string(),
            },
            queries: vec![DataQuery {
                name: "orders".to_string(),
                sql: "SELECT id, total FROM orders".to_string(),
            }],
        }],
        pages: Vec::new(),
    };

    let json = serde_json::to_string(&report).unwrap();
    let restored: Report = serde_json::from_str(&json).unwrap();

    assert!(json.contains(r#""type":"sqlite""#));
    assert_eq!(restored.data_sources, report.data_sources);
}

#[test]
fn data_header_repeats_by_default_when_deserialized() {
    let json = r#"{
        "kind": { "DataHeader": { "source": "patients" } },
        "height": 8.0,
        "items": []
    }"#;

    let band: Band = serde_json::from_str(json).unwrap();

    assert!(matches!(
        band.kind,
        BandKind::DataHeader {
            source,
            repeat_on_each_page: true
        } if source == "patients"
    ));
}
