//! Change area — single change workspace with three-column layout.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

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
}

impl Default for State {
    fn default() -> Self {
        let mut sections = HashSet::new();
        sections.insert("overview".to_string());
        sections.insert("capabilities".to_string());
        sections.insert("steps".to_string());
        sections.insert("changed_files".to_string());
        Self {
            selected_change: None,
            expanded_sections: sections,
            expanded_nodes: HashSet::new(),
            tabs: tab_bar::TabState::default(),
            changed_files: vec![],
            interactions: HashMap::new(),
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
            // Expand all tree nodes for the newly selected change.
            state.expanded_nodes.clear();
            if let Some(change) = project
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
            let just_opened = interaction::update(ix, msg);

            if just_opened && ix.mode == InteractionMode::Terminal {
                interaction::spawn_terminal(ix);
            }
            if is_mode_switch && ix.mode == InteractionMode::Terminal {
                interaction::spawn_terminal(ix);
            }

            let wants_agent = (just_opened && ix.mode == InteractionMode::AgentChat)
                || (is_mode_switch && ix.mode == InteractionMode::AgentChat);
            if wants_agent && ix.chat_session.is_none() {
                interaction::spawn_agent_session(ix, &session_name);
            }

            ix.terminal_focused = ix.visible && ix.mode == InteractionMode::Terminal;
        }
        Message::SelectChangedFile(path) => {
            if let Some(root) = &project.project_root {
                if let Some(diff) = vcs::file_diff(root, &path) {
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
        }
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(action)) => {
            crate::handle_editor_action(&mut state.tabs, action, highlighter);
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    state: &'a State,
    project: &'a ProjectData,
) -> Element<'a, Message> {
    let list = view_list(state, project);
    let content = view_content(state);
    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let ix = state.active_interaction();
    let visible = ix.map_or(false, |i| i.visible);
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

    if let Some(ix) = ix {
        if ix.visible {
            let interaction_col = interaction::view_column(ix, Message::Interaction);

            main_row = main_row.push(
                container(interaction_col)
                    .width(ix.width)
                    .height(Length::Fill)
                    .style(theme::surface),
            );
        }
    }

    main_row.height(Length::Fill).into()
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let mut selector = column![].spacing(theme::SPACING_XS);
    let all_changes: Vec<_> = project
        .active_changes
        .iter()
        .chain(project.archived_changes.iter())
        .collect();

    for ch in &all_changes {
        let is_selected = state
            .selected_change
            .as_ref()
            .map_or(false, |s| s == &ch.name);
        let style = if is_selected {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::list_item
        };
        let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
            .width(ICON_SIZE)
            .height(ICON_SIZE);
        let label = row![icon, text(&ch.name).size(theme::FONT_MD).wrapping(Wrapping::None)]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);
        selector = selector.push(
            button(label)
                .on_press(Message::SelectChange(ch.name.clone()))
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style),
        );
    }

    let change_picker = collapsible::view(
        "Change",
        state.expanded_sections.contains("picker"),
        Message::ToggleSection("picker".to_string()),
        selector.into(),
    );

    let change = find_change(state, project);
    let mut list_col = column![change_picker].spacing(0.0);

    if let Some(change) = change {
        list_col = list_col.push(view_overview_section(state, change));
        list_col = list_col.push(view_caps_section(state, change));
        list_col = list_col.push(view_steps_section(state, change));
    } else {
        list_col = list_col.push(
            container(
                text("Select a change")
                    .size(theme::FONT_MD)
                    .color(theme::TEXT_MUTED),
            )
            .padding(theme::SPACING_LG),
        );
    }

    // Changed files section (always visible, independent of selected change).
    list_col = list_col.push(view_changed_files_section(state));

    scrollable(list_col)
        .height(Length::Fill)
        .into()
}

