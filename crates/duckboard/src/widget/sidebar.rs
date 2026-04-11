//! Narrow icon sidebar for area navigation.

use iced::widget::{button, column, container, text, Space};
use iced::{Center, Element, Length};

use crate::area::Area;
use crate::theme;

pub fn view<'a, M: Clone + 'a>(
    active: &Area,
    on_area: impl Fn(Area) -> M + 'a,
    on_refresh: M,
) -> Element<'a, M> {
    let mut nav = column![].spacing(theme::SPACING_XS).align_x(Center);

    for area in Area::ALL {
        let is_active = *active == area;
        let style = if is_active {
            theme::nav_button_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::nav_button
        };
        let btn = button(text(area.label()).size(16).center().width(Length::Fill))
            .width(36)
            .height(36)
            .on_press(on_area(area))
            .style(style);
        nav = nav.push(btn);
    }

    let refresh = button(text("\u{21bb}").size(14).center().width(Length::Fill))
        .width(36)
        .height(36)
        .on_press(on_refresh)
        .style(theme::nav_button);

    container(
        column![nav, Space::new().height(Length::Fill), refresh,]
            .align_x(Center)
            .spacing(theme::SPACING_XS)
            .height(Length::Fill),
    )
    .width(theme::SIDEBAR_WIDTH)
    .height(Length::Fill)
    .padding(theme::SPACING_XS)
    .style(theme::sidebar)
    .into()
}
