use super::*;

#[test]
fn drag_delta_is_clamped_to_band_bounds() {
    let delta = constrained_delta(10.0, 30.0, 5.0, 15.0, -50.0, 100.0, 100.0, 40.0);

    assert_eq!(delta, (-10.0, 25.0));
}

#[test]
fn resize_is_clamped_and_preserves_minimum_size() {
    let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

    resize_rectangle(
        &mut x,
        &mut y,
        &mut width,
        &mut height,
        ResizeHandle::BottomRight,
        100.0,
        100.0,
        40.0,
        20.0,
    );
    assert_eq!((x, y, width, height), (10.0, 5.0, 30.0, 15.0));

    resize_rectangle(
        &mut x,
        &mut y,
        &mut width,
        &mut height,
        ResizeHandle::TopLeft,
        100.0,
        100.0,
        40.0,
        20.0,
    );
    assert_eq!((x, y, width, height), (39.0, 19.0, 1.0, 1.0));
}

#[test]
fn middle_handle_changes_only_height() {
    let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

    resize_rectangle(
        &mut x,
        &mut y,
        &mut width,
        &mut height,
        ResizeHandle::Bottom,
        12.0,
        3.0,
        100.0,
        40.0,
    );

    assert_eq!((x, y, width, height), (10.0, 5.0, 20.0, 13.0));
}

#[test]
fn side_handle_changes_only_width() {
    let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

    resize_rectangle(
        &mut x,
        &mut y,
        &mut width,
        &mut height,
        ResizeHandle::Right,
        4.0,
        12.0,
        100.0,
        40.0,
    );

    assert_eq!((x, y, width, height), (10.0, 5.0, 24.0, 10.0));
}

#[test]
fn property_geometry_is_clamped_to_band() {
    let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

    assert!(set_rectangle_geometry(
        &mut x,
        &mut y,
        &mut width,
        &mut height,
        GeometryField::Width,
        200.0,
        100.0,
        40.0,
    ));

    assert_eq!((x, y, width, height), (10.0, 5.0, 90.0, 10.0));
}

#[test]
fn text_color_hex_round_trip() {
    let color = ReportColor {
        r: 30,
        g: 140,
        b: 155,
        a: 128,
    };

    let encoded = format_report_color(color);

    assert_eq!(encoded, "#1E8C9B80");
    assert_eq!(parse_report_color(&encoded), Some(color));
    assert_eq!(parse_report_color("#not-a-color"), None);
}

#[test]
fn text_layout_options_json_round_trip() {
    let mut report = blank_report();
    let mut item = new_text_item("DejaVu Sans".to_string());
    let Item::Text(text) = &mut item else {
        unreachable!();
    };
    text.word_wrap = true;
    text.auto_height = true;
    text.underline = true;
    text.strikeout = true;
    text.padding = Padding {
        left: Mm(1.0),
        top: Mm(2.0),
        right: Mm(3.0),
        bottom: Mm(4.0),
    };
    text.background = Some(ReportColor::rgb(225, 190, 35));
    text.border = Some(Border {
        left: true,
        top: false,
        right: true,
        bottom: true,
        width: 0.75,
    });
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(20.0),
        items: vec![item],
    });

    let path = std::env::temp_dir().join(format!(
        "report-rs-text-options-{}.json",
        std::process::id()
    ));
    report
        .save_to_file(path.to_string_lossy().as_ref())
        .expect("report must serialize");
    let decoded =
        Report::from_file(path.to_string_lossy().as_ref()).expect("report must deserialize");
    std::fs::remove_file(path).expect("temporary report must be removed");
    let Item::Text(text) = &decoded.pages[0].bands[0].items[0] else {
        unreachable!();
    };

    assert!(text.word_wrap);
    assert!(text.auto_height);
    assert!(text.underline);
    assert!(text.strikeout);
    assert_eq!(text.padding.left, Mm(1.0));
    assert_eq!(text.padding.top, Mm(2.0));
    assert_eq!(text.padding.right, Mm(3.0));
    assert_eq!(text.padding.bottom, Mm(4.0));
    assert_eq!(text.background, Some(ReportColor::rgb(225, 190, 35)));
    let border = text.border.as_ref().expect("border must round-trip");
    assert!(border.left);
    assert!(!border.top);
    assert!(border.right);
    assert!(border.bottom);
    assert_eq!(border.width, 0.75);
}

#[test]
fn shape_border_width_json_round_trip() {
    let mut report = blank_report();
    let mut shape = new_shape_item();
    let Item::Rectangle(rectangle) = &mut shape else {
        unreachable!();
    };
    rectangle.border_width = Mm(2.25);
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![shape],
    });
    let path = std::env::temp_dir().join(format!(
        "report-rs-shape-options-{}.json",
        std::process::id()
    ));
    report
        .save_to_file(path.to_string_lossy().as_ref())
        .expect("report must serialize");
    let decoded =
        Report::from_file(path.to_string_lossy().as_ref()).expect("report must deserialize");
    std::fs::remove_file(path).expect("temporary report must be removed");

    let Item::Rectangle(rectangle) = &decoded.pages[0].bands[0].items[0] else {
        unreachable!();
    };
    assert_eq!(rectangle.border_width, Mm(2.25));
}

