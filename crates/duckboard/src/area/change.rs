//! Change area — single change workspace with three-column layout.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, row, scrollable, svg, text, Space};
use iced::widget::text::Wrapping;
use iced::{Element, Length};

use crate::data::{ChangeData, ProjectData, StepCompletion};
use crate::theme;
use crate::vcs::{self, ChangedFile, FileStatus};
use crate::widget::{collapsible, interaction_toggle, tab_bar, tree_view};

use super::interaction::{self, InteractionMode, InteractionState};

const ICON_BRANCH: &[u8] = include_bytes!("../../assets/icon_branch.svg");
const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");
const ICON_SPEC: &[u8] = include_bytes!("../../assets/icon_spec.svg");
const ICON_DOC: &[u8] = include_bytes!("../../assets/icon_doc.svg");
const ICON_SPEC_DELTA: &[u8] = include_bytes!("../../assets/icon_spec_delta.svg");
const ICON_DOC_DELTA: &[u8] = include_bytes!("../../assets/icon_doc_delta.svg");
const ICON_STEP: &[u8] = include_bytes!("../../assets/icon_step.svg");
const ICON_STEP_DONE: &[u8] = include_bytes!("../../assets/icon_step_done.svg");
const ICON_STEP_PARTIAL: &[u8] = include_bytes!("../../assets/icon_step_partial.svg");
const ICON_EXPLORE: &[u8] = include_bytes!("../../assets/icon_explore.svg");

const ICON_SIZE: f32 = 14.0;

// ── State ────────────────────────────────────────────────────────────────────

pub struct State {
    pub selected_change: Option<String>,
    pub expanded_sections: HashSet<String>,
    pub expanded_nodes: HashSet<String>,
    pub tabs: tab_bar::TabState,
    pub changed_files: Vec<ChangedFile>,
    /// Per-change interaction states keyed by change name.
    /// Switching changes keeps previous sessions alive.
    pub interactions: HashMap<String, InteractionState>,
    /// Virtual exploration changes (not persisted to duckspec).
    pub explorations: Vec<String>,
    /// Counter for generating unique exploration names.
    pub exploration_counter: usize,
    /// When true, content column shows the audit view.
    pub audit_active: bool,
}

impl State {
    pub fn new(project_root: Option<&Path>) -> Self {
        let mut sections = HashSet::new();
        sections.insert("overview".to_string());
        sections.insert("capabilities".to_string());
        sections.insert("steps".to_string());
        sections.insert("changed_files".to_string());
        let (explorations, exploration_counter) =
            crate::chat_store::load_explorations(project_root);
        Self {
            selected_change: None,
            expanded_sections: sections,
            expanded_nodes: HashSet::new(),
            tabs: tab_bar::TabState::default(),
            changed_files: vec![],
            interactions: HashMap::new(),
            explorations,
            exploration_counter,
            audit_active: false,
        }
    }
}

impl State {
    /// Get the active change's interaction state (if a change is selected).
    pub fn active_interaction(&self) -> Option<&InteractionState> {
        let name = self.selected_change.as_ref()?;
        self.interactions.get(name)
    }

    /// Get the active change's interaction state mutably, creating it if needed.
    pub fn active_interaction_mut(&mut self) -> Option<&mut InteractionState> {
        let name = self.selected_change.as_ref()?;
        Some(self.interactions.entry(name.clone()).or_default())
    }

    /// Whether the currently selected change is an exploration (virtual).
    pub fn is_exploration_selected(&self) -> bool {
        self.selected_change
            .as_ref()
            .is_some_and(|name| self.explorations.contains(name))
    }

