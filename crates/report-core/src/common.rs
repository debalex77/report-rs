use iced::widget::button;
use iced::{Theme, border};

//--------------------------------------------------------------------
//--- BUTTON
//--------------------------------------------------------------------
/// Returns a style button with a given radius
pub fn style_button(radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = button::primary(theme, status);
        style.border.radius = border::radius(radius);
        style
    }
}