#[test]
fn auto_height_propagates_through_layout_to_repeating_bands() {
    fn layout_with_wrapped_text() -> Item {
        let mut first = new_text_item("DejaVu Sans".to_string());
        set_item_frame(&mut first, 0.0, 0.0, 30.0, 5.0);
        let mut second = new_text_item("DejaVu Sans".to_string());
        set_item_frame(&mut second, 30.0, 0.0, 30.0, 5.0);
        let mut text = new_text_item("DejaVu Sans".to_string());
        set_item_frame(&mut text, 60.0, 0.0, 30.0, 5.0);
        let Item::Text(text_item) = &mut text else {
            unreachable!();
        };
        text_item.width = Mm(30.0);
        text_item.height = Mm(5.0);
        text_item.text = "Responsabil_sdfhjbksdfhhsgdffiaghfkhgqashg".to_string();
        text_item.word_wrap = true;
        text_item.auto_height = true;

        Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(90.0),
            height: Mm(5.0),
            items: vec![first, second, text],
        })
    }

    let mut report = blank_report();
    report.pages[0].bands = vec![
        Band {
            kind: BandKind::PageHeader,
            height: Mm(5.0),
            items: vec![layout_with_wrapped_text()],
        },
        Band {
            kind: BandKind::Data {
                source: "rows".to_string(),
            },
            height: Mm(5.0),
            items: vec![layout_with_wrapped_text()],
        },
        Band {
            kind: BandKind::PageFooter,
            height: Mm(5.0),
            items: vec![layout_with_wrapped_text()],
        },
    ];

    assert!(propagate_auto_heights(&mut report));
    for band in &report.pages[0].bands {
        assert!(band.height.0 > 5.0);
        let Item::HorizontalLayout(layout) = &band.items[0] else {
            unreachable!();
        };
        assert!(layout.height.0 > 5.0);
        assert_eq!(band.height, layout.height);
        assert!(
            layout
                .items
                .iter()
                .all(|item| normalized_geometry(item).3 == layout.height.0)
        );
    }
}

#[test]
fn hsv_color_conversion_uses_expected_primary_colors() {
    assert_eq!(
        hsv_to_report_color(0.0, 1.0, 1.0),
        ReportColor::rgb(255, 0, 0)
    );
    assert_eq!(
        hsv_to_report_color(120.0, 1.0, 1.0),
        ReportColor::rgb(0, 255, 0)
    );
    assert_eq!(
        hsv_to_report_color(240.0, 1.0, 1.0),
        ReportColor::rgb(0, 0, 255)
    );
}

#[test]
fn property_groups_have_expected_initial_state() {
    let groups = CollapsedGroups::default();

    assert!(!groups.is_collapsed(PropertyGroup::General));
    assert!(!groups.is_collapsed(PropertyGroup::Geometry));
    assert!(!groups.is_collapsed(PropertyGroup::TextValue));
    assert!(groups.is_collapsed(PropertyGroup::Font));
    assert!(groups.is_collapsed(PropertyGroup::TextColor));
    assert!(groups.is_collapsed(PropertyGroup::Alignment));
}

#[test]
fn millimeters_use_two_decimal_places() {
    assert_eq!(format_mm(0.0), "0.00");
    assert_eq!(format_mm(12.345), "12.35");
}

#[test]
fn font_points_use_two_decimal_places() {
    assert_eq!(format_pt(0.0), "0.00");
    assert_eq!(format_pt(12.345), "12.35");
}

#[test]
fn decimal_places_accepts_a_zero_mask() {
    assert_eq!(parse_decimal_places("00"), Some(Some(2)));
    assert_eq!(parse_decimal_places("2"), Some(Some(2)));
    assert_eq!(parse_decimal_places(""), Some(None));
    assert_eq!(parse_decimal_places("2.0"), None);
}

#[test]
fn save_path_is_normalized_to_json_extension() {
    assert_eq!(
        ensure_json_extension(PathBuf::from("report")),
        PathBuf::from("report.json")
    );
    assert_eq!(
        ensure_json_extension(PathBuf::from("report.JSON")),
        PathBuf::from("report.JSON")
    );
    assert_eq!(
        ensure_json_extension(PathBuf::from("report.txt")),
        PathBuf::from("report.json")
    );
}

#[test]
fn blank_report_contains_an_empty_a4_page() {
    let report = blank_report();

    assert_eq!(report.pages.len(), 1);
    assert!(report.pages[0].bands.is_empty());
    assert_eq!(report.pages[0].dimensions(), (Mm(210.0), Mm(297.0)));
}

#[test]
fn band_resize_changes_only_height_and_stays_inside_printable_page() {
    let mut report = blank_report();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: Vec::new(),
    });

    assert!(resize_band_height(&mut report.pages[0], 0, 12.5));
    assert_eq!(report.pages[0].bands[0].height, Mm(42.5));

    resize_band_height(&mut report.pages[0], 0, 1_000.0);
    assert_eq!(
        report.pages[0].bands[0].height,
        report.pages[0].printable_height()
    );
}

#[test]
fn band_resize_does_not_shrink_below_its_items() {
    let mut report = blank_report();
    let mut item = new_text_item("DejaVu Sans".to_string());
    set_item_frame(&mut item, 0.0, 18.0, 40.0, 12.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::PageHeader,
        height: Mm(40.0),
        items: vec![item],
    });

    assert!(resize_band_height(&mut report.pages[0], 0, -100.0));
    assert_eq!(report.pages[0].bands[0].height, Mm(30.0));
}

#[test]
fn fit_band_to_contents_removes_top_and_bottom_space() {
    let mut report = blank_report();
    let mut first = new_text_item("DejaVu Sans".to_string());
    let mut second = new_text_item("DejaVu Sans".to_string());
    set_item_frame(&mut first, 0.0, 4.0, 40.0, 8.0);
    set_item_frame(&mut second, 0.0, 18.0, 40.0, 12.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::PageHeader,
        height: Mm(55.0),
        items: vec![first, second],
    });

    assert!(fit_band_to_contents(&mut report.pages[0], 0));
    assert_eq!(report.pages[0].bands[0].height, Mm(26.0));
    assert_eq!(
        normalized_geometry(&report.pages[0].bands[0].items[0]).1,
        0.0
    );
    assert_eq!(
        normalized_geometry(&report.pages[0].bands[0].items[1]).1,
        14.0
    );
}

#[test]
fn fit_empty_band_uses_minimum_height() {
    let mut report = blank_report();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportFooter,
        height: Mm(30.0),
        items: Vec::new(),
    });

    assert!(fit_band_to_contents(&mut report.pages[0], 0));
    assert_eq!(report.pages[0].bands[0].height, Mm(5.0));
    assert!(!fit_band_to_contents(&mut report.pages[0], 0));
}

