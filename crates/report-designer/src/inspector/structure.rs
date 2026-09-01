use super::*;

pub(super) fn view(app: &DesignerApp) -> Element<'_, Message> {
    let mut tree = iced::widget::column![
        text(format!("▾ Report: {}", app.report.name)).size(12),
        text("  ▾ Page 1").size(11),
    ]
    .spacing(2);
    if let Some(page) = app.report.pages.first() {
        for (band_index, band) in page.bands.iter().enumerate() {
            let selected = app.selection.is_none() && app.active_band == Some(band_index);
            let drop_target = app.structure_drag.is_some()
                && app.structure_drop_target == Some(StructureDropTarget::Band(band_index));
            let band_node = mouse_area(
                container(
                    row![
                        Space::new().width(14),
                        text("▾").size(10),
                        structure_svg(
                            include_bytes!("../../../../assets/report-band-symbolic.svg"),
                            selected,
                        ),
                        text(band_name(&band.kind)).size(11),
                    ]
                    .spacing(4),
                )
                .width(Fill)
                .padding([3, 5])
                .style(move |theme| structure_node_style(theme, selected, drop_target)),
            )
            .on_press(Message::SelectStructureBand(band_index))
            .on_enter(Message::HoverStructureBand(band_index))
            .on_release(Message::DropStructureItem);
            tree = tree.push(
                row![
                    band_node,
                    structure_band_move_button(
                        include_bytes!("../../../../assets/go-top-symbolic.svg"),
                        (band_index > 0).then_some(Message::MoveBandUp(band_index)),
                    ),
                    structure_band_move_button(
                        include_bytes!("../../../../assets/go-bottom-symbolic.svg"),
                        (band_index + 1 < page.bands.len())
                            .then_some(Message::MoveBandDown(band_index)),
                    ),
                ]
                .spacing(2)
                .align_y(iced::Alignment::Center),
            );
            for (item_index, item) in band.items.iter().enumerate() {
                let selection = Selection::top_level(band_index, item_index);
                tree = append_item(app, tree, item, selection, 2);
            }
        }
    }
    scrollable(tree).height(Fill).into()
}

fn structure_band_move_button(
    icon: &'static [u8],
    message: Option<Message>,
) -> iced::widget::Button<'static, Message> {
    button(
        svg(svg::Handle::from_memory(icon))
            .width(13)
            .height(13)
            .style(|theme: &Theme, _status| svg::Style {
                color: Some(theme.palette().text),
            }),
    )
    .width(24)
    .height(24)
    .padding(5)
    .style(button::text)
    .on_press_maybe(message)
}

fn append_item<'a>(
    app: &'a DesignerApp,
    mut tree: iced::widget::Column<'a, Message>,
    item: &'a Item,
    selection: Selection,
    depth: u16,
) -> iced::widget::Column<'a, Message> {
    let layout = item_layout(item);
    let collapsed = app.collapsed_structure_layouts.contains(&selection);
    let marker = if layout.is_some() {
        if collapsed { "▸" } else { "▾" }
    } else {
        "•"
    };
    let selected = app.selected_items.contains(&selection);
    let drop_target = app.structure_drag.is_some()
        && app.structure_drop_target == Some(StructureDropTarget::Item(selection))
        && app.structure_drag != Some(selection);
    let move_inside = drop_target
        && layout.is_some()
        && app.structure_drag.is_some_and(|source| {
            source.band != selection.band || source.parent_indices() != selection.parent_indices()
        });
    let insert_before = drop_target && !move_inside;
    let indent = Space::new().width(depth as f32 * 14.0);
    let label = format!("{}: {}", item_type_name(item), item_name(item));
    let label_widget: Element<'a, Message> = if app.structure_rename == Some(selection) {
        text_input("Item name", &app.structure_name_input)
            .id(iced::widget::Id::new("structure-item-name"))
            .size(11)
            .padding([3, 5])
            .on_input(Message::StructureNameChanged)
            .on_submit(Message::CommitStructureRename)
            .into()
    } else {
        let node = container(
            row![structure_item_svg(item, selected), text(label).size(11)]
                .spacing(5)
                .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .padding([3, 5])
        .style(move |theme| structure_node_style(theme, selected, move_inside));
        let node: Element<'a, Message> = if insert_before {
            stack![
                node,
                container(
                    container(Space::new().height(2))
                        .width(Fill)
                        .style(structure_insert_style),
                )
                .width(Fill)
                .height(Fill)
                .align_y(iced::alignment::Vertical::Top)
            ]
            .into()
        } else {
            node.into()
        };
        mouse_area(node)
            .on_press(Message::BeginStructureDrag(selection))
            .on_enter(Message::HoverStructureDrop(selection))
            .on_release(Message::DropStructureItem)
            .on_double_click(Message::BeginStructureRename(selection))
            .interaction(mouse::Interaction::Grab)
            .into()
    };
    let row = if layout.is_some() {
        row![
            indent,
            button(text(marker).size(10))
                .padding([2, 4])
                .style(button::text)
                .on_press(Message::ToggleStructureLayout(selection)),
            label_widget,
        ]
        .spacing(2)
    } else {
        row![indent, text(marker).size(10), label_widget,].spacing(4)
    };
    tree = tree.push(row.align_y(iced::Alignment::Center));
    if !collapsed && let Some(layout) = layout {
        for (index, child) in layout.items.iter().enumerate() {
            if let Some(child_selection) = selection.push(index) {
                tree = append_item(app, tree, child, child_selection, depth + 1);
            }
        }
    }
    tree
}

fn structure_item_svg(item: &Item, selected: bool) -> iced::widget::Svg<'static> {
    let icon = match item {
        Item::Text(_) | Item::Line(_) => {
            include_bytes!("../../../../assets/text-item-symbolic.svg").as_slice()
        }
        Item::Image(_) => include_bytes!("../../../../assets/image-item-symbolic.svg").as_slice(),
        Item::Rectangle(_) => {
            include_bytes!("../../../../assets/shape-item-symbolic.svg").as_slice()
        }
        Item::HorizontalLayout(_) => {
            include_bytes!("../../../../assets/horizontal-layout-symbolic.svg").as_slice()
        }
        Item::VerticalLayout(_) => {
            include_bytes!("../../../../assets/vertical-layout-symbolic.svg").as_slice()
        }
    };
    structure_svg(icon, selected)
}

fn structure_svg(icon: &'static [u8], selected: bool) -> iced::widget::Svg<'static> {
    svg(svg::Handle::from_memory(icon))
        .width(14)
        .height(14)
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(if selected {
                theme.extended_palette().primary.strong.text
            } else {
                theme.palette().text
            }),
        })
}

fn structure_insert_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().success.strong.color,
        )),
        ..Default::default()
    }
}

fn structure_node_style(theme: &Theme, selected: bool, drop_target: bool) -> container::Style {
    let palette = theme.extended_palette();
    let background = if drop_target {
        Some(Background::Color(palette.success.weak.color))
    } else if selected {
        Some(Background::Color(palette.primary.strong.color))
    } else {
        None
    };
    container::Style {
        background,
        text_color: selected.then_some(palette.primary.strong.text),
        border: iced::Border {
            color: if drop_target {
                palette.success.strong.color
            } else {
                Color::TRANSPARENT
            },
            width: if drop_target { 1.0 } else { 0.0 },
            radius: iced::border::radius(5),
        },
        ..Default::default()
    }
}
