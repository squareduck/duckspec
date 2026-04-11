//! Collapsible section with toggle header.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length};

use crate::theme;

pub fn view<'a, M: Clone + 'a>(
    title: &'a str,
    expanded: bool,
    on_toggle: M,
    content: Element<'a, M>,
) -> Element<'a, M> {
    let arrow = if expanded { "\u{25bf}" } else { "\u{25b9}" };

    // Top separator line
    let separator = container(Space::new().width(Length::Fill).height(1.0))
        .width(Length::Fill)
        .style(theme::divider);

    let header = button(
        row![
            text(title)
                .size(11)
                .color(theme::TEXT_SECONDARY),
            Space::new().width(Length::Fill),
            text(arrow).size(11).color(theme::TEXT_MUTED),
        ]
        .width(Length::Fill),
    )
    .on_press(on_toggle)
    .width(Length::Fill)
    .style(theme::section_header)
    .padding([theme::SPACING_SM, theme::SPACING_SM]);

    let mut col = column![separator, header].spacing(0.0);

    if expanded {
        col = col.push(
            container(content)
                .padding([theme::SPACING_XS, 0.0]),
        );
    }

    col.into()
}