#[test]
fn move_band_swaps_neighbouring_bands() {
    let mut report = blank_report();
    report.pages[0].bands = vec![
        Band {
            kind: BandKind::ReportHeader,
            height: Mm(10.0),
            items: Vec::new(),
        },
        Band {
            kind: BandKind::Data {
                source: "patients".to_string(),
            },
            height: Mm(10.0),
            items: Vec::new(),
        },
    ];

    assert!(move_band(&mut report.pages[0], 1, 0));
    assert!(matches!(
        report.pages[0].bands[0].kind,
        BandKind::Data { .. }
    ));
    assert!(!move_band(&mut report.pages[0], 0, 2));
}

#[test]
fn passive_messages_do_not_close_context_menu() {
    assert!(!update::message_closes_context_menu(&Message::FontLoaded));
    assert!(!update::message_closes_context_menu(
        &Message::ModifiersChanged(keyboard::Modifiers::SHIFT)
    ));
}

#[test]
fn context_menu_action_closes_context_menu() {
    assert!(update::message_closes_context_menu(&Message::Copy));
    assert!(update::message_closes_context_menu(
        &Message::FitActiveBandToContents
    ));
}

#[test]
fn passive_messages_do_not_close_app_menu() {
    assert!(!update::message_closes_app_menu(&Message::FontLoaded));
    assert!(!update::message_closes_app_menu(
        &Message::ModifiersChanged(keyboard::Modifiers::SHIFT)
    ));
}

#[test]
fn app_menu_action_closes_app_menu() {
    assert!(update::message_closes_app_menu(&Message::Load));
    assert!(update::message_closes_app_menu(&Message::Undo));
    assert!(update::message_closes_app_menu(&Message::OpenAbout));
}

#[test]
fn band_inputs_sync_data_source_and_height() {
    let band = Band {
        kind: BandKind::Data {
            source: "orders".to_string(),
        },
        height: Mm(27.5),
        items: Vec::new(),
    };
    let mut inputs = BandInputs::default();

    inputs.sync(&band);

    assert_eq!(inputs.height, "27.50");
    assert_eq!(inputs.data_source, "orders");
}

#[test]
fn history_selection_is_invalid_after_selected_item_disappears() {
    let mut report = blank_report();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![new_text_item("DejaVu Sans".to_string())],
    });
    let selection = Selection::top_level(0, 0);
    assert!(report_contains_selection(&report, selection));

    report.pages[0].bands[0].items.clear();

    assert!(!report_contains_selection(&report, selection));
}

#[test]
fn inserted_items_receive_sequential_unique_names() {
    let mut first = new_text_item("DejaVu Sans".to_string());
    assign_unique_item_name(&mut first, &[]);
    let mut second = new_text_item("DejaVu Sans".to_string());
    assign_unique_item_name(&mut second, &[first.clone()]);

    assert_eq!(item_name(&first), "itemText1");
    assert_eq!(item_name(&second), "itemText2");
}

#[test]
fn item_name_edit_requires_a_nonempty_unique_name() {
    let mut report = blank_report();
    let mut first = new_text_item("DejaVu Sans".to_string());
    let mut second = new_text_item("DejaVu Sans".to_string());
    *item_name_mut(&mut first) = "itemText1".to_string();
    *item_name_mut(&mut second) = "itemText2".to_string();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![first, second],
    });
    let selection = Selection::top_level(0, 0);

    assert_eq!(
        rename_report_item(&mut report, selection, "reportTitle"),
        Ok(true)
    );
    assert_eq!(
        item_name(item_at_selection(&report, selection).unwrap()),
        "reportTitle"
    );
    assert_eq!(
        rename_report_item(&mut report, selection, "itemText2"),
        Err("Another item already uses this name")
    );
    assert_eq!(
        rename_report_item(&mut report, selection, "   "),
        Err("Item name cannot be empty")
    );
}

#[test]
fn equalize_nested_layout_children_preserves_the_nested_structure() {
    let mut inner = new_layout_item(true);
    let Item::HorizontalLayout(inner_layout) = &mut inner else {
        unreachable!();
    };
    inner_layout.width = Mm(60.0);
    inner_layout.height = Mm(10.0);
    inner_layout.items = vec![
        new_text_item("DejaVu Sans".to_string()),
        new_text_item("DejaVu Sans".to_string()),
    ];
    arrange_layout_children(&mut inner_layout.items, true, 60.0, 10.0);

    let mut outer = Item::VerticalLayout(LayoutItem {
        name: "verticalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(60.0),
        height: Mm(40.0),
        items: vec![inner, new_text_item("DejaVu Sans".to_string())],
    });
    set_item_frame(
        item_layout_mut(&mut outer)
            .unwrap()
            .items
            .get_mut(0)
            .unwrap(),
        0.0,
        0.0,
        60.0,
        10.0,
    );
    set_item_frame(
        item_layout_mut(&mut outer)
            .unwrap()
            .items
            .get_mut(1)
            .unwrap(),
        0.0,
        10.0,
        60.0,
        30.0,
    );

    assert!(equalize_layout_children(&mut outer));
    let Item::VerticalLayout(outer) = outer else {
        unreachable!();
    };
    assert_eq!(normalized_geometry(&outer.items[0]).3, 20.0);
    assert_eq!(normalized_geometry(&outer.items[1]).3, 20.0);
    let Item::HorizontalLayout(inner) = &outer.items[0] else {
        unreachable!();
    };
    assert_eq!(inner.items.len(), 2);
    assert!(
        inner
            .items
            .iter()
            .all(|child| normalized_geometry(child).3 == 20.0)
    );
}

