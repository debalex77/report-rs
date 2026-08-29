use iced::{Subscription, event, keyboard};

use super::message::Message;

pub(super) fn keyboard_shortcuts() -> Subscription<Message> {
    event::listen_with(|event, status, _window| {
        if status == event::Status::Captured {
            return None;
        }
        let iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            physical_key,
            modifiers,
            repeat,
            ..
        }) = event
        else {
            return None;
        };
        if repeat {
            return None;
        }
        if key == keyboard::Key::Named(keyboard::key::Named::Delete) {
            return Some(Message::Delete);
        }
        if !modifiers.control() {
            return None;
        }
        match key.to_latin(physical_key) {
            Some('c') => Some(Message::Copy),
            Some('v') => Some(Message::Paste),
            Some('x') => Some(Message::Cut),
            Some('a') => Some(Message::SelectAll),
            Some('s') if modifiers.shift() => Some(Message::SaveAs),
            Some('s') => Some(Message::Save),
            Some('z') if modifiers.shift() => Some(Message::Redo),
            Some('z') => Some(Message::Undo),
            Some('y') => Some(Message::Redo),
            _ => None,
        }
    })
}