    /// Promote an exploration to a real change: remove from explorations list,
    /// migrate interaction state and chat session to the new name.
    pub fn promote_exploration(
        &mut self,
        exploration_name: &str,
        real_name: &str,
        project_root: Option<&Path>,
    ) {
        self.explorations.retain(|n| n != exploration_name);
        if let Some(ix) = self.interactions.remove(exploration_name) {
            self.interactions.insert(real_name.to_string(), ix);
        }
        if self.selected_change.as_deref() == Some(exploration_name) {
            self.selected_change = Some(real_name.to_string());
        }
        crate::chat_store::save_explorations(
            &self.explorations,
            self.exploration_counter,
            project_root,
        );
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SelectChange(String),
    ToggleSection(String),
    ToggleNode(String),
    SelectItem(String),
    SelectTab(usize),
    CloseTab(usize),
    Interaction(interaction::Msg),
    SelectChangedFile(PathBuf),
    TabContent(tab_bar::TabContentMsg),
    AddExploration,
    RemoveExploration(String),
    ShowAudit,
    RefreshAudit,
    SelectAuditError { change: String, artifact_id: String },
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(
    state: &mut State,
    message: Message,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    match message {
        Message::SelectChange(name) => {
            state.selected_change = Some(name.clone());
            state.audit_active = false;
            state.expanded_nodes.clear();

            // Expand tree nodes for real changes.
            if !state.explorations.contains(&name)
                && let Some(change) = project
                    .active_changes
                    .iter()
                    .chain(project.archived_changes.iter())
                    .find(|c| c.name == name)
            {
                crate::data::TreeNode::collect_parent_ids(
                    &change.cap_tree,
                    &mut state.expanded_nodes,
                );
            }

            // Auto-open interaction for any change.
            let ix = state.interactions.entry(name.clone()).or_default();
            if !ix.visible {
                ix.visible = true;
                if ix.mode == InteractionMode::Terminal {
                    interaction::spawn_terminal(ix);
                }
                if ix.mode == InteractionMode::AgentChat && ix.chat_session.is_none() {
                    interaction::spawn_agent_session(ix, &name, project.project_root.as_deref(), highlighter);
                }
            }
        }
        Message::ToggleSection(id) => {
            if !state.expanded_sections.remove(&id) {
                state.expanded_sections.insert(id);
            }
        }
        Message::ToggleNode(id) => {
            if !state.expanded_nodes.remove(&id) {
                state.expanded_nodes.insert(id);
            }
        }
        Message::SelectItem(id) => {
            open_artifact(state, &id, project, highlighter);
        }
        Message::SelectTab(idx) => state.tabs.select(idx),
        Message::CloseTab(idx) => state.tabs.close(idx),
        Message::Interaction(msg) => {
            let session_name = state
                .selected_change
                .clone()
                .unwrap_or_else(|| "default".to_string());
            let Some(ix) = state.active_interaction_mut() else { return };
            let is_mode_switch = matches!(msg, interaction::Msg::SwitchMode(_));
            let just_opened = interaction::update(ix, msg, highlighter);

            if just_opened && ix.mode == InteractionMode::Terminal {
                interaction::spawn_terminal(ix);
            }
            if is_mode_switch && ix.mode == InteractionMode::Terminal {
                interaction::spawn_terminal(ix);
            }

            let wants_agent = (just_opened || is_mode_switch) && ix.mode == InteractionMode::AgentChat;
            if wants_agent && ix.chat_session.is_none() {
                interaction::spawn_agent_session(ix, &session_name, project.project_root.as_deref(), highlighter);
            }

            ix.terminal_focused = ix.visible && ix.mode == InteractionMode::Terminal;
        }
        Message::SelectChangedFile(path) => {
            state.audit_active = false;
            if let Some(root) = &project.project_root
                && let Some(diff) = vcs::file_diff(root, &path) {
                    let id = format!("vcs:{}", path.display());
                    let title = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());

                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("txt");
                    let syntax = highlighter.find_syntax(ext);
                    let old_lines: Vec<String> = diff.old_content.lines().map(String::from).collect();
                    let new_lines: Vec<String> = diff.new_content.lines().map(String::from).collect();
                    let highlight = crate::widget::diff_view::DiffHighlight {
                        old_spans: highlighter.highlight_lines(&old_lines, syntax),
                        new_spans: highlighter.highlight_lines(&new_lines, syntax),
                    };

                    let diff_status = diff.status;
                    let diff_path = diff.path.clone();
                    let editor = crate::widget::diff_view::build_editor(&diff, Some(&highlight));
                    state.tabs.open_diff(id, title, editor, diff_path, diff_status);
                }
        }
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(action)) => {
            crate::handle_editor_action(&mut state.tabs, action, highlighter);
        }
        Message::AddExploration => {
            state.exploration_counter += 1;
            let name = format!("Exploration {}", state.exploration_counter);
            state.explorations.push(name.clone());
            state.selected_change = Some(name.clone());
            crate::chat_store::save_explorations(&state.explorations, state.exploration_counter, project.project_root.as_deref());
            // Auto-open interaction panel.
            let ix = state.interactions.entry(name.clone()).or_default();
            ix.visible = true;
            if ix.mode == InteractionMode::AgentChat && ix.chat_session.is_none() {
                interaction::spawn_agent_session(ix, &name, project.project_root.as_deref(), highlighter);
            }
            if ix.mode == InteractionMode::Terminal {
                interaction::spawn_terminal(ix);
            }
        }
        Message::RemoveExploration(name) => {
            state.explorations.retain(|n| n != &name);
            state.interactions.remove(&name);
            if state.selected_change.as_deref() == Some(&name) {
                state.selected_change = None;
            }
            crate::chat_store::delete_session(&name, project.project_root.as_deref());
            crate::chat_store::save_explorations(&state.explorations, state.exploration_counter, project.project_root.as_deref());
        }
        Message::ShowAudit => {
            state.audit_active = true;
            state.selected_change = None;
        }
        Message::RefreshAudit => {
            // Handled by main.rs — calls project.revalidate().
        }
        Message::SelectAuditError { change, artifact_id } => {
            state.audit_active = false;
            state.selected_change = Some(change.clone());
            state.expanded_nodes.clear();
            if let Some(ch) = project
                .active_changes
                .iter()
                .chain(project.archived_changes.iter())
                .find(|c| c.name == change)
            {
                crate::data::TreeNode::collect_parent_ids(
                    &ch.cap_tree,
                    &mut state.expanded_nodes,
                );
            }
            open_artifact(state, &artifact_id, project, highlighter);
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    state: &'a State,
    project: &'a ProjectData,
) -> Element<'a, Message> {
    let list = view_list(state, project);
    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let is_exploration = state.is_exploration_selected();
    let ix = state.active_interaction();

    // Exploration mode: no content column or toggle, interaction fills remaining width.
    if is_exploration {
        let mut main_row = row![
            container(list)
                .width(theme::LIST_COLUMN_WIDTH)
                .height(Length::Fill)
                .style(theme::surface),
            divider,
        ];

        if let Some(ix) = ix {
            let interaction_col = interaction::view_column(ix, Message::Interaction);
            main_row = main_row.push(
                container(interaction_col)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::surface),
            );
        }

        return main_row.height(Length::Fill).into();
    }