#[test]
fn layout_text_style_updates_all_nested_text_items() {
    let mut nested = Item::VerticalLayout(LayoutItem {
        name: "verticalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(60.0),
        height: Mm(30.0),
        items: vec![
            new_text_item("DejaVu Sans".to_string()),
            Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout1".to_string(),
                x: Mm(0.0),
                y: Mm(15.0),
                width: Mm(60.0),
                height: Mm(15.0),
                items: vec![new_image_item(), new_text_item("DejaVu Sans".to_string())],
            }),
        ],
    });

    let count = update_text_items(&mut nested, &mut |text| {
        text.font_family = "DejaVu Serif".to_string();
        text.bold = true;
        text.text_color = ReportColor::rgb(45, 105, 200);
        text.border = Some(Border {
            left: true,
            top: true,
            right: true,
            bottom: true,
            width: 0.5,
        });
    });

    assert_eq!(count, 2);
    let representative = first_text_item(&nested).expect("layout must contain text");
    assert_eq!(representative.font_family, "DejaVu Serif");
    assert!(representative.bold);
    assert_eq!(representative.text_color, ReportColor::rgb(45, 105, 200));
    assert!(representative.border.is_some());
    let Item::VerticalLayout(layout) = nested else {
        unreachable!();
    };
    let Item::HorizontalLayout(inner) = &layout.items[1] else {
        unreachable!();
    };
    let Item::Text(second) = &inner.items[1] else {
        unreachable!();
    };
    assert_eq!(second.font_family, "DejaVu Serif");
    assert!(second.bold);
    assert_eq!(second.text_color, ReportColor::rgb(45, 105, 200));
    assert!(second.border.is_some());
    assert!(matches!(inner.items[0], Item::Image(_)));
}

#[test]
fn inserted_item_moves_to_next_free_grid_position() {
    let mut occupied = new_text_item("DejaVu Sans".to_string());
    set_item_origin(&mut occupied, 0.0, 0.0);
    let candidate = new_text_item("DejaVu Sans".to_string());

    let position = find_free_item_position(&candidate, &[occupied], 190.0, 30.0);

    assert_eq!(position, Some((55.0, 0.0)));
}

#[test]
fn layout_items_have_real_geometry_and_distinct_types() {
    let horizontal = new_layout_item(true);
    let vertical = new_layout_item(false);

    assert!(matches!(horizontal, Item::HorizontalLayout(_)));
    assert!(matches!(vertical, Item::VerticalLayout(_)));
    assert_eq!(geometry_values(&horizontal), (0.0, 0.0, 60.0, 20.0));
}

#[test]
fn vertical_layout_distributes_two_items_equally() {
    let mut items = vec![
        new_text_item("DejaVu Sans".to_string()),
        new_text_item("DejaVu Sans".to_string()),
    ];

    arrange_layout_children(&mut items, false, 50.0, 20.0);

    assert_eq!(geometry_values(&items[0]), (0.0, 0.0, 50.0, 10.0));
    assert_eq!(geometry_values(&items[1]), (0.0, 10.0, 50.0, 10.0));
}

#[test]
fn horizontal_layout_distributes_two_items_equally() {
    let mut items = vec![
        new_text_item("DejaVu Sans".to_string()),
        new_text_item("DejaVu Sans".to_string()),
    ];

    arrange_layout_children(&mut items, true, 100.0, 10.0);

    assert_eq!(geometry_values(&items[0]), (0.0, 0.0, 50.0, 10.0));
    assert_eq!(geometry_values(&items[1]), (50.0, 0.0, 50.0, 10.0));
}

#[test]
fn horizontal_layout_divider_changes_only_adjacent_widths() {
    let mut layout = Item::HorizontalLayout(LayoutItem {
        name: "horizontalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(100.0),
        height: Mm(10.0),
        items: vec![
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
        ],
    });
    reflow_layout(&mut layout);

    assert!(resize_layout_divider(&mut layout, 0, true, 10.0));

    let Item::HorizontalLayout(layout) = layout else {
        unreachable!();
    };
    assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 60.0, 10.0));
    assert_eq!(geometry_values(&layout.items[1]), (60.0, 0.0, 40.0, 10.0));
}

#[test]
fn vertical_layout_divider_changes_only_adjacent_heights() {
    let mut layout = Item::VerticalLayout(LayoutItem {
        name: "verticalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(50.0),
        height: Mm(20.0),
        items: vec![
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
        ],
    });
    reflow_layout(&mut layout);

    assert!(resize_layout_divider(&mut layout, 0, false, 4.0));

    let Item::VerticalLayout(layout) = layout else {
        unreachable!();
    };
    assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 50.0, 14.0));
    assert_eq!(geometry_values(&layout.items[1]), (0.0, 14.0, 50.0, 6.0));
}

#[test]
fn nested_selection_resolves_layout_child() {
    let mut report = blank_report();
    let child = new_text_item("DejaVu Sans".to_string());
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(50.0),
            height: Mm(10.0),
            items: vec![child],
        })],
    });
    let selection = Selection::top_level(0, 0).push(0).unwrap();

    assert!(matches!(
        item_at_selection(&report, selection),
        Some(Item::Text(_))
    ));
    assert!(report_contains_selection(&report, selection));
}

#[test]
fn selection_reaches_text_inside_two_nested_layouts() {
    let mut report = blank_report();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![Item::VerticalLayout(LayoutItem {
            name: "verticalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(50.0),
            height: Mm(20.0),
            items: vec![Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout1".to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(50.0),
                height: Mm(10.0),
                items: vec![new_text_item("DejaVu Sans".to_string())],
            })],
        })],
    });
    let selection = Selection::top_level(0, 0)
        .push(0)
        .and_then(|selection| selection.push(0))
        .unwrap();

    assert!(matches!(
        item_at_selection(&report, selection),
        Some(Item::Text(_))
    ));
    assert!(report_contains_selection(&report, selection));
}

#[test]
fn empty_selection_path_has_no_descendant_and_does_not_panic() {
    assert_eq!(selected_descendant_path(&[], 0), None);
    assert_eq!(selected_descendant_path(&[0], 0), None);
    assert_eq!(selected_descendant_path(&[0, 2], 0), Some(&[2][..]));
    assert_eq!(selected_descendant_path(&[1, 2], 0), None);
}

