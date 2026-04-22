//! Selectable list primitive.
//!
//! Two layers:
//!
//! - [`ListRow`] — builder for a single row (icon + label + optional badge /
//!   trailing). Used standalone by file_finder and slash-completion for
//!   visual consistency.
//! - [`view`] — renders `Vec<ListRow>` as a column, with empty-state text.
//!   Callers own scrolling and section wrapping (`collapsible::view`).

use std::borrow::Cow;

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, svg, text, Space};
use iced::{Color, Element, Length};

use crate::theme;
use crate::widget::horizontal_pan;
use crate::widget::pan_row::PanRow;

const ICON_SIZE: f32 = 14.0;

type StyleFn = fn(&iced::Theme, button::Status) -> button::Style;

pub enum Badge {
    ErrorDot,
    ErrorCount(u32),
}

pub struct ListRow<'a, Msg> {
    label: Cow<'a, str>,
    icon: Option<&'static [u8]>,
    icon_tint: Option<Color>,
    leading: Option<Element<'a, Msg>>,
    trailing: Option<Element<'a, Msg>>,
    sticky_trailing: Option<Element<'a, Msg>>,
    badge: Option<Badge>,
    indent_level: usize,
    selected: bool,
    on_press: Option<Msg>,
    spacing: f32,
    fill_width: bool,
}

impl<'a, Msg: Clone + 'a> ListRow<'a, Msg> {
    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            icon_tint: None,
            leading: None,
            trailing: None,
            sticky_trailing: None,
            badge: None,
            indent_level: 0,
            selected: false,
            on_press: None,
            spacing: theme::SPACING_XS,
            fill_width: true,
        }
    }

    /// When `false`, the row's button uses `Length::Shrink` so the column
    /// has a discoverable natural max-row width. Required by sectioned
    /// lists wrapped in `horizontal_pan` — Fill-width buttons collapse
    /// during the pan widget's measure pass.
    pub fn fill_width(mut self, fill: bool) -> Self {
        self.fill_width = fill;
        self
    }

    /// Override horizontal spacing between leading / icon / label / trailing.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn icon(mut self, bytes: &'static [u8]) -> Self {
        self.icon = Some(bytes);
        self
    }

    /// Override the default `text_muted` tint applied to the icon SVG.
    pub fn icon_tint(mut self, tint: Color) -> Self {
        self.icon_tint = Some(tint);
        self
    }

    pub fn leading(mut self, el: Element<'a, Msg>) -> Self {
        self.leading = Some(el);
        self
    }

    pub fn trailing(mut self, el: Element<'a, Msg>) -> Self {
        self.trailing = Some(el);
        self
    }

    /// Trailing element pinned to the right edge of the visible viewport
    /// rather than the row's natural width — stays in place as the column
    /// pans horizontally. Only takes effect in the `view()` path
    /// (`fill_width = false`); in fill-width mode it renders identically
    /// to a regular `trailing` element.
    pub fn sticky_trailing(mut self, el: Element<'a, Msg>) -> Self {
        self.sticky_trailing = Some(el);
        self
    }

    pub fn badge(mut self, badge: Badge) -> Self {
        self.badge = Some(badge);
        self
    }

    pub fn indent(mut self, level: usize) -> Self {
        self.indent_level = level;
        self
    }

    pub fn selected(mut self, sel: bool) -> Self {
        self.selected = sel;
        self
    }

    pub fn on_press(mut self, msg: Msg) -> Self {
        self.on_press = Some(msg);
        self
    }

    pub fn into_element(self) -> Element<'a, Msg> {
        let mut inner = row![]
            .spacing(self.spacing)
            .align_y(iced::Center);

        if let Some(leading) = self.leading {
            inner = inner.push(leading);
        }
        if let Some(bytes) = self.icon {
            inner = inner.push(icon_svg(bytes, self.icon_tint));
        }

        inner = inner.push(
            text(self.label)
                .size(theme::font_md())
                .wrapping(Wrapping::None),
        );

        // In shrink-width mode, sticky_trailing is overlaid by PanRow at the
        // viewport's right edge. In fill-width mode there's no panning to
        // anchor against, so it falls back to a regular inline trailing.
        let mut sticky_trailing = self.sticky_trailing;
        let mut inline_trailing = self.trailing.or_else(|| self.badge.map(render_badge));
        if self.fill_width && inline_trailing.is_none() {
            inline_trailing = sticky_trailing.take();
        }
        if let Some(t) = inline_trailing {
            inner = inner.push(Space::new().width(Length::Fill));
            inner = inner.push(t);
        }

        let content: Element<'a, Msg> = if self.indent_level > 0 {
            let indent = (self.indent_level as f32) * theme::SPACING_LG;
            row![Space::new().width(indent), inner]
                .align_y(iced::Center)
                .into()
        } else {
            inner.into()
        };

        let style: StyleFn = if self.selected {
            theme::list_item_active
        } else {
            theme::list_item
        };

        if self.fill_width {
            let mut btn = button(content)
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style);
            if let Some(msg) = self.on_press {
                btn = btn.on_press(msg);
            }
            btn.into()
        } else {
            // Shrink-width path: a button at natural width can't paint a
            // viewport-spanning highlight. PanRow reads the visible viewport
            // and paints/hits across it instead.
            let mut row = PanRow::new(content)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style);
            if let Some(msg) = self.on_press {
                row = row.on_press(msg);
            }
            if let Some(st) = sticky_trailing {
                row = row.sticky_trailing(st);
            }
            row.into()
        }
    }
}

impl<'a, Msg: Clone + 'a> From<ListRow<'a, Msg>> for Element<'a, Msg> {
    fn from(row: ListRow<'a, Msg>) -> Self {
        row.into_element()
    }
}

pub fn view<'a, Msg: Clone + 'a>(
    rows: Vec<ListRow<'a, Msg>>,
    empty: Option<&str>,
) -> Element<'a, Msg> {
    if rows.is_empty() {
        return match empty {
            Some(s) => container(
                text(s.to_string())
                    .size(theme::font_md())
                    .color(theme::text_muted()),
            )
            .padding([theme::SPACING_XS, theme::SPACING_SM])
            .into(),
            None => Space::new().into(),
        };
    }

    let mut col = column![].spacing(0.0);
    for r in rows {
        col = col.push(r.fill_width(false).into_element());
    }
    horizontal_pan::view(col)
}

fn icon_svg<'a, Msg: 'a>(bytes: &'static [u8], tint: Option<Color>) -> Element<'a, Msg> {
    svg(svg::Handle::from_memory(bytes))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(theme::svg_tint(tint.unwrap_or_else(theme::text_muted)))
        .into()
}

fn render_badge<'a, Msg: 'a>(badge: Badge) -> Element<'a, Msg> {
    match badge {
        Badge::ErrorDot => text("\u{2022}")
            .size(theme::font_md())
            .color(theme::error())
            .into(),
        Badge::ErrorCount(n) => text(n.to_string())
            .size(theme::font_sm())
            .color(theme::error())
            .into(),
    }
}
