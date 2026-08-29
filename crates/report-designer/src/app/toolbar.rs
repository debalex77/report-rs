use super::*;

impl DesignerApp {
    pub(super) fn toolbar(&self) -> Element<'_, Message> {
        row![
            button(text("Load").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press(Message::Load),
            button(text("Save").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press(Message::Save),
            button(text("Reload").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press_maybe(self.path.is_some().then_some(Message::Reload)),
            toolbar_separator(),
            button(text("Undo").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press_maybe((!self.undo_stack.is_empty()).then_some(Message::Undo)),
            button(text("Redo").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press_maybe((!self.redo_stack.is_empty()).then_some(Message::Redo)),
            toolbar_separator(),
            alignment_button(
                include_bytes!("../../../../assets/edit-select-all-symbolic.svg"),
                self.guides_visible
            )
            .on_press(Message::ToggleGuides),
            alignment_button(
                include_bytes!("../../../../assets/preferences-system-symbolic.svg"),
                true
            )
            .on_press(Message::OpenSettings),
            button(container(text("−").size(14)).center(Fill))
                .width(30)
                .height(30)
                .padding(0)
                .style(common::style_button(8.0))
                .on_press(Message::ZoomOut),
            button(text(format!("{}%", (self.zoom * 100.0).round() as u32)))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press(Message::ZoomReset),
            button(container(text("+").size(14)).center(Fill))
                .width(30)
                .height(30)
                .padding(0)
                .style(common::style_button(6.0))
                .on_press(Message::ZoomIn),
            text(
                self.path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Untitled report".to_string())
            )
            .size(13)
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