#[test]
fn vertical_layout_keeps_horizontal_layouts_nested() {
    let selected = vec![new_layout_item(true), new_layout_item(true)];

    let (children, retained_name) = flatten_matching_layouts(selected, false);

    assert_eq!(children.len(), 2);
    assert!(
        children
            .iter()
            .all(|item| matches!(item, Item::HorizontalLayout(_)))
    );
    assert!(retained_name.is_none());
}

#[test]
fn nesting_layout_preserves_inner_divider_proportions() {
    let mut inner = Item::HorizontalLayout(LayoutItem {
        name: "horizontalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(100.0),
        height: Mm(10.0),
        items: vec![
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
        ],
    });
    let Item::HorizontalLayout(layout) = &mut inner else {
        unreachable!();
    };
    set_item_frame(&mut layout.items[0], 0.0, 0.0, 30.0, 10.0);
    set_item_frame(&mut layout.items[1], 30.0, 0.0, 70.0, 10.0);

    arrange_layout_children(std::slice::from_mut(&mut inner), false, 200.0, 20.0);

    let Item::HorizontalLayout(layout) = inner else {
        unreachable!();
    };
    assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 60.0, 20.0));
    assert_eq!(geometry_values(&layout.items[1]), (60.0, 0.0, 140.0, 20.0));
}

#[test]
fn layout_label_is_positioned_above_container() {
    let layout = new_layout_item(true);
    let bounds = Rectangle::new(Point::new(100.0, 80.0), Size::new(200.0, 40.0));

    let label = layout_label_bounds(&layout, bounds, false).unwrap();

    assert_eq!(label.x, bounds.x);
    assert_eq!(label.y + label.height, bounds.y - 2.0);
    assert!(!label.intersects(&bounds));
}

#[test]
fn vertical_layout_label_is_positioned_right_of_container() {
    let layout = new_layout_item(false);
    let bounds = Rectangle::new(Point::new(100.0, 80.0), Size::new(200.0, 40.0));

    let label = layout_label_bounds(&layout, bounds, false).unwrap();

    assert_eq!(label.x, bounds.x + bounds.width + 2.0);
    assert_eq!(label.y, bounds.y);
    assert!(!label.intersects(&bounds));
}

#[test]
fn vertical_layout_nested_in_horizontal_keeps_label_inside_its_own_bounds() {
    let layout = new_layout_item(false);
    let bounds = Rectangle::new(Point::new(100.0, 80.0), Size::new(200.0, 40.0));

    let label = layout_label_bounds(&layout, bounds, true).unwrap();

    assert!(bounds.contains(label.position()));
    assert!(label.x + label.width <= bounds.x + bounds.width);
}

#[test]
fn first_nested_horizontal_layout_label_is_clickable_above_parent() {
    let child = Item::HorizontalLayout(LayoutItem {
        name: "horizontalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(100.0),
        height: Mm(10.0),
        items: vec![new_text_item("DejaVu Sans".to_string())],
    });
    let parent = Item::VerticalLayout(LayoutItem {
        name: "verticalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(100.0),
        height: Mm(20.0),
        items: vec![child],
    });
    let parent_selection = Selection::top_level(0, 0);

    let hit = hit_test_item(
        &parent,
        100.0,
        100.0,
        1.0,
        Point::new(105.0, 85.0),
        parent_selection,
        false,
    );

    assert_eq!(hit, parent_selection.push(0));
}

#[test]
fn matching_layout_is_flattened_when_adding_another_item() {
    let existing = Item::HorizontalLayout(LayoutItem {
        name: "horizontalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(100.0),
        height: Mm(10.0),
        items: vec![
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
        ],
    });
    let text3 = new_text_item("DejaVu Sans".to_string());

    let (children, retained_name) = flatten_matching_layouts(vec![existing, text3], true);

    assert_eq!(children.len(), 3);
    assert_eq!(retained_name.as_deref(), Some("horizontalLayout1"));
    assert!(children.iter().all(|item| matches!(item, Item::Text(_))));
}

#[test]
fn resizing_horizontal_layout_right_edge_resizes_only_last_child() {
    let mut layout = Item::HorizontalLayout(LayoutItem {
        name: "horizontalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(100.0),
        height: Mm(10.0),
        items: vec![
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
        ],
    });
    reflow_layout(&mut layout);
    resize_item(&mut layout, ResizeHandle::Right, 20.0, 0.0, 200.0, 50.0);

    let Item::HorizontalLayout(layout) = layout else {
        unreachable!();
    };
    assert_eq!(layout.width, Mm(120.0));
    assert_eq!(layout.height, Mm(10.0));
    assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 50.0, 10.0));
    assert_eq!(geometry_values(&layout.items[1]), (50.0, 0.0, 70.0, 10.0));
}

#[test]
fn resizing_vertical_layout_right_edge_resizes_all_child_widths() {
    let mut layout = Item::VerticalLayout(LayoutItem {
        name: "verticalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(50.0),
        height: Mm(30.0),
        items: vec![
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
        ],
    });
    reflow_layout(&mut layout);

    resize_item(&mut layout, ResizeHandle::Right, 20.0, 0.0, 200.0, 100.0);

    let Item::VerticalLayout(layout) = layout else {
        unreachable!();
    };
    assert_eq!(layout.width, Mm(70.0));
    assert!(
        layout
            .items
            .iter()
            .all(|item| geometry_values(item).2 == 70.0)
    );
}

#[test]
fn resizing_vertical_layout_bottom_expands_last_nested_layout_contents() {
    let horizontal_layout = |name: &str| {
        let mut item = Item::HorizontalLayout(LayoutItem {
            name: name.to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(100.0),
            height: Mm(10.0),
            items: vec![
                new_text_item("DejaVu Sans".to_string()),
                new_text_item("DejaVu Sans".to_string()),
            ],
        });
        reflow_layout(&mut item);
        item
    };
    let mut layout = Item::VerticalLayout(LayoutItem {
        name: "verticalLayout1".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(100.0),
        height: Mm(20.0),
        items: vec![
            horizontal_layout("horizontalLayout1"),
            horizontal_layout("horizontalLayout2"),
        ],
    });
    reflow_layout(&mut layout);

    resize_item(&mut layout, ResizeHandle::Bottom, 0.0, 20.0, 150.0, 100.0);

    let Item::VerticalLayout(layout) = layout else {
        unreachable!();
    };
    assert_eq!(layout.height, Mm(40.0));
    let Item::HorizontalLayout(last) = &layout.items[1] else {
        unreachable!();
    };
    assert_eq!(geometry_values(&layout.items[1]), (0.0, 10.0, 100.0, 30.0));
    assert!(
        last.items
            .iter()
            .all(|item| geometry_values(item).3 == 30.0)
    );
}

