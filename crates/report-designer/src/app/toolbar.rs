use super::*;

impl DesignerApp {
    pub(super) fn toolbar(&self) -> Element<'_, Message> {
        row![
            toolbar_text_button("New Report", Some(Message::NewReport)),
            toolbar_text_button("Load", Some(Message::Load)),
            toolbar_text_button("Save", Some(Message::Save)),
            toolbar_text_button("Reload", self.path.is_some().then_some(Message::Reload)),
            toolbar_separator(),
            toolbar_text_button(
                "Undo",
                (!self.undo_stack.is_empty()).then_some(Message::Undo)
            ),
            toolbar_text_button(
                "Redo",
                (!self.redo_stack.is_empty()).then_some(Message::Redo)
            ),
            toolbar_separator(),
            alignment_button(
                include_bytes!("../../../../assets/edit-select-all-symbolic.svg"),
                self.guides_visible
            )
            .on_press(Message::ToggleGuides),
            alignment_button(
                include_bytes!("../../../../assets/preferences-system-symbolic.svg"),
                false
            )
            .on_press(Message::OpenSettings),
            toolbar_square_button("−", Message::ZoomOut),
            toolbar_text_button(
                format!("{}%", (self.zoom * 100.0).round() as u32),
                Some(Message::ZoomReset)
            ),
            toolbar_square_button("+", Message::ZoomIn),
            text(
                self.path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Untitled report".to_string())
            )
            .size(13)
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
