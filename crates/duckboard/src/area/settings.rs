//! Settings area — font configuration UI.

use iced::widget::{
    button, column, container, pick_list, row, scrollable, slider, text, Space,
};
use iced::{Center, Element, Length};

use crate::config::{self, Config};
use crate::theme;

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct State {
    pub system_fonts: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            system_fonts: Vec::new(),
        }
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    LoadFonts,
    UiFontSelected(String),
    UiFontSizeChanged(f32),
    ContentFontSelected(String),
    ContentFontSizeChanged(f32),
    ResetDefaults,
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(state: &mut State, config: &mut Config, message: Message) {
    match message {
        Message::LoadFonts => {
            if state.system_fonts.is_empty() {
                state.system_fonts = config::list_system_fonts();
            }
        }
        Message::UiFontSelected(family) => {
            config.ui.font_family = family;
            let _ = config::save(config);
        }
        Message::UiFontSizeChanged(size) => {
            config.ui.font_size = size;
            let _ = config::save(config);
        }
        Message::ContentFontSelected(family) => {
            config.content.font_family = family;
            let _ = config::save(config);
        }
        Message::ContentFontSizeChanged(size) => {
            config.content.font_size = size;
            let _ = config::save(config);
        }
        Message::ResetDefaults => {
            *config = Config::default();
            let _ = config::save(config);
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a State, config: &'a Config) -> Element<'a, Message> {
    let heading = text("Settings").size(22.0).color(theme::text_primary());

    let ui_section = font_section(
        "UI Font",
        "Font used for interface elements like labels, buttons, and navigation.",
        &config.ui.font_family,
        config.ui.font_size,
        &state.system_fonts,
        Message::UiFontSelected,
        Message::UiFontSizeChanged,
    );

    let content_section = font_section(
        "Content Font",
        "Font used for code, file content, and the terminal.",
        &config.content.font_family,
        config.content.font_size,
        &state.system_fonts,
        Message::ContentFontSelected,
        Message::ContentFontSizeChanged,
    );

    let reset = button(
        text("Reset to defaults")
            .size(theme::font_sm())
            .color(theme::text_secondary()),
    )
    .on_press(Message::ResetDefaults)
    .style(theme::dashboard_action);

    let body = column![
        heading,
        Space::new().height(theme::SPACING_XL),
        ui_section,
        Space::new().height(theme::SPACING_XL),
        content_section,
        Space::new().height(theme::SPACING_XL),
        reset,
    ]
    .width(Length::Fill)
    .max_width(480);

    container(
        scrollable(
            container(body)
                .padding([theme::SPACING_XL, theme::SPACING_XL]),
        )
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::surface)
    .into()
}

fn font_section<'a>(
    title: &'a str,
    description: &'a str,
    current_family: &'a str,
    current_size: f32,
    system_fonts: &'a [String],
    on_font: impl Fn(String) -> Message + 'a,
    on_size: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    let label = text(title)
        .size(theme::font_md())
        .color(theme::text_primary());
    let desc = text(description)
        .size(theme::font_sm())
        .color(theme::text_muted());

    let selected = if current_family.is_empty() {
        None
    } else {
        Some(current_family.to_string())
    };

    let font_picker = pick_list(
        system_fonts.to_vec(),
        selected,
        on_font,
    )
    .placeholder("System default")
    .width(280);

    let size_label = text(format!("{:.0}px", current_size))
        .size(theme::font_sm())
        .color(theme::text_secondary())
        .width(40)
        .align_x(Center);

    let size_slider = slider(8.0..=32.0, current_size, on_size).step(1.0).width(200);

    let size_row = row![
        text("Size").size(theme::font_sm()).color(theme::text_secondary()),
        size_slider,
        size_label,
    ]
    .spacing(theme::SPACING_SM)
    .align_y(Center);

    column![
        label,
        desc,
        Space::new().height(theme::SPACING_SM),
        font_picker,
        Space::new().height(theme::SPACING_SM),
        size_row,
    ]
    .spacing(theme::SPACING_XS)
    .into()
}