#[test]
fn generated_text_matches_generated_item_name() {
    let mut item = new_text_item("DejaVu Sans".to_string());
    assign_unique_item_name(&mut item, &[]);

    apply_generated_text(&mut item);

    let Item::Text(text) = item else {
        unreachable!();
    };
    assert_eq!(text.name, "itemText1");
    assert_eq!(text.text, "text1");
}

#[test]
fn pasted_text_matches_new_item_name() {
    let mut item = new_text_item("DejaVu Sans".to_string());
    let Item::Text(text) = &mut item else {
        unreachable!();
    };
    text.name = "itemText7".to_string();
    text.text = "copied text".to_string();

    apply_pasted_text(&mut item);

    let Item::Text(text) = item else {
        unreachable!();
    };
    assert_eq!(text.name, "itemText7");
    assert_eq!(text.text, "text7");
}

#[test]
fn pasted_layout_updates_nested_texts_from_their_new_names() {
    let mut text8 = new_text_item("DejaVu Sans".to_string());
    let Item::Text(text) = &mut text8 else {
        unreachable!();
    };
    text.name = "itemText8".to_string();
    text.text = "copied first".to_string();

    let mut text9 = new_text_item("DejaVu Sans".to_string());
    let Item::Text(text) = &mut text9 else {
        unreachable!();
    };
    text.name = "itemText9".to_string();
    text.text = "copied second".to_string();

    let mut layout = Item::VerticalLayout(LayoutItem {
        name: "verticalLayout2".to_string(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(100.0),
        height: Mm(20.0),
        items: vec![
            text8,
            Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout3".to_string(),
                x: Mm(0.0),
                y: Mm(10.0),
                width: Mm(100.0),
                height: Mm(10.0),
                items: vec![text9],
            }),
        ],
    });

    apply_pasted_text(&mut layout);

    let Item::VerticalLayout(layout) = layout else {
        unreachable!();
    };
    let Item::Text(first) = &layout.items[0] else {
        unreachable!();
    };
    let Item::HorizontalLayout(nested) = &layout.items[1] else {
        unreachable!();
    };
    let Item::Text(second) = &nested.items[0] else {
        unreachable!();
    };
    assert_eq!(first.text, "text8");
    assert_eq!(second.text, "text9");
}

#[test]
fn generated_text_number_is_global_across_report_bands() {
    let mut report = blank_report();
    let mut first = new_text_item("DejaVu Sans".to_string());
    assign_unique_item_name_in_report(&mut first, &report);
    apply_generated_text(&mut first);
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(20.0),
        items: vec![first],
    });
    report.pages[0].bands.push(Band {
        kind: BandKind::Data {
            source: "data".to_string(),
        },
        height: Mm(20.0),
        items: Vec::new(),
    });
    let mut second = new_text_item("DejaVu Sans".to_string());

    assign_unique_item_name_in_report(&mut second, &report);
    apply_generated_text(&mut second);

    let Item::Text(second) = second else {
        unreachable!();
    };
    assert_eq!(second.name, "itemText2");
    assert_eq!(second.text, "text2");
}

#[test]
fn structure_reorder_keeps_top_level_item_geometry() {
    let mut report = blank_report();
    let mut first = new_text_item("DejaVu Sans".to_string());
    let mut second = new_text_item("DejaVu Sans".to_string());
    *item_name_mut(&mut first) = "itemText1".to_string();
    *item_name_mut(&mut second) = "itemText2".to_string();
    set_item_frame(&mut first, 7.0, 3.0, 21.0, 8.0);
    set_item_frame(&mut second, 40.0, 12.0, 31.0, 9.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![first, second],
    });

    let selection = reorder_item_same_parent(
        &mut report,
        Selection::top_level(0, 0),
        Selection::top_level(0, 1),
    )
    .expect("items with the same parent can be reordered");
    let items = &report.pages[0].bands[0].items;

    assert_eq!(selection, Selection::top_level(0, 1));
    assert_eq!(item_name(&items[0]), "itemText2");
    assert_eq!(geometry_values(&items[0]), (40.0, 12.0, 31.0, 9.0));
    assert_eq!(item_name(&items[1]), "itemText1");
    assert_eq!(geometry_values(&items[1]), (7.0, 3.0, 21.0, 8.0));
}

#[test]
fn structure_reorder_moves_nested_item_into_existing_layout_slot() {
    let mut report = blank_report();
    let mut children = ["itemText1", "itemText2", "itemText3"].map(|name| {
        let mut item = new_text_item("DejaVu Sans".to_string());
        *item_name_mut(&mut item) = name.to_string();
        item
    });
    set_item_frame(&mut children[0], 0.0, 0.0, 10.0, 12.0);
    set_item_frame(&mut children[1], 10.0, 0.0, 20.0, 12.0);
    set_item_frame(&mut children[2], 30.0, 0.0, 30.0, 12.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(60.0),
            height: Mm(12.0),
            items: children.into(),
        })],
    });
    let parent = Selection::top_level(0, 0);
    let source = parent.push(0).unwrap();
    let target = parent.push(2).unwrap();

    let selection = reorder_item_same_parent(&mut report, source, target)
        .expect("layout children can be reordered");
    let Item::HorizontalLayout(layout) = &report.pages[0].bands[0].items[0] else {
        unreachable!();
    };

    assert_eq!(selection, target);
    assert_eq!(item_name(&layout.items[0]), "itemText2");
    assert_eq!(item_name(&layout.items[1]), "itemText3");
    assert_eq!(item_name(&layout.items[2]), "itemText1");
    assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 10.0, 12.0));
    assert_eq!(geometry_values(&layout.items[1]), (10.0, 0.0, 20.0, 12.0));
    assert_eq!(geometry_values(&layout.items[2]), (30.0, 0.0, 30.0, 12.0));
}

