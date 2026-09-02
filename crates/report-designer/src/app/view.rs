use super::*;

impl DesignerApp {
    pub(super) fn view(&self) -> Element<'_, Message> {
        let Some(page) = self.report.pages.first() else {
            return container(text("The report does not contain any pages"))
                .center(Fill)
                .into();
        };
        let scale = BASE_SCALE * self.zoom;
        let canvas = Canvas::new(DesignerCanvas {
            page,
            selection: self.selection,
            selected_items: &self.selected_items,
            active_band: self.active_band,
            scale,
            font_names: &self.font_names,
            guides_visible: self.guides_visible,
            images: &self.images,
            data_field_drag: self.data_field_drag.is_some(),
        })
        .width(page.width().0 * scale + PAGE_MARGIN * 2.0)
        .height(page.height().0 * scale + PAGE_MARGIN * 2.0);
        let workspace = container(
            scrollable(container(canvas).center_x(Fill))
                .width(Fill)
                .height(Fill),
        )
        .width(Fill)
        .height(Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(196, 200, 205))),
            ..Default::default()
        });
        let menu_bar = row![
            menu_button("File", AppMenu::File, self.open_menu),
            menu_button("Edit", AppMenu::Edit, self.open_menu),
            menu_button("Info", AppMenu::Info, self.open_menu),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center);
        let work_area: Element<'_, Message> = if self.properties_visible {
            row![
                if self.toolbox_visible {
                    self.toolbox()
                } else {
                    Space::new().into()
                },
                workspace,
                Canvas::new(PropertiesResizer)
                    .width(RESIZER_WIDTH)
                    .height(Fill),
                self.inspector()
            ]
            .height(Fill)
            .into()
        } else {
            row![
                if self.toolbox_visible {
                    self.toolbox()
                } else {
                    Space::new().into()
                },
                workspace
            ]
            .height(Fill)
            .into()
        };
        let status = if self.dirty {
            format!("● {}", self.status)
        } else {
            self.status.clone()
        };
        let preview_progress: Element<'_, Message> = if self.preview_loading {
            container(
                row![
                    text("Preparing preview").size(11),
                    progress_bar(0.0..=100.0, 65.0).length(180).girth(6),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .into()
        } else {
            Space::new().width(0).into()
        };
        let status_bar = row![
            status_icon_button(
                include_bytes!("../../../../assets/toolbox-symbolic.svg"),
                self.toolbox_visible
            )
            .on_press(Message::ToggleToolbox),
            text(status).size(12),
            preview_progress,
            Space::new().width(Fill),
            report_status_frame(self.dirty || self.path.is_none()),
            status_icon_button(
                include_bytes!("../../../../assets/sidebar-show-right-symbolic.svg"),
                self.properties_visible
            )
            .on_press(Message::ToggleProperties),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        let mut layout = iced::widget::column![container(menu_bar).padding([3, 8]).width(Fill)]
            .push(container(self.toolbar()).padding(10).width(Fill))
            .push(work_area);
        if let Some(error) = &self.error_message {
            let error_bar = row![
                text(format!("⚠  {error}")).size(12),
                Space::new().width(Fill),
                button(container(text("×").size(14)).center(Fill))
                    .width(24)
                    .height(22)
                    .padding(0)
                    .style(common::style_button(4.0))
                    .on_press(Message::DismissError),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);
            layout = layout.push(
                container(error_bar)
                    .padding([4, 8])
                    .width(Fill)
                    .style(container::danger),
            );
        }
        let base: Element<'_, Message> = layout
            .push(container(status_bar).padding([2, 8]).width(Fill))
            .into();
        let base = if let Some(menu) = self.open_menu {
            stack![base, self.menu_popup(menu)].into()
        } else if let Some(position) = self.context_menu_position {
            stack![base, self.context_menu_popup(position)].into()
        } else {
            base
        };
        let content: Element<'_, Message> = if self.function_picker_visible {
            let modal = opaque(
                container(self.function_picker_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else if self.query_rules_editor.is_some() {
            let modal = opaque(
                container(self.query_rules_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else if self.pending_data_field_drop.is_some() {
            let modal = opaque(
                container(self.data_field_drop_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else if self.query_field_picker.is_some() {
            let modal = opaque(
                container(self.query_field_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else if self.data_query_editor.is_some() {
            let modal = opaque(
                container(self.data_query_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else if self.data_source_editor.is_some() {
            let modal = opaque(
                container(self.data_source_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else if self.pending_layout_move.is_some() {
            let modal = opaque(
                container(self.layout_move_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else if self.settings.is_some() {
            let modal = opaque(
                container(self.settings_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.58))),
                        ..Default::default()
                    }),
            );
            stack![base, modal].into()
        } else if self.about_visible {
            let modal = opaque(
                container(self.about_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else {
            base
        };
        if let Some(position) = self.query_text_menu_position {
            stack![content, data_sources::query_text_context_popup(position)].into()
        } else if let (Some(query), Some(position)) = (
            self.open_data_templates.as_deref(),
            self.data_templates_position,
        ) {
            stack![
                content,
                inspector::data::templates_popup(self, query, position)
            ]
            .into()
        } else {
            content
        }
    }
}
