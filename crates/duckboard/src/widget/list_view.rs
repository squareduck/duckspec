//! Selectable list primitive.
//!
//! Two layers:
//!
//! - [`ListRow`] — builder for a single row (leading + icon + label). Used
//!   standalone by file_finder and slash-completion for visual consistency.
//! - [`view`] — renders `Vec<ListRow>` as a column, with empty-state text.
//!   Callers own scrolling and section wrapping (`collapsible::view`).
//!
//! Error state is conveyed by coloring the label and icon red via the
//! [`ListRow::errored`] setter. Rows carry no trailing indicators, so
//! horizontal panning (see [`super::horizontal_pan`]) can scroll freely
//! without hiding per-row affordances.

use std::borrow::Cow;

use iced::widget::text::Wrapping;
use iced::widget::{MouseArea, Space, button, column, container, row, svg, text};
use iced::{Color, Element, Length};

use crate::theme;
use crate::widget::horizontal_pan;
use crate::widget::pan_row::PanRow;

pub const ICON_SIZE: f32 = 14.0;

type StyleFn = fn(&iced::Theme, button::Status) -> button::Style;

pub struct ListRow<'a, Msg> {
    label: Cow<'a, str>,
    icon: Option<&'static [u8]>,
    icon_tint: Option<Color>,
    leading: Option<Element<'a, Msg>>,
    indent_level: usize,
    selected: bool,
    errored: bool,
    on_press: Option<Msg>,
    /// (on_enter, on_exit) — wraps the row in a mouse_area so callers can
    /// track cursor hover (e.g. to swap the icon for a close button).
    hover: Option<(Msg, Msg)>,
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
            indent_level: 0,
            selected: false,
            errored: false,
            on_press: None,
            hover: None,
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

    /// Override horizontal spacing between leading / icon / label.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn icon(mut self, bytes: &'static [u8]) -> Self {
        self.icon = Some(bytes);
        self
    }

    /// Override the default `text_muted` tint applied to the icon SVG.
    /// Ignored when the row is `errored` — error tint wins.
    pub fn icon_tint(mut self, tint: Color) -> Self {
        self.icon_tint = Some(tint);
        self
    }

    pub fn leading(mut self, el: Element<'a, Msg>) -> Self {
        self.leading = Some(el);
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

    /// Mark the row as containing errors. Tints the label and the icon red;
    /// the specific diagnostic is expected to be shown elsewhere (inline
    /// error panel, dashboard).
    pub fn errored(mut self, err: bool) -> Self {
        self.errored = err;
        self
    }

    pub fn on_press(mut self, msg: Msg) -> Self {
        self.on_press = Some(msg);
        self
    }

    /// Emit `enter` when the cursor enters the row and `exit` when it
    /// leaves. Click behavior is unaffected — the underlying button still
    /// fires `on_press`.
    pub fn on_hover(mut self, enter: Msg, exit: Msg) -> Self {
        self.hover = Some((enter, exit));
        self
    }

    pub fn into_element(self) -> Element<'a, Msg> {
        let mut inner = row![].spacing(self.spacing).align_y(iced::Center);

        if let Some(leading) = self.leading {
            inner = inner.push(leading);
        }
        if let Some(bytes) = self.icon {
            let tint = if self.errored {
                Some(theme::error())
            } else {
                self.icon_tint
            };
            inner = inner.push(icon_svg(bytes, tint));
        }

        let mut label = text(self.label)
            .size(theme::font_md())
            .wrapping(Wrapping::None);
        if self.errored {
            label = label.color(theme::error());
        }
        inner = inner.push(label);

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
            let elem: Element<'a, Msg> = btn.into();
            match self.hover {
                Some((enter, exit)) => MouseArea::new(elem)
                    .on_enter(enter)
                    .on_exit(exit)
                    .into(),
                None => elem,
            }
        } else {
            // Shrink-width path: a button at natural width can't paint a
            // viewport-spanning highlight. PanRow reads the visible viewport
            // and paints/hits across it instead. Hover is driven by PanRow
            // rather than a wrapping `MouseArea` — PanRow's hover state
            // survives content swaps inside the row (e.g. icon ↔ close
            // button), whereas `MouseArea` recomputes from `layout.bounds`
            // and can oscillate when those bounds change between renders.
            let mut row = PanRow::new(content)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style);
            if let Some(msg) = self.on_press {
                row = row.on_press(msg);
            }
            if let Some((enter, exit)) = self.hover {
                row = row.on_hover(enter, exit);
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