#[test]
fn structure_reorder_rejects_different_parents() {
    let mut report = blank_report();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![
            Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout1".to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(60.0),
                height: Mm(12.0),
                items: vec![new_text_item("DejaVu Sans".to_string())],
            }),
            new_text_item("DejaVu Sans".to_string()),
        ],
    });
    let source = Selection::top_level(0, 0).push(0).unwrap();
    let target = Selection::top_level(0, 1);

    assert_eq!(reorder_item_same_parent(&mut report, source, target), None);
    let items = &report.pages[0].bands[0].items;
    assert_eq!(items.len(), 2);
    assert_eq!(item_name(&items[0]), "horizontalLayout1");
    assert!(matches!(items[1], Item::Text(_)));
}

#[test]
fn structure_drag_moves_nested_item_to_another_band() {
    let mut report = blank_report();
    let mut child = new_text_item("DejaVu Sans".to_string());
    *item_name_mut(&mut child) = "itemText1".to_string();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(60.0),
            height: Mm(12.0),
            items: vec![child],
        })],
    });
    report.pages[0].bands.push(Band {
        kind: BandKind::Data {
            source: "data".to_string(),
        },
        height: Mm(30.0),
        items: Vec::new(),
    });
    let source = Selection::top_level(0, 0).push(0).unwrap();

    let selection = move_item_to_band(&mut report, source, 1)
        .expect("an item can be moved from a layout to another band");

    assert_eq!(selection, Selection::top_level(1, 0));
    let Item::HorizontalLayout(layout) = &report.pages[0].bands[0].items[0] else {
        unreachable!();
    };
    assert!(layout.items.is_empty());
    assert_eq!(item_name(&report.pages[0].bands[1].items[0]), "itemText1");
}

#[test]
fn dismantling_layout_preserves_children_visual_positions() {
    let mut report = blank_report();
    let mut first = new_text_item("DejaVu Sans".to_string());
    let mut second = new_text_item("DejaVu Sans".to_string());
    *item_name_mut(&mut first) = "itemText1".to_string();
    *item_name_mut(&mut second) = "itemText2".to_string();
    set_item_frame(&mut first, 0.0, 0.0, 20.0, 10.0);
    set_item_frame(&mut second, 20.0, 0.0, 30.0, 10.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(12.0),
            y: Mm(7.0),
            width: Mm(50.0),
            height: Mm(10.0),
            items: vec![first, second],
        })],
    });

    let selections = dismantle_layout(&mut report, Selection::top_level(0, 0))
        .expect("selected layout can be dismantled");
    let items = &report.pages[0].bands[0].items;

    assert_eq!(selections.len(), 2);
    assert_eq!(selections[0], Selection::top_level(0, 0));
    assert_eq!(selections[1], Selection::top_level(0, 1));
    assert_eq!(item_name(&items[0]), "itemText1");
    assert_eq!(geometry_values(&items[0]), (12.0, 7.0, 20.0, 10.0));
    assert_eq!(item_name(&items[1]), "itemText2");
    assert_eq!(geometry_values(&items[1]), (32.0, 7.0, 30.0, 10.0));
}

#[test]
fn structure_drag_moves_item_into_layout_and_equalizes_slots() {
    let mut report = blank_report();
    let mut outside = new_text_item("DejaVu Sans".to_string());
    let mut inside = new_text_item("DejaVu Sans".to_string());
    *item_name_mut(&mut outside) = "itemText1".to_string();
    *item_name_mut(&mut inside) = "itemText2".to_string();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![
            outside,
            Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout1".to_string(),
                x: Mm(10.0),
                y: Mm(5.0),
                width: Mm(60.0),
                height: Mm(12.0),
                items: vec![inside],
            }),
        ],
    });

    let selection = move_item_into_layout(
        &mut report,
        Selection::top_level(0, 0),
        Selection::top_level(0, 1),
    )
    .expect("top-level item can be moved into a layout");
    let Item::HorizontalLayout(layout) = &report.pages[0].bands[0].items[0] else {
        unreachable!();
    };

    assert_eq!(selection, Selection::top_level(0, 0).push(1).unwrap());
    assert_eq!(item_name(&layout.items[0]), "itemText2");
    assert_eq!(item_name(&layout.items[1]), "itemText1");
    assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 30.0, 12.0));
    assert_eq!(geometry_values(&layout.items[1]), (30.0, 0.0, 30.0, 12.0));
}

#[test]
fn structure_drag_moves_item_between_layouts() {
    let mut report = blank_report();
    let mut first = new_text_item("DejaVu Sans".to_string());
    let mut second = new_text_item("DejaVu Sans".to_string());
    *item_name_mut(&mut first) = "itemText1".to_string();
    *item_name_mut(&mut second) = "itemText2".to_string();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![
            Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout1".to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(40.0),
                height: Mm(10.0),
                items: vec![first],
            }),
            Item::VerticalLayout(LayoutItem {
                name: "verticalLayout1".to_string(),
                x: Mm(50.0),
                y: Mm(0.0),
                width: Mm(30.0),
                height: Mm(20.0),
                items: vec![second],
            }),
        ],
    });
    let source = Selection::top_level(0, 0).push(0).unwrap();
    let target = Selection::top_level(0, 1);

    let selection = move_item_into_layout(&mut report, source, target)
        .expect("item can be moved between layouts");
    let Item::HorizontalLayout(source_layout) = &report.pages[0].bands[0].items[0] else {
        unreachable!();
    };
    let Item::VerticalLayout(target_layout) = &report.pages[0].bands[0].items[1] else {
        unreachable!();
    };

    assert!(source_layout.items.is_empty());
    assert_eq!(selection, target.push(1).unwrap());
    assert_eq!(item_name(&target_layout.items[0]), "itemText2");
    assert_eq!(item_name(&target_layout.items[1]), "itemText1");
    assert_eq!(
        geometry_values(&target_layout.items[0]),
        (0.0, 0.0, 30.0, 10.0)
    );
    assert_eq!(
        geometry_values(&target_layout.items[1]),
        (0.0, 10.0, 30.0, 10.0)
    );
}

