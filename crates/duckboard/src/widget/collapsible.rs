//! Collapsible section with toggle header.

use iced::widget::{button, column, container, row, svg, text, Space};
use iced::{Element, Length};

use crate::theme;

const ICON_CHEVRON_RIGHT: &[u8] = include_bytes!("../../assets/icon_chevron_right.svg");
const ICON_CHEVRON_DOWN: &[u8] = include_bytes!("../../assets/icon_chevron_down.svg");

pub fn view<'a, M: Clone + 'a>(
    title: &'a str,
    expanded: bool,
    on_toggle: M,
    content: Element<'a, M>,
) -> Element<'a, M> {
    let header = button(
        row![
            chevron(expanded),
            text(title.to_uppercase())
                .size(theme::font_sm())
                .color(theme::text_secondary()),
        ]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Center)
        .width(Length::Fill),
    )
    .on_press(on_toggle)
    .width(Length::Fill)
    .height(32.0)
    .style(theme::section_header)
    .padding([theme::SPACING_SM, theme::SPACING_SM]);

    let mut col = column![top_divider(), header].spacing(0.0);

    if expanded {
        col = col.push(content);
    }

    col.into()
}

/// 1px horizontal hairline, used to separate stacked sections.
pub fn top_divider<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new())
        .width(Length::Fill)
        .height(1.0)
        .style(theme::divider)
        .into()
}

/// Collapse/expand chevron matching the icon set used in list rows.
pub fn chevron<'a, M: 'a>(expanded: bool) -> Element<'a, M> {
    let bytes = if expanded { ICON_CHEVRON_DOWN } else { ICON_CHEVRON_RIGHT };
    let size = theme::font_sm();
    svg(svg::Handle::from_memory(bytes))
        .width(size)
        .height(size)
        .style(theme::svg_tint(theme::text_muted()))
        .into()
}