    // Normal mode: content column + optional interaction panel.
    let content = if state.audit_active {
        view_audit(project)
    } else {
        view_content(state, project)
    };
    let visible = ix.is_some_and(|i| i.visible);
    let width = ix.map_or(theme::INTERACTION_COLUMN_WIDTH, |i| i.width);

    let toggle =
        interaction_toggle::view(visible, width, |m| Message::Interaction(interaction::Msg::Handle(m)));

    let mut main_row = row![
        container(list)
            .width(theme::LIST_COLUMN_WIDTH)
            .height(Length::Fill)
            .style(theme::surface),
        divider,
        container(content)
            .width(Length::Fill)
            .height(Length::Fill),
        toggle,
    ];

    if let Some(ix) = ix
        && ix.visible {
            let interaction_col = interaction::view_column(ix, Message::Interaction);

            main_row = main_row.push(
                container(interaction_col)
                    .width(ix.width)
                    .height(Length::Fill)
                    .style(theme::surface),
            );
        }

    main_row.height(Length::Fill).into()
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let mut selector = column![].spacing(theme::SPACING_XS);

    // Exploration changes (virtual) — listed first.
    for name in &state.explorations {
        let is_selected = state.selected_change.as_deref() == Some(name);
        let style = if is_selected {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::list_item
        };
        let icon = svg(svg::Handle::from_memory(ICON_EXPLORE))
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(theme::svg_tint(theme::text_muted()));
        let close_btn = button(text("\u{00d7}").size(theme::font_md()))
            .on_press(Message::RemoveExploration(name.clone()))
            .padding(0.0)
            .style(theme::icon_button);
        let label = row![
            icon,
            text(name).size(theme::font_md()).wrapping(Wrapping::None),
            Space::new().width(Length::Fill),
            close_btn,
        ]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Center);
        selector = selector.push(
            button(label)
                .on_press(Message::SelectChange(name.clone()))
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style),
        );
    }

    // Real changes from duckspec.
    let all_changes: Vec<_> = project
        .active_changes
        .iter()
        .chain(project.archived_changes.iter())
        .collect();

    for ch in &all_changes {
        let is_selected = state
            .selected_change
            .as_ref() == Some(&ch.name);
        let style = if is_selected {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::list_item
        };
        let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(theme::svg_tint(theme::text_muted()));
        let mut label = row![icon, text(&ch.name).size(theme::font_md()).wrapping(Wrapping::None)]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);
        if let Some(v) = project.validations.get(&ch.name) {
            let count = v.total_count();
            if count > 0 {
                label = label.push(Space::new().width(Length::Fill));
                label = label.push(
                    text(count.to_string())
                        .size(theme::font_sm())
                        .color(theme::error()),
                );
            }
        }
        selector = selector.push(
            button(label)
                .on_press(Message::SelectChange(ch.name.clone()))
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style),
        );
    }

    let change_section = {
        let arrow = if state.expanded_sections.contains("picker") { "\u{25bf}" } else { "\u{25b9}" };
        let header = button(
            row![
                text("CHANGE")
                    .size(theme::font_sm())
                    .color(theme::text_secondary()),
                Space::new().width(Length::Fill),
                text(arrow).size(theme::font_sm()).color(theme::text_muted()),
            ]
            .width(Length::Fill),
        )
        .on_press(Message::ToggleSection("picker".to_string()))
        .width(Length::Fill)
        .style(theme::section_header)
        .padding([theme::SPACING_SM, theme::SPACING_SM]);

        let mut col = column![
            row![
                container(header).width(Length::Fill),
                button(text("+").size(theme::font_sm()).color(theme::text_secondary()))
                    .on_press(Message::AddExploration)
                    .padding([theme::SPACING_SM, theme::SPACING_SM])
                    .style(theme::section_header),
            ]
        ].spacing(0.0);

        if state.expanded_sections.contains("picker") {
            col = col.push(selector);
        }
        col
    };

    // Audit header — always visible.
    let audit_section = {
        let total_errors: usize = project.validations.values().map(|v| v.total_count()).sum();
        let title_color = if state.audit_active {
            theme::accent()
        } else {
            theme::text_secondary()
        };
        let style = if state.audit_active {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::section_header
        };
        let mut label = row![
            text("AUDIT")
                .size(theme::font_sm())
                .color(title_color),
        ]
        .width(Length::Fill);
        if total_errors > 0 {
            label = label.push(Space::new().width(Length::Fill));
            label = label.push(
                text(total_errors.to_string())
                    .size(theme::font_sm())
                    .color(theme::error()),
            );
        }
        button(label)
            .on_press(Message::ShowAudit)
            .width(Length::Fill)
            .style(style)
            .padding([theme::SPACING_SM, theme::SPACING_SM])
    };

    let change = find_change(state, project);
    let is_exploration = state.is_exploration_selected();
    let mut list_col = column![audit_section, change_section].spacing(0.0);

    if is_exploration {
        // Explorations only show the interaction column, no overview/caps/steps.
        list_col = list_col.push(
            container(
                text("Exploration mode — use the agent or terminal to work freely.")
                    .size(theme::font_md())
                    .color(theme::text_muted()),
            )
            .padding([theme::SPACING_SM, theme::SPACING_SM]),
        );
    } else if !state.audit_active {
        if let Some(change) = change {
            let error_ids: HashSet<String> = project
                .validations
                .get(&change.name)
                .map(|v| v.file_errors.iter().map(|(p, _)| p.clone()).collect())
                .unwrap_or_default();
            list_col = list_col.push(view_overview_section(state, change, &error_ids));
            list_col = list_col.push(view_caps_section(state, change, &error_ids));
            list_col = list_col.push(view_steps_section(state, change, &error_ids));
        }
    }

    if !state.audit_active && change.is_none() && !is_exploration {
        list_col = list_col.push(
            container(
                text("Select a change")
                    .size(theme::font_md())
                    .color(theme::text_muted()),
            )
            .padding([theme::SPACING_SM, theme::SPACING_SM]),
        );
    }

    // Changed files section (always visible, independent of selected change).
    list_col = list_col.push(view_changed_files_section(state));

    scrollable(list_col)
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .height(Length::Fill)
        .into()
}