#[test]
fn structure_drag_rejects_moving_layout_into_its_descendant() {
    let mut report = blank_report();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![Item::VerticalLayout(LayoutItem {
            name: "verticalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(60.0),
            height: Mm(20.0),
            items: vec![Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout1".to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(60.0),
                height: Mm(20.0),
                items: Vec::new(),
            })],
        })],
    });
    let source = Selection::top_level(0, 0);
    let target = source.push(0).unwrap();

    assert_eq!(move_item_into_layout(&mut report, source, target), None);
    assert_eq!(
        item_name(&report.pages[0].bands[0].items[0]),
        "verticalLayout1"
    );
}

#[test]
fn structure_multi_drag_reorders_items_as_one_group() {
    let mut report = blank_report();
    let items = ["itemText1", "itemText2", "itemText3", "itemText4"].map(|name| {
        let mut item = new_text_item("DejaVu Sans".to_string());
        *item_name_mut(&mut item) = name.to_string();
        item
    });
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: items.into(),
    });
    let sources = [Selection::top_level(0, 0), Selection::top_level(0, 2)];

    let selections = reorder_items_same_parent(&mut report, &sources, Selection::top_level(0, 3))
        .expect("multiple siblings can be reordered together");
    let names = report.pages[0].bands[0]
        .items
        .iter()
        .map(item_name)
        .collect::<Vec<_>>();

    assert_eq!(names, ["itemText2", "itemText1", "itemText3", "itemText4"]);
    assert_eq!(
        selections,
        [Selection::top_level(0, 1), Selection::top_level(0, 2)]
    );
}

#[test]
fn structure_multi_drag_moves_items_into_layout() {
    let mut report = blank_report();
    let mut first = new_text_item("DejaVu Sans".to_string());
    let mut second = new_text_item("DejaVu Sans".to_string());
    *item_name_mut(&mut first) = "itemText1".to_string();
    *item_name_mut(&mut second) = "itemText2".to_string();
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(30.0),
        items: vec![
            first,
            second,
            Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout1".to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(60.0),
                height: Mm(10.0),
                items: Vec::new(),
            }),
        ],
    });
    let sources = [Selection::top_level(0, 0), Selection::top_level(0, 1)];

    let selections = move_items_into_layout(&mut report, &sources, Selection::top_level(0, 2))
        .expect("multiple items can be moved into a layout");
    let Item::HorizontalLayout(layout) = &report.pages[0].bands[0].items[0] else {
        unreachable!();
    };

    assert_eq!(selections.len(), 2);
    assert_eq!(item_name(&layout.items[0]), "itemText1");
    assert_eq!(item_name(&layout.items[1]), "itemText2");
    assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 30.0, 10.0));
    assert_eq!(geometry_values(&layout.items[1]), (30.0, 0.0, 30.0, 10.0));
}

#[test]
fn moving_item_to_band_grows_target_band_when_needed() {
    let mut report = blank_report();
    let mut item = new_text_item("DejaVu Sans".to_string());
    set_item_frame(&mut item, 5.0, 28.0, 40.0, 12.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportHeader,
        height: Mm(50.0),
        items: vec![item],
    });
    report.pages[0].bands.push(Band {
        kind: BandKind::Data {
            source: "data".to_string(),
        },
        height: Mm(20.0),
        items: Vec::new(),
    });

    move_item_to_band(&mut report, Selection::top_level(0, 0), 1)
        .expect("item can be moved to target band");

    assert_eq!(report.pages[0].bands[1].height, Mm(40.0));
}

#[test]
fn fitting_items_never_shrinks_band() {
    let mut report = blank_report();
    let mut item = new_text_item("DejaVu Sans".to_string());
    set_item_frame(&mut item, 0.0, 2.0, 20.0, 8.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::Data {
            source: "data".to_string(),
        },
        height: Mm(30.0),
        items: vec![item],
    });

    assert!(!grow_band_to_fit_items(&mut report, 0));
    assert_eq!(report.pages[0].bands[0].height, Mm(30.0));
}

#[test]
fn canvas_move_can_expand_item_beyond_current_band_height() {
    let mut report = blank_report();
    let mut item = new_text_item("DejaVu Sans".to_string());
    set_item_frame(&mut item, 0.0, 10.0, 20.0, 8.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::Data {
            source: "data".to_string(),
        },
        height: Mm(20.0),
        items: vec![item],
    });

    move_item(
        &mut report.pages[0].bands[0].items[0],
        0.0,
        7.0,
        100.0,
        27.0,
    );
    assert!(grow_band_to_fit_items(&mut report, 0));

    assert_eq!(geometry_values(&report.pages[0].bands[0].items[0]).1, 17.0);
    assert_eq!(report.pages[0].bands[0].height, Mm(25.0));
}

#[test]
fn canvas_resize_can_grow_band_at_bottom_edge() {
    let mut report = blank_report();
    let mut item = new_text_item("DejaVu Sans".to_string());
    set_item_frame(&mut item, 0.0, 8.0, 20.0, 10.0);
    report.pages[0].bands.push(Band {
        kind: BandKind::ReportFooter,
        height: Mm(20.0),
        items: vec![item],
    });

    resize_item(
        &mut report.pages[0].bands[0].items[0],
        ResizeHandle::Bottom,
        0.0,
        6.0,
        100.0,
        26.0,
    );
    assert!(grow_band_to_fit_items(&mut report, 0));

    assert_eq!(geometry_values(&report.pages[0].bands[0].items[0]).3, 16.0);
    assert_eq!(report.pages[0].bands[0].height, Mm(24.0));
}
