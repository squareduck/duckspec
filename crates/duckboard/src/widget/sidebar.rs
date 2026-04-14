//! Narrow icon sidebar for area navigation.

use iced::widget::{button, column, container, svg, Space};
use iced::{Center, Element, Length};

use crate::area::Area;
use crate::theme;

const ICON_DASHBOARD: &[u8] = include_bytes!("../../assets/icon_dashboard.svg");
const ICON_CHANGE: &[u8] = include_bytes!("../../assets/icon_change.svg");
const ICON_CAPS: &[u8] = include_bytes!("../../assets/icon_caps.svg");
const ICON_CODEX: &[u8] = include_bytes!("../../assets/icon_codex.svg");
const ICON_REFRESH: &[u8] = include_bytes!("../../assets/icon_refresh.svg");

fn area_icon(area: Area) -> svg::Handle {
    let bytes: &'static [u8] = match area {
        Area::Dashboard => ICON_DASHBOARD,
        Area::Change => ICON_CHANGE,
        Area::Caps => ICON_CAPS,
        Area::Codex => ICON_CODEX,
    };
    svg::Handle::from_memory(bytes)
}

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
        let tint = if is_active { theme::accent() } else { theme::text_secondary() };
        let icon = svg(area_icon(area)).width(20).height(20).style(theme::svg_tint(tint));
        let btn = button(
            container(icon)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Center)
                .align_y(Center),
        )
        .width(36)
        .height(36)
        .on_press(on_area(area))
        .style(style);
        nav = nav.push(btn);
    }

    let refresh_icon = svg(svg::Handle::from_memory(ICON_REFRESH))
        .width(18)
        .height(18)
        .style(theme::svg_tint(theme::text_secondary()));
    let refresh = button(
        container(refresh_icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Center)
            .align_y(Center),
    )
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