fn view_overview_section<'a>(state: &'a State, change: &'a ChangeData) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if change.has_proposal {
        let id = format!("{}/proposal.md", change.prefix);
        items = items.push(file_item("proposal.md", &id, state));
    }
    if change.has_design {
        let id = format!("{}/design.md", change.prefix);
        items = items.push(file_item("design.md", &id, state));
    }
    if !change.has_proposal && !change.has_design {
        items = items.push(text("No overview files").size(theme::FONT_MD).color(theme::TEXT_MUTED));
    }

    collapsible::view(
        "Overview",
        state.expanded_sections.contains("overview"),
        Message::ToggleSection("overview".to_string()),
        items.into(),
    )
}

fn view_caps_section<'a>(state: &'a State, change: &'a ChangeData) -> Element<'a, Message> {
    let content = if change.cap_tree.is_empty() {
        column![text("No capability changes").size(theme::FONT_MD).color(theme::TEXT_MUTED)].into()
    } else {
        tree_view::view(
            &change.cap_tree,
            &state.expanded_nodes,
            state.tabs.active_tab().map(|t| t.id.as_str()),
            |id| Message::ToggleNode(id),
            |id| Message::SelectItem(id),
        )
    };

    collapsible::view(
        "Capabilities",
        state.expanded_sections.contains("capabilities"),
        Message::ToggleSection("capabilities".to_string()),
        content,
    )
}

fn view_steps_section<'a>(state: &'a State, change: &'a ChangeData) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if change.steps.is_empty() {
        items = items.push(text("No steps").size(theme::FONT_MD).color(theme::TEXT_MUTED));
    } else {
        for step in &change.steps {
            let is_active = state.tabs.active_tab().map_or(false, |t| t.id == step.id);
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
                .height(ICON_SIZE);
            let label = row![
                icon,
                text(format!("{:02}-{}", step.number, step.label)).size(theme::FONT_MD).wrapping(Wrapping::None),
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);
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
        items = items.push(text("No changes").size(theme::FONT_MD).color(theme::TEXT_MUTED));
    } else {
        for cf in &state.changed_files {
            let status_char = match cf.status {
                FileStatus::Modified => "M",
                FileStatus::Added => "A",
                FileStatus::Deleted => "D",
            };
            let color = theme::vcs_status_color(&cf.status);
            let tab_id = format!("vcs:{}", cf.path.display());
            let is_active = state.tabs.active_tab().map_or(false, |t| t.id == tab_id);
            let style = if is_active {
                theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };

            let label = row![
                text(status_char)
                    .size(theme::FONT_MD)
                    .font(iced::Font::MONOSPACE)
                    .color(color),
                text(cf.path.display().to_string()).size(theme::FONT_MD).wrapping(Wrapping::None),
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

fn icon_for_artifact(label: &str) -> &'static [u8] {
    match label {
        l if l.starts_with("spec.delta") => ICON_SPEC_DELTA,
        l if l.starts_with("spec") => ICON_SPEC,
        l if l.starts_with("doc.delta") => ICON_DOC_DELTA,
        l if l.starts_with("doc") => ICON_DOC,
        _ => ICON_FILE,
    }
}

fn file_item<'a>(label: &str, id: &str, state: &State) -> Element<'a, Message> {
    let is_active = state.tabs.active_tab().map_or(false, |t| t.id == id);
    let style = if is_active {
        theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        theme::list_item
    };
    let icon = svg(svg::Handle::from_memory(icon_for_artifact(label)))
        .width(ICON_SIZE)
        .height(ICON_SIZE);
    let content = row![icon, text(label.to_string()).size(theme::FONT_MD).wrapping(Wrapping::None)]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Center);
    button(content)
        .on_press(Message::SelectItem(id.to_string()))
        .width(Length::Fill)
        .padding([2.0, theme::SPACING_SM])
        .style(style)
        .into()
}

fn view_content<'a>(state: &'a State) -> Element<'a, Message> {
    let bar = tab_bar::view_bar(
        &state.tabs,
        |i| Message::SelectTab(i),
        |i| Message::CloseTab(i),
    );
    let body = tab_bar::view_content(&state.tabs).map(Message::TabContent);

    column![bar, body].height(Length::Fill).into()
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