fn view_overview_section<'a>(
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if change.has_proposal {
        let id = format!("{}/proposal.md", change.prefix);
        let has_err = error_ids.contains(&id);
        items = items.push(file_item("proposal.md", &id, has_err, state));
    }
    if change.has_design {
        let id = format!("{}/design.md", change.prefix);
        let has_err = error_ids.contains(&id);
        items = items.push(file_item("design.md", &id, has_err, state));
    }
    if !change.has_proposal && !change.has_design {
        items = items.push(
            container(text("No overview files").size(theme::font_md()).color(theme::text_muted()))
                .padding([2.0, theme::SPACING_SM]),
        );
    }

    collapsible::view(
        "Overview",
        state.expanded_sections.contains("overview"),
        Message::ToggleSection("overview".to_string()),
        items.into(),
    )
}

fn view_caps_section<'a>(
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let content = if change.cap_tree.is_empty() {
        container(text("No capability changes").size(theme::font_md()).color(theme::text_muted()))
            .padding([2.0, theme::SPACING_SM])
            .into()
    } else {
        tree_view::view(
            &change.cap_tree,
            &state.expanded_nodes,
            state.tabs.active_tab().map(|t| t.id.as_str()),
            error_ids,
            Message::ToggleNode,
            Message::SelectItem,
        )
    };

    collapsible::view(
        "Capabilities",
        state.expanded_sections.contains("capabilities"),
        Message::ToggleSection("capabilities".to_string()),
        content,
    )
}

