//! Thin status bar strip showing the user's current selection path.
//!
//! Each area produces its own `Vec<String>` of segments via its own
//! `breadcrumbs(...)` function; this widget renders them uniformly. An
//! optional `trailing` label is right-aligned and used by the shell to show
//! the current project folder.

use iced::widget::text::Wrapping;
use iced::widget::{Space, container, row, text};
use iced::{Element, Length};

use crate::theme;

pub fn view<'a, Msg: 'a>(
    segments: Vec<String>,
    trailing: Option<String>,
    hint: Option<String>,
) -> Element<'a, Msg> {
    let mut bar = row![].spacing(theme::SPACING_XS);
    let last = segments.len().saturating_sub(1);
    for (i, seg) in segments.into_iter().enumerate() {
        if i > 0 {
            bar = bar.push(
                text("\u{203a}")
                    .size(theme::font_sm())
                    .color(theme::text_muted()),
            );
        }
        let color = if i == last {
            theme::text_primary()
        } else {
            theme::text_muted()
        };
        bar = bar.push(
            text(seg)
                .size(theme::font_sm())
                .wrapping(Wrapping::None)
                .color(color),
        );
    }
    let has_trailing = trailing.is_some() || hint.is_some();
    if has_trailing {
        bar = bar.push(Space::new().width(Length::Fill));
    }
    if let Some(h) = hint {
        bar = bar.push(
            text(h)
                .size(theme::font_sm())
                .wrapping(Wrapping::None)
                .color(theme::text_muted()),
        );
    }
    if let Some(label) = trailing {
        bar = bar.push(
            text(label)
                .size(theme::font_sm())
                .wrapping(Wrapping::None)
                .color(theme::text_secondary()),
        );
    }
    container(bar)
        .padding([2.0, theme::SPACING_SM])
        .width(Length::Fill)
        .style(theme::surface)
        .into()
}
