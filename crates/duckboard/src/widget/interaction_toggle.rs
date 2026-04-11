//! Thin vertical strip to toggle the interaction column.

use iced::widget::{button, container, text};
use iced::{Element, Length};

use crate::theme;

pub fn view<'a, M: Clone + 'a>(expanded: bool, on_toggle: M) -> Element<'a, M> {
    let arrow = if expanded { "\u{25b8}" } else { "\u{25c2}" };

    button(
        text(arrow).size(14).color(theme::TEXT_MUTED).center().width(Length::Fill),
    )
    .on_press(on_toggle)
    .width(16.0)
    .height(Length::Fill)
    .padding(0.0)
    .style(theme::interaction_toggle)
    .into()
}