fn view_steps_section<'a>(
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if change.steps.is_empty() {
        items = items.push(
            container(text("No steps").size(theme::font_md()).color(theme::text_muted()))
                .padding([2.0, theme::SPACING_SM]),
        );
    } else {
        for step in &change.steps {
            let is_active = state.tabs.active_tab().is_some_and(|t| t.id == step.id);
            let has_error = error_ids.contains(&step.id);
            let style = if is_active {
                theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };
            let icon_bytes: &[u8] = match step.completion {
                StepCompletion::Done => ICON_STEP_DONE,
                StepCompletion::Partial(_, _) => ICON_STEP_PARTIAL,
                StepCompletion::NoTasks => ICON_STEP,
            };
            let icon = svg(svg::Handle::from_memory(icon_bytes))
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .style(theme::svg_tint(theme::text_muted()));
            let mut label = row![
                icon,
                text(format!("{:02}-{}", step.number, step.label)).size(theme::font_md()).wrapping(Wrapping::None),
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);
            if has_error {
                label = label.push(Space::new().width(Length::Fill));
                label = label.push(
                    text("\u{2022}").size(theme::font_md()).color(theme::error()),
                );
            }
            items = items.push(
                button(label)
                    .on_press(Message::SelectItem(step.id.clone()))
                    .width(Length::Fill)
                    .padding([2.0, theme::SPACING_SM])
                    .style(style),
            );
        }
    }

    collapsible::view(
        "Steps",
        state.expanded_sections.contains("steps"),
        Message::ToggleSection("steps".to_string()),
        items.into(),
    )
}

