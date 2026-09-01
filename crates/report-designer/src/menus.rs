use super::*;
use iced::widget::column;

impl DesignerApp {
    pub(super) fn layout_move_dialog(&self) -> Element<'_, Message> {
        dialog_container(
            column![
                text("Move item from layout").size(20),
                text("The selected item belongs to a layout. Choose how the operation should be performed.")
                    .size(13),
                button(text("Move entire layout").size(13))
                    .width(Fill)
                    .style(button::primary)
                    .on_press(Message::MoveEntireLayout),
                button(text("Dismantle layout and move item").size(13))
                    .width(Fill)
                    .style(common::style_button(5.0))
                    .on_press(Message::DismantleLayoutAndMoveItem),
                button(text("Cancel").size(13))
                    .width(Fill)
                    .style(common::style_button(5.0))
                    .on_press(Message::CancelLayoutMove),
            ]
            .spacing(10)
            .padding(18),
            460.0,
        )
    }

    pub(super) fn settings_dialog(&self) -> Element<'_, Message> {
        let Some(settings) = &self.settings else {
            return Space::new().into();
        };
        let orientation = row![
            button(text("Portrait").size(13))
                .style(if matches!(settings.orientation, Orientation::Portrait) {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SettingsOrientationChanged(Orientation::Portrait)),
            button(text("Landscape").size(13))
                .style(if matches!(settings.orientation, Orientation::Landscape) {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SettingsOrientationChanged(Orientation::Landscape)),
        ]
        .spacing(5);

        let mut content = column![
            row![
                text("Designer settings").size(20),
                Space::new().width(Fill),
                button(container(text("×").size(14)).center(Fill))
                    .width(28)
                    .height(28)
                    .padding(0)
                    .style(common::style_button(4.0))
                    .on_press(Message::CloseSettings),
            ]
            .align_y(iced::Alignment::Center),
            text("Orientation").size(13),
            orientation,
            text("Margins (mm)").size(13),
        ]
        .spacing(8);

        for (label, field) in [
            ("Left", MarginField::Left),
            ("Top", MarginField::Top),
            ("Right", MarginField::Right),
            ("Bottom", MarginField::Bottom),
        ] {
            content = content.push(
                row![
                    text(label).size(12).width(52),
                    spin_button("−", Message::SettingsMarginStep(field, -0.10)),
                    text_input("mm", settings.margin(field))
                        .width(90)
                        .size(13)
                        .padding(6)
                        .on_input(move |value| Message::SettingsMarginChanged(field, value)),
                    spin_button("+", Message::SettingsMarginStep(field, 0.10)),
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center),
            );
        }

        content = content
            .push(text("Default font family").size(13))
            .push(
                combo_box(
                    &self.font_families,
                    "Font family",
                    Some(&settings.font_family),
                    Message::SettingsFontChanged,
                )
                .size(13)
                .padding(6)
                .on_input(Message::SettingsFontChanged),
            )
            .push(
                row![
                    button(text("Cancel").size(13))
                        .style(common::style_button(5.0))
                        .on_press(Message::CloseSettings),
                    button(text("Apply").size(13))
                        .style(button::primary)
                        .on_press(Message::ApplySettings),
                ]
                .spacing(6),
            );

        container(scrollable(content.padding(18)).height(460))
            .width(430)
            .style(|theme: &Theme| {
                let mut dialog_background = theme.palette().background;
                dialog_background.a = 1.0;
                container::Style {
                    background: Some(Background::Color(dialog_background)),
                    border: iced::Border {
                        color: theme.extended_palette().background.strong.text,
                        width: 1.0,
                        radius: iced::border::radius(14),
                    },
                    text_color: Some(theme.palette().text),
                    shadow: iced::Shadow {
                        color: Color::from_rgba8(0, 0, 0, 0.38),
                        offset: iced::Vector::new(0.0, 8.0),
                        blur_radius: 24.0,
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    pub(super) fn about_dialog(&self) -> Element<'_, Message> {
        dialog_container(
            column![
                row![
                    text(concat!(
                        "Designer (report-rs v",
                        env!("CARGO_PKG_VERSION"),
                        ")"
                    ))
                    .size(20),
                    Space::new().width(Fill),
                    button(container(text("×").size(14)).center(Fill))
                        .width(28)
                        .height(28)
                        .padding(0)
                        .style(common::style_button(4.0))
                        .on_press(Message::CloseAbout),
                ]
                .align_y(iced::Alignment::Center),
                text("Visual designer for creating and editing JSON report templates.").size(13),
                button(text("Close").size(13))
                    .style(button::primary)
                    .on_press(Message::CloseAbout),
            ]
            .spacing(12)
            .padding(18),
            430.0,
        )
    }

    pub(super) fn menu_popup(&self, menu: AppMenu) -> Element<'_, Message> {
        let actions: Element<'_, Message> = match menu {
            AppMenu::File => {
                let mut actions = column![
                    popup_menu_action("New report", Some(Message::NewReport)),
                    popup_menu_action("Load", Some(Message::Load)),
                    popup_menu_action("Save             Ctrl+S", Some(Message::Save)),
                    popup_menu_action("Save as…  Ctrl+Shift+S", Some(Message::SaveAs)),
                    popup_menu_action("Reload", self.path.is_some().then_some(Message::Reload)),
                    popup_menu_separator(),
                    popup_menu_action(
                        if self.recent_reports_expanded {
                            "Recent reports  ▾"
                        } else {
                            "Recent reports  ▸"
                        },
                        (!self.recent_reports.is_empty()).then_some(Message::ToggleRecentReports),
                    ),
                ];
                if self.recent_reports_expanded {
                    for path in &self.recent_reports {
                        actions = actions.push(popup_menu_action_owned(
                            format!("    {}", truncate(&path.display().to_string(), 34)),
                            Some(Message::OpenRecentReport(path.clone())),
                        ));
                    }
                }
                actions.spacing(2).width(230).into()
            }
            AppMenu::Edit => column![
                popup_menu_action(
                    "Undo        Ctrl+Z",
                    (!self.undo_stack.is_empty()).then_some(Message::Undo),
                ),
                popup_menu_action(
                    "Redo        Ctrl+Y",
                    (!self.redo_stack.is_empty()).then_some(Message::Redo),
                ),
                popup_menu_separator(),
                popup_menu_action(
                    "Copy        Ctrl+C",
                    self.selection.is_some().then_some(Message::Copy),
                ),
                popup_menu_action(
                    "Paste       Ctrl+V",
                    self.clipboard_item.is_some().then_some(Message::Paste)
                ),
                popup_menu_action(
                    "Cut           Ctrl+X",
                    self.selection.is_some().then_some(Message::Cut),
                ),
                popup_menu_action(
                    "Delete",
                    self.selection.is_some().then_some(Message::Delete)
                ),
                popup_menu_action(
                    "Select all  Ctrl+A",
                    self.active_band.is_some().then_some(Message::SelectAll),
                ),
                popup_menu_separator(),
                popup_menu_action("Designer settings", Some(Message::OpenSettings)),
            ]
            .spacing(2)
            .width(210)
            .into(),
            AppMenu::Info => column![popup_menu_action("About", Some(Message::OpenAbout))]
                .spacing(2)
                .width(180)
                .into(),
        };

        let left = match menu {
            AppMenu::File => 8.0,
            AppMenu::Edit => 57.0,
            AppMenu::Info => 107.0,
        };
        let popup: Element<'_, Message> = container(opaque(
            container(actions)
                .padding(5)
                .width(if menu == AppMenu::File { 240 } else { 220 })
                .style(popup_menu_style),
        ))
        .padding(iced::Padding {
            top: 34.0,
            right: 0.0,
            bottom: 0.0,
            left,
        })
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Top)
        .width(Fill)
        .height(Fill)
        .into();

        stack![
            mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::CloseMenu),
            popup
        ]
        .into()
    }

    pub(super) fn context_menu_popup(&self, position: Point) -> Element<'_, Message> {
        let can_dismantle = self
            .selection
            .and_then(|selection| item_at_selection(&self.report, selection))
            .is_some_and(|item| item_layout(item).is_some());
        let can_fit_band = self.selection.is_none() && self.active_band.is_some();
        let actions = column![
            popup_menu_action(
                "Copy        Ctrl+C",
                self.selection.is_some().then_some(Message::Copy)
            ),
            popup_menu_action(
                "Paste       Ctrl+V",
                self.clipboard_item.is_some().then_some(Message::Paste)
            ),
            popup_menu_action(
                "Cut           Ctrl+X",
                self.selection.is_some().then_some(Message::Cut)
            ),
            popup_menu_action(
                "Delete",
                self.selection.is_some().then_some(Message::Delete)
            ),
            popup_menu_action(
                "Dismantle layout",
                can_dismantle.then_some(Message::DismantleSelectedLayout)
            ),
            popup_menu_action(
                "Fit band to contents",
                can_fit_band.then_some(Message::FitActiveBandToContents)
            ),
            popup_menu_action(
                "Select all  Ctrl+A",
                self.active_band.is_some().then_some(Message::SelectAll)
            ),
        ]
        .spacing(2)
        .width(210);
        let popup: Element<'_, Message> = container(opaque(
            container(actions)
                .padding(5)
                .width(220)
                .style(popup_menu_style),
        ))
        .padding(iced::Padding {
            top: position.y.max(0.0),
            right: 0.0,
            bottom: 0.0,
            left: position.x.max(0.0),
        })
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Top)
        .width(Fill)
        .height(Fill)
        .into();

        stack![
            mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::CloseContextMenu),
            popup
        ]
        .into()
    }

    pub(super) fn toolbox(&self) -> Element<'_, Message> {
        let content = column![
            text("Report bands").size(12),
            toolbox_button(
                include_bytes!("../../../assets/report-band-symbolic.svg"),
                "ReportHeader",
                DesignerTool::ReportHeader,
            ),
            toolbox_button(
                include_bytes!("../../../assets/report-band-symbolic.svg"),
                "DataHeader",
                DesignerTool::DataHeader,
            ),
            toolbox_button(
                include_bytes!("../../../assets/report-band-symbolic.svg"),
                "DataBand",
                DesignerTool::DataBand,
            ),
            toolbox_button(
                include_bytes!("../../../assets/report-band-symbolic.svg"),
                "ReportFooter",
                DesignerTool::ReportFooter,
            ),
            toolbox_separator(),
            text("Items").size(12),
            toolbox_button(
                include_bytes!("../../../assets/text-item-symbolic.svg"),
                "TextItem",
                DesignerTool::Text,
            ),
            toolbox_button(
                include_bytes!("../../../assets/image-item-symbolic.svg"),
                "ImageItem",
                DesignerTool::Image,
            ),
            toolbox_button(
                include_bytes!("../../../assets/shape-item-symbolic.svg"),
                "ShapeItem",
                DesignerTool::Shape,
            ),
            toolbox_separator(),
            text("Layouts").size(12),
            toolbox_button(
                include_bytes!("../../../assets/horizontal-layout-symbolic.svg"),
                "HorizontalLayout",
                DesignerTool::HorizontalLayout,
            ),
            toolbox_button(
                include_bytes!("../../../assets/vertical-layout-symbolic.svg"),
                "VerticalLayout",
                DesignerTool::VerticalLayout,
            ),
            Space::new().height(Fill),
            toolbox_separator(),
            toolbox_button(
                include_bytes!("../../../assets/delete-item-symbolic.svg"),
                "Delete item",
                DesignerTool::Delete,
            ),
        ]
        .spacing(5)
        .padding(8);

        container(content)
            .width(190)
            .height(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(theme.palette().background)),
                border: iced::Border {
                    color: theme.extended_palette().background.strong.color,
                    width: 0.0,
                    radius: iced::border::radius(0),
                },
                ..Default::default()
            })
            .into()
    }
}
