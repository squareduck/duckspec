//! Collapsible section with toggle header.

use iced::widget::{button, column, row, text, Space};
use iced::{Element, Length};

use crate::theme;

pub fn view<'a, M: Clone + 'a>(
    title: &'a str,
    expanded: bool,
    on_toggle: M,
    content: Element<'a, M>,
) -> Element<'a, M> {
    let arrow = if expanded { "\u{25bf}" } else { "\u{25b9}" };

    let header = button(
        row![
            text(title.to_uppercase())
                .size(theme::FONT_SM)
                .color(theme::TEXT_SECONDARY),
            Space::new().width(Length::Fill),
            text(arrow).size(theme::FONT_SM).color(theme::TEXT_MUTED),
        ]
        .width(Length::Fill),
    )
    .on_press(on_toggle)
    .width(Length::Fill)
    .style(theme::section_header)
    .padding([theme::SPACING_SM, theme::SPACING_SM]);

    let mut col = column![header].spacing(0.0);

    if expanded {
        col = col.push(content);
    }

    col.into()
}