fn view_changed_files_section<'a>(state: &'a State) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if state.changed_files.is_empty() {
        items = items.push(
            container(text("No changes").size(theme::font_md()).color(theme::text_muted()))
                .padding([2.0, theme::SPACING_SM]),
        );
    } else {
        for cf in &state.changed_files {
            let status_char = match cf.status {
                FileStatus::Modified => "M",
                FileStatus::Added => "A",
                FileStatus::Deleted => "D",
            };
            let color = theme::vcs_status_color(&cf.status);
            let tab_id = format!("vcs:{}", cf.path.display());
            let is_active = state.tabs.active_tab().is_some_and(|t| t.id == tab_id);
            let style = if is_active {
                theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };

            let label = row![
                text(status_char)
                    .size(theme::font_md())
                    .font(theme::content_font())
                    .color(color),
                text(cf.path.display().to_string()).size(theme::font_md()).wrapping(Wrapping::None),
            ]
            .spacing(theme::SPACING_SM);

            items = items.push(
                button(label)
                    .on_press(Message::SelectChangedFile(cf.path.clone()))
                    .width(Length::Fill)
                    .padding([2.0, theme::SPACING_SM])
                    .style(style),
            );
        }
    }

    collapsible::view(
        "Changed Files",
        state.expanded_sections.contains("changed_files"),
        Message::ToggleSection("changed_files".to_string()),
        items.into(),
    )
}

fn view_audit<'a>(project: &'a ProjectData) -> Element<'a, Message> {
    let total_errors: usize = project.validations.values().map(|v| v.total_count()).sum();

    // ── Header ─────────────────────────────────────────────────────────
    let summary = if total_errors == 0 {
        text("All checks passed")
            .size(theme::font_md())
            .color(theme::success())
    } else {
        text(format!(
            "{} error{}",
            total_errors,
            if total_errors == 1 { "" } else { "s" }
        ))
        .size(theme::font_md())
        .color(theme::error())
    };

    let header = container(
        row![
            column![
                text("Audit").size(22.0).color(theme::text_primary()),
                summary,
            ]
            .spacing(theme::SPACING_XS),
            Space::new().width(Length::Fill),
            button(
                text("Refresh")
                    .size(theme::font_md())
                    .color(theme::accent()),
            )
            .on_press(Message::RefreshAudit)
            .padding([theme::SPACING_SM, theme::SPACING_LG])
            .style(theme::dashboard_action),
        ]
        .align_y(iced::Center)
        .width(Length::Fill),
    )
    .padding([theme::SPACING_LG, theme::SPACING_XL]);

    let mut content = column![header].spacing(theme::SPACING_MD);

    if total_errors > 0 {
        let mut changes: Vec<_> = project.validations.iter().collect();
        changes.sort_by_key(|(name, _)| name.as_str());

        for (change_name, validation) in changes {
            content = content.push(view_audit_change(change_name, validation));
        }
    }

    scrollable(
        container(content)
            .padding([0.0, theme::SPACING_XL])
            .width(Length::Fill),
    )
    .direction(theme::thin_scrollbar_direction())
    .style(theme::thin_scrollbar)
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

/// Render a single change's errors as a card.
fn view_audit_change<'a>(
    change_name: &'a str,
    validation: &'a crate::data::ChangeValidation,
) -> Element<'a, Message> {
    let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
        .width(16.0)
        .height(16.0)
        .style(theme::svg_tint(theme::text_muted()));
    let change_header = row![
        icon,
        text(change_name).size(15.0).color(theme::text_primary()),
        Space::new().width(Length::Fill),
        text(format!(
            "{} error{}",
            validation.total_count(),
            if validation.total_count() == 1 { "" } else { "s" }
        ))
        .size(theme::font_sm())
        .color(theme::text_muted()),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Center);

    let mut card = column![change_header].spacing(theme::SPACING_MD);

    // Per-file error groups.
    for (path, errors) in &validation.file_errors {
        let artifact_id = path.clone();
        let change = change_name.to_string();
        let file_link = button(
            text(path.as_str())
                .size(theme::font_md())
                .color(theme::accent()),
        )
        .on_press(Message::SelectAuditError {
            change,
            artifact_id,
        })
        .padding(0.0)
        .style(theme::link_button);

        let mut error_list = column![].spacing(theme::SPACING_XS);
        for err in errors {
            error_list = error_list.push(
                text(err.as_str())
                    .size(theme::font_md())
                    .color(theme::error()),
            );
        }

        let group = column![
            file_link,
            container(error_list).padding([0.0, theme::SPACING_LG]),
        ]
        .spacing(theme::SPACING_XS);

        card = card.push(group);
    }

    // Cross-file change-level errors.
    if !validation.change_errors.is_empty() {
        let mut structural = column![
            text("Structural").size(theme::font_md()).color(theme::text_secondary()),
        ]
        .spacing(theme::SPACING_XS);

        for err in &validation.change_errors {
            structural = structural.push(
                container(
                    text(err.as_str())
                        .size(theme::font_md())
                        .color(theme::error()),
                )
                .padding([0.0, theme::SPACING_LG]),
            );
        }
        card = card.push(structural);
    }

    container(card)
        .padding(theme::SPACING_LG)
        .width(Length::Fill)
        .style(theme::audit_card)
        .into()
}

