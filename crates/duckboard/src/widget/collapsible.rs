//! Collapsible section with toggle header.

use iced::widget::{Space, button, column, container, row, svg, text};
use iced::{Element, Length};

use crate::theme;

const ICON_CHEVRON_RIGHT: &[u8] = include_bytes!("../../assets/icon_chevron_right.svg");
const ICON_CHEVRON_DOWN: &[u8] = include_bytes!("../../assets/icon_chevron_down.svg");
pub const ICON_PLUS: &[u8] = include_bytes!("../../assets/icon_plus.svg");
pub const ICON_CLOSE: &[u8] = include_bytes!("../../assets/icon_close.svg");

/// Icon-only `×` close button. Renders the same `font_sm × font_sm` SVG used
/// by `add_button`, so it visually matches a section-header `+` when both
/// sit at the right edge of a row that supplies the surrounding padding.
/// No internal padding — the caller's row owns the spacing.
pub fn close_button<'a, M: Clone + 'a>(on_press: M) -> Element<'a, M> {
    close_button_sized(on_press, theme::font_sm())
}

/// Close button at a caller-chosen pixel size. Use this when the close
/// button replaces a list-row icon on hover and the two must match widths
/// so the row's label doesn't jump.
pub fn close_button_sized<'a, M: Clone + 'a>(on_press: M, size: f32) -> Element<'a, M> {
    let icon = svg(svg::Handle::from_memory(ICON_CLOSE))
        .width(size)
        .height(size)
        .style(theme::svg_tint(theme::text_muted()));
    button(icon)
        .on_press(on_press)
        .padding(0.0)
        .style(theme::icon_button)
        .into()
}

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
    .style(theme::section_header)
    .padding([theme::SPACING_XS, theme::SPACING_SM]);

    let mut col = column![top_divider(), header].spacing(0.0);

    if expanded {
        col = col.push(top_divider());
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
    let bytes = if expanded {
        ICON_CHEVRON_DOWN
    } else {
        ICON_CHEVRON_RIGHT
    };
    let size = theme::font_sm();
    svg(svg::Handle::from_memory(bytes))
        .width(size)
        .height(size)
        .style(theme::svg_tint(theme::text_muted()))
        .into()
}

/// `+` button styled to sit flush at the right edge of a section header.
/// Explicit height is set to match the natural height of the sibling header
/// button (chevron + text widget at `font_sm` line-height plus the same
/// `[XS, SM]` padding). Without this, the icon-only button would be shorter
/// than its sibling and a `Length::Fill` would over-expand inside the
/// surrounding column.
pub fn add_button<'a, M: Clone + 'a>(on_press: M) -> Element<'a, M> {
    let icon = svg(svg::Handle::from_memory(ICON_PLUS))
        .width(theme::font_sm())
        .height(theme::font_sm())
        .style(theme::svg_tint(theme::text_secondary()));
    let natural_height = theme::font_sm() * 1.3 + 2.0 * theme::SPACING_XS;
    button(icon)
        .on_press(on_press)
        .height(natural_height)
        .padding([theme::SPACING_XS, theme::SPACING_SM])
        .style(theme::section_header)
        .into()
}
