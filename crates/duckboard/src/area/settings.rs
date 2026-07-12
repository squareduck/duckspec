//! Settings area — fonts, global default model, chat affordances, and per-project override.

use std::path::Path;

use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, slider, text, toggler,
};
use iced::{Center, Element, Length};

use crate::agent;
use crate::config::{self, Config};
use crate::theme;
use crate::widget::agent_chat::{self, ModelChoice};
use duckchat::{ModelInfo, ModelRef};

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct State {
    pub system_fonts: Vec<String>,
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    LoadFonts,
    UiFontSelected(String),
    UiFontSizeChanged(f32),
    ContentFontSelected(String),
    ContentFontSizeChanged(f32),
    /// Global main-chat default (concrete choice only).
    GlobalModelSelected(ModelChoice),
    /// Project override (`id == None` → use global default).
    ModelDefaultSelected(ModelChoice),
    AgentInputHintsToggled(bool),
    /// Global oneshot model for a harness (`choice.id` is the model id).
    OneshotModelSelected { harness: String, choice: ModelChoice },
    ResetDefaults,
}

/// Which harnesses should offer an oneshot model picker.
///
/// Empty when agent input hints are off, or when a harness has no catalog models.
/// Order follows the catalog harness order of the provided slices.
#[cfg(test)]
pub fn oneshot_picker_harnesses(
    agent_input_hints: bool,
    catalog: &[(impl AsRef<str>, &[ModelInfo])],
) -> Vec<String> {
    if !agent_input_hints {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (harness, models) in catalog {
        if !models.is_empty() {
            out.push(harness.as_ref().to_string());
        }
    }
    out
}

/// Harness ids with non-empty process-catalog slices, in catalog order.
fn oneshot_harnesses_from_process_catalog() -> Vec<String> {
    let models = agent::available_models();
    let mut out = Vec::new();
    for m in &models {
        if !out.iter().any(|h| h == &m.harness) {
            out.push(m.harness.clone());
        }
    }
    out
}

fn harness_label(harness: &str) -> &str {
    match harness {
        "claude-code" => "Claude Code",
        "grok" => "Grok",
        other => other,
    }
}

fn oneshot_choices_for(models: &[ModelInfo]) -> Vec<ModelChoice> {
    models
        .iter()
        .map(|m| ModelChoice {
            harness: Some(m.harness.clone()),
            id: Some(m.id.clone()),
            label: m.display.clone(),
            closed_label: m.display.clone(),
        })
        .collect()
}

/// Selected oneshot picker choice for a harness: same ladder as the worker
/// (`resolve_oneshot_model`), not the first catalog entry when config is unset.
pub fn selected_oneshot_choice(
    harness: &str,
    configured: Option<&str>,
    models: &[ModelInfo],
) -> ModelChoice {
    let choices = oneshot_choices_for(models);
    let resolved = agent::resolve_oneshot_model(harness, configured, models);
    let selected_ref = resolved.map(|id| ModelRef::new(harness, id));
    agent_chat::selected_model_choice(&choices, selected_ref.as_ref())
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(
    state: &mut State,
    config: &mut Config,
    project_root: Option<&Path>,
    message: Message,
) {
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
        Message::GlobalModelSelected(choice) => {
            if let Some(model) = choice.to_ref() {
                config.set_global_model_default(Some(model));
                let _ = config::save(config);
            }
        }
        Message::ModelDefaultSelected(choice) => {
            if let Some(root) = project_root {
                // A real choice carries its own harness; the sentinel maps to `None`
                // (use global default).
                let model = choice.to_ref();
                config.set_project_model_default(root, model);
                let _ = config::save(config);
            }
        }
        Message::AgentInputHintsToggled(on) => {
            config.chat.agent_input_hints = on;
            let _ = config::save(config);
        }
        Message::OneshotModelSelected { harness, choice } => {
            let model = choice.id;
            config.chat.set_oneshot_model(&harness, model);
            let _ = config::save(config);
        }
        Message::ResetDefaults => {
            *config = Config::default();
            // Re-seed a concrete global default when the process catalog has
            // models — ModelCatalogReady is one-shot and will not re-seed.
            let catalog = agent::available_models();
            let _ = agent::seed_global_default_if_unset(config, &catalog);
            let _ = config::save(config);
        }
    }
}

/// Closed selection for the global default picker: a real catalog choice when
/// set and available; **Missing** when unset or absent from the catalog.
fn global_default_picker_selected(config: &Config, choices: &[ModelChoice]) -> ModelChoice {
    match config.global_model_default() {
        Some(m)
            if choices.iter().any(|c| {
                c.harness.as_deref() == Some(m.harness.as_str())
                    && c.id.as_deref() == Some(m.model.as_str())
            }) =>
        {
            agent_chat::selected_model_choice(choices, Some(m))
        }
        Some(m) => agent_chat::missing_closed_model_choice(Some(m)),
        None => agent_chat::missing_closed_model_choice(None),
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    state: &'a State,
    config: &'a Config,
    project_root: Option<&Path>,
) -> Element<'a, Message> {
    let heading = text("Settings").size(22.0).color(theme::text_primary());

    let global_heading = text("Global")
        .size(theme::font_lg())
        .color(theme::text_primary());

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

    let global_model = global_model_section(config);
    let chat_section = chat_section(config);

    let mut body = column![
        heading,
        Space::new().height(theme::SPACING_XL),
        global_heading,
        Space::new().height(theme::SPACING_MD),
        ui_section,
        Space::new().height(theme::SPACING_XL),
        content_section,
        Space::new().height(theme::SPACING_XL),
        global_model,
        Space::new().height(theme::SPACING_XL),
        chat_section,
    ];

    if let Some(root) = project_root {
        let project_heading = text("This project")
            .size(theme::font_lg())
            .color(theme::text_primary());
        // Extra gap so Global and This project read as peer top-level sections.
        body = body
            .push(Space::new().height(theme::SPACING_XL))
            .push(Space::new().height(theme::SPACING_LG))
            .push(project_heading)
            .push(Space::new().height(theme::SPACING_MD))
            .push(project_model_section(config, root));
    }

    let reset = button(
        text("Reset to defaults")
            .size(theme::font_sm())
            .color(theme::text_secondary()),
    )
    .on_press(Message::ResetDefaults)
    .style(theme::dashboard_action);

    let body = body
        .push(Space::new().height(theme::SPACING_XL))
        .push(reset)
        .width(Length::Fill)
        .max_width(480);

    container(
        scrollable(container(body).padding([theme::SPACING_XL, theme::SPACING_XL]))
            .direction(theme::thin_scrollbar_direction())
            .style(theme::thin_scrollbar),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::surface)
    .into()
}

fn global_model_section<'a>(config: &Config) -> Element<'a, Message> {
    let label = text("Default Model")
        .size(theme::font_md())
        .color(theme::text_primary());
    let desc = text(
        "Model new chats use by default in every project. A project override or \
         per-chat selection can narrow it.",
    )
    .size(theme::font_sm())
    .color(theme::text_muted());

    let choices = agent_chat::global_model_choices();
    let selected = global_default_picker_selected(config, &choices);
    let picker = pick_list(choices, Some(selected), Message::GlobalModelSelected)
        .width(280)
        .style(theme::pick_list_style)
        .menu_style(theme::pick_list_menu);

    column![label, desc, Space::new().height(theme::SPACING_SM), picker]
        .spacing(theme::SPACING_XS)
        .into()
}

fn chat_section<'a>(config: &Config) -> Element<'a, Message> {
    let label = text("Chat")
        .size(theme::font_md())
        .color(theme::text_primary());
    let desc = text(
        "Optional freeform reply chips after a turn (⌘-number when shown). \
         Applies to all projects.",
    )
    .size(theme::font_sm())
    .color(theme::text_muted());

    let agent_row = toggler(config.chat.agent_input_hints)
        .label("Agent input hints")
        .on_toggle(Message::AgentInputHintsToggled);
    let agent_help = text(
        "When enabled (default off), a settled oneshot may offer up to three freeform \
         replies as fast-response chips — only while idle, with no next-action ghost, \
         and not while a question tool is active. Click or ⌘n to send.",
    )
    .size(theme::font_sm())
    .color(theme::text_muted());

    let mut col = column![
        label,
        desc,
        Space::new().height(theme::SPACING_SM),
        agent_row,
        agent_help,
    ]
    .spacing(theme::SPACING_XS);

    if config.chat.agent_input_hints {
        let oneshot_intro = text(
            "Oneshot model (titles and reply chips) per agent backend. Global — not \
             per project. Only backends with models available on this machine.",
        )
        .size(theme::font_sm())
        .color(theme::text_muted());
        col = col
            .push(Space::new().height(theme::SPACING_SM))
            .push(oneshot_intro);

        for harness in oneshot_harnesses_from_process_catalog() {
            let models = agent::models_for_harness(&harness);
            if models.is_empty() {
                continue;
            }
            let choices = oneshot_choices_for(&models);
            let selected = selected_oneshot_choice(
                &harness,
                config.chat.oneshot_model(&harness),
                &models,
            );
            let harness_owned = harness.clone();
            let picker = pick_list(choices, Some(selected), move |choice| {
                Message::OneshotModelSelected {
                    harness: harness_owned.clone(),
                    choice,
                }
            })
            .width(280)
            .style(theme::pick_list_style)
            .menu_style(theme::pick_list_menu);

            let row_label = text(format!("Oneshot · {}", harness_label(&harness)))
                .size(theme::font_sm())
                .color(theme::text_secondary());
            col = col
                .push(Space::new().height(theme::SPACING_SM))
                .push(row_label)
                .push(picker);
        }
    }

    col.into()
}

fn project_model_section<'a>(config: &Config, root: &Path) -> Element<'a, Message> {
    let label = text("Default Model")
        .size(theme::font_md())
        .color(theme::text_primary());
    let desc = text(
        "Override the global default for new chats in this project. A per-chat \
         selection still wins. “Use global default” clears the override.",
    )
    .size(theme::font_sm())
    .color(theme::text_muted());

    let choices = agent_chat::project_override_model_choices();
    let current = config.project_model_default(root);
    let selected = agent_chat::selected_model_choice(&choices, current.as_ref());
    let picker = pick_list(choices, Some(selected), Message::ModelDefaultSelected)
        .width(280)
        .style(theme::pick_list_style)
        .menu_style(theme::pick_list_menu);

    column![label, desc, Space::new().height(theme::SPACING_SM), picker]
        .spacing(theme::SPACING_XS)
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

    let font_picker = pick_list(system_fonts.to_vec(), selected, on_font)
        .placeholder("System default")
        .width(280)
        .style(theme::pick_list_style)
        .menu_style(theme::pick_list_menu);

    let size_label = text(format!("{:.0}px", current_size))
        .size(theme::font_sm())
        .color(theme::text_secondary())
        .width(40)
        .align_x(Center);

    let size_slider = slider(8.0_f32..=32.0_f32, current_size, on_size)
        .step(1.0_f32)
        .width(200);

    let size_row = row![
        text("Size")
            .size(theme::font_sm())
            .color(theme::text_secondary()),
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

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs() -> Vec<String> {
    vec!["Settings".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mi(harness: &str, id: &str) -> ModelInfo {
        ModelInfo {
            harness: harness.into(),
            id: id.into(),
            display: id.into(),
            context_window: None,
        }
    }

    /// @spec chat/oneshot-models Settings pickers when hints enabled: With agent input hints on, each harness with catalog models offers an oneshot model picker
    #[test]
    fn with_agent_input_hints_on_each_harness_with_catalog_models_offers_picker() {
        // GIVEN agent input hints enabled
        // AND at least one harness with a non-empty catalog slice
        let claude = vec![mi("claude-code", "haiku")];
        let grok = vec![mi("grok", "grok-4.5")];
        let catalog: [(&str, &[ModelInfo]); 2] = [
            ("claude-code", &claude),
            ("grok", &grok),
        ];

        // WHEN the Chat settings section is shown
        let harnesses = oneshot_picker_harnesses(true, &catalog);

        // THEN an oneshot model picker is offered for each harness that has catalog models
        assert_eq!(harnesses, vec!["claude-code", "grok"]);
    }

    /// @spec chat/oneshot-models Settings pickers when hints enabled: With agent input hints off, oneshot model pickers are not shown
    #[test]
    fn with_agent_input_hints_off_oneshot_model_pickers_are_not_shown() {
        // GIVEN agent input hints disabled
        // AND at least one harness with a non-empty catalog slice
        let claude = vec![mi("claude-code", "haiku")];
        let catalog: [(&str, &[ModelInfo]); 1] = [("claude-code", &claude)];

        // WHEN the Chat settings section is shown
        let harnesses = oneshot_picker_harnesses(false, &catalog);

        // THEN no oneshot model picker is shown
        assert!(harnesses.is_empty());
    }

    #[test]
    fn unset_config_selects_string_match_default_not_first_catalog_entry() {
        // Claude: first catalog is sonnet; string-match default is haiku.
        let claude = vec![
            mi("claude-code", "claude-sonnet-5"),
            mi("claude-code", "claude-haiku-4-5"),
            mi("claude-code", "claude-opus-4-8"),
        ];
        let selected = selected_oneshot_choice("claude-code", None, &claude);
        assert_eq!(
            selected.id.as_deref(),
            Some("claude-haiku-4-5"),
            "unset Claude oneshot must prefer haiku match, not first catalog entry"
        );

        // Grok: first is grok-4.5; string-match default is composer-fast.
        let grok = vec![
            mi("grok", "grok-4.5"),
            mi("grok", "grok-composer-2.5-fast"),
        ];
        let selected = selected_oneshot_choice("grok", None, &grok);
        assert_eq!(
            selected.id.as_deref(),
            Some("grok-composer-2.5-fast"),
            "unset Grok oneshot must prefer composer-fast, not first catalog entry"
        );
    }

    #[test]
    fn configured_oneshot_id_in_catalog_is_selected() {
        let claude = vec![
            mi("claude-code", "claude-sonnet-5"),
            mi("claude-code", "claude-haiku-4-5"),
        ];
        let selected =
            selected_oneshot_choice("claude-code", Some("claude-sonnet-5"), &claude);
        assert_eq!(selected.id.as_deref(), Some("claude-sonnet-5"));
    }

    #[test]
    fn empty_catalog_harness_is_skipped_by_oneshot_picker_helper() {
        let claude = vec![mi("claude-code", "haiku")];
        let empty: Vec<ModelInfo> = Vec::new();
        let catalog: [(&str, &[ModelInfo]); 2] = [
            ("claude-code", &claude),
            ("grok", &empty),
        ];
        let harnesses = oneshot_picker_harnesses(true, &catalog);
        assert_eq!(harnesses, vec!["claude-code"]);
    }

    #[test]
    fn global_picker_shows_missing_when_unset() {
        let cfg = Config::default();
        let choices = vec![ModelChoice {
            harness: Some("grok".into()),
            id: Some("grok-4.5".into()),
            label: "Grok · Grok 4.5".into(),
            closed_label: "Grok 4.5".into(),
        }];
        let selected = global_default_picker_selected(&cfg, &choices);
        assert_eq!(selected.closed_label, "Missing");
        assert_eq!(selected.label, "Missing");
    }

    #[test]
    fn global_picker_shows_missing_when_configured_model_not_in_choices() {
        let mut cfg = Config::default();
        cfg.set_global_model_default(Some(ModelRef::new("grok", "gone")));
        let choices = vec![ModelChoice {
            harness: Some("claude-code".into()),
            id: Some("sonnet".into()),
            label: "Claude Code · Sonnet".into(),
            closed_label: "Sonnet".into(),
        }];
        let selected = global_default_picker_selected(&cfg, &choices);
        assert_eq!(selected.closed_label, "Missing");
        assert_eq!(selected.id.as_deref(), Some("gone"));
    }

    #[test]
    fn global_picker_shows_catalog_choice_when_set_and_available() {
        let mut cfg = Config::default();
        cfg.set_global_model_default(Some(ModelRef::new("grok", "grok-4.5")));
        let choices = vec![ModelChoice {
            harness: Some("grok".into()),
            id: Some("grok-4.5".into()),
            label: "Grok · Grok 4.5".into(),
            closed_label: "Grok 4.5".into(),
        }];
        let selected = global_default_picker_selected(&cfg, &choices);
        assert_eq!(selected.closed_label, "Grok 4.5");
        assert_eq!(selected.id.as_deref(), Some("grok-4.5"));
    }

    #[test]
    fn reset_reseeds_global_default_from_catalog() {
        let mut cfg = Config::default();
        cfg.set_global_model_default(Some(ModelRef::new("claude-code", "opus")));
        // Simulate ResetDefaults body without process catalog dependency:
        cfg = Config::default();
        let catalog = vec![
            mi("claude-code", "sonnet"),
            mi("grok", "grok-4.5"),
        ];
        assert!(agent::seed_global_default_if_unset(&mut cfg, &catalog));
        assert_eq!(
            cfg.global_model_default(),
            Some(&ModelRef::new("grok", "grok-4.5"))
        );
    }
}
