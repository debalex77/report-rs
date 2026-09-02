use super::*;

impl DesignerApp {
    pub(super) fn toolbar(&self) -> Element<'_, Message> {
        row![
            toolbar_action_button(
                include_bytes!("../../../../assets/document-new-symbolic.svg"),
                "New Report",
                Some(Message::NewReport)
            ),
            toolbar_separator(),
            toolbar_action_button(
                include_bytes!("../../../../assets/document-open-symbolic.svg"),
                "Load",
                Some(Message::Load)
            ),
            toolbar_action_button(
                include_bytes!("../../../../assets/document-save-symbolic.svg"),
                "Save",
                Some(Message::Save)
            ),
            toolbar_action_button(
                include_bytes!("../../../../assets/view-refresh-symbolic.svg"),
                "Reload",
                self.path.is_some().then_some(Message::Reload)
            ),
            toolbar_separator(),
            toolbar_action_button(
                include_bytes!("../../../../assets/preview-symbolic.svg"),
                "Preview",
                Some(Message::Preview)
            ),
            toolbar_separator(),
            toolbar_action_button(
                include_bytes!("../../../../assets/edit-undo-symbolic.svg"),
                "Undo",
                (!self.undo_stack.is_empty()).then_some(Message::Undo)
            ),
            toolbar_action_button(
                include_bytes!("../../../../assets/edit-redo-symbolic.svg"),
                "Redo",
                (!self.redo_stack.is_empty()).then_some(Message::Redo)
            ),
            toolbar_separator(),
            tooltip(
                toolbar_icon_button(
                    include_bytes!("../../../../assets/edit-select-all-symbolic.svg"),
                    Message::ToggleGuides,
                ),
                text(if self.guides_visible {
                    "Hide rulers"
                } else {
                    "Show rulers"
                })
                .size(11),
                tooltip::Position::Bottom,
            ),
            tooltip(
                toolbar_icon_button(
                    include_bytes!("../../../../assets/preferences-system-symbolic.svg"),
                    Message::OpenSettings,
                ),
                text("Report settings").size(11),
                tooltip::Position::Bottom,
            ),
            toolbar_square_button("−", Message::ZoomOut),
            toolbar_zoom_button(
                format!("{}%", (self.zoom * 100.0).round() as u32),
                Message::ZoomReset
            ),
            toolbar_square_button("+", Message::ZoomIn),
            path_frame(
                self.path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Untitled report".to_string())
            )
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