fn icon_for_artifact(label: &str) -> &'static [u8] {
    match label {
        l if l.starts_with("spec.delta") => ICON_SPEC_DELTA,
        l if l.starts_with("spec") => ICON_SPEC,
        l if l.starts_with("doc.delta") => ICON_DOC_DELTA,
        l if l.starts_with("doc") => ICON_DOC,
        _ => ICON_FILE,
    }
}

fn file_item<'a>(label: &str, id: &str, has_error: bool, state: &State) -> Element<'a, Message> {
    let is_active = state.tabs.active_tab().is_some_and(|t| t.id == id);
    let style = if is_active {
        theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        theme::list_item
    };
    let icon = svg(svg::Handle::from_memory(icon_for_artifact(label)))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(theme::svg_tint(theme::text_muted()));
    let mut content = row![icon, text(label.to_string()).size(theme::font_md()).wrapping(Wrapping::None)]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Center);
    if has_error {
        content = content.push(Space::new().width(Length::Fill));
        content = content.push(
            text("\u{2022}").size(theme::font_md()).color(theme::error()),
        );
    }
    button(content)
        .on_press(Message::SelectItem(id.to_string()))
        .width(Length::Fill)
        .padding([2.0, theme::SPACING_SM])
        .style(style)
        .into()
}

fn view_content<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let bar = tab_bar::view_bar(
        &state.tabs,
        Message::SelectTab,
        Message::CloseTab,
    );
    let body = tab_bar::view_content(&state.tabs).map(Message::TabContent);

    // Error panel for the active artifact.
    let error_panel = state
        .tabs
        .active_tab()
        .and_then(|tab| {
            let change_name = state.selected_change.as_ref()?;
            let validation = project.validations.get(change_name)?;
            let errors = validation
                .file_errors
                .iter()
                .find(|(path, _)| *path == tab.id)?;
            Some(&errors.1)
        })
        .filter(|errs| !errs.is_empty());

    let mut col = column![bar, body].height(Length::Fill);

    if let Some(errors) = error_panel {
        let divider = container(Space::new().width(Length::Fill))
            .height(1.0)
            .style(theme::divider);

        let mut error_list = column![].spacing(theme::SPACING_XS);
        for err in errors {
            error_list = error_list.push(
                text(err.as_str())
                    .size(theme::font_md())
                    .color(theme::error()),
            );
        }

        let panel = container(
            column![
                text("Errors")
                    .size(theme::font_sm())
                    .color(theme::text_secondary()),
                error_list,
            ]
            .spacing(theme::SPACING_SM),
        )
        .padding(theme::SPACING_SM)
        .width(Length::Fill)
        .style(theme::surface);

        col = col.push(divider);
        col = col.push(panel);
    }

    col.into()
}

fn find_change<'a>(state: &State, project: &'a ProjectData) -> Option<&'a ChangeData> {
    let name = state.selected_change.as_ref()?;
    project
        .active_changes
        .iter()
        .chain(project.archived_changes.iter())
        .find(|c| &c.name == name)
}

fn open_artifact(
    state: &mut State,
    id: &str,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    if let Some(content) = project.read_artifact(id) {
        let title = id.rsplit('/').next().unwrap_or(id).to_string();
        crate::open_artifact_tab(&mut state.tabs, id.to_string(), title, content, id, highlighter);
    }
}
