//! Change area — single change workspace with three-column layout.

use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Element, Length};

use crate::data::{ChangeData, ProjectData};
use crate::theme;
use crate::vcs::{self, ChangedFile, FileStatus};
use crate::widget::{collapsible, interaction_toggle, tab_bar, tree_view};

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct State {
    pub selected_change: Option<String>,
    pub expanded_sections: HashSet<String>,
    pub expanded_nodes: HashSet<String>,
    pub tabs: tab_bar::TabState,
    pub interaction_visible: bool,
    pub interaction_width: f32,
    pub changed_files: Vec<ChangedFile>,
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
            interaction_visible: false,
            interaction_width: theme::INTERACTION_COLUMN_WIDTH,
            changed_files: vec![],
        }
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
    TogglePin(usize),
    InteractionHandle(interaction_toggle::HandleMsg),
    TerminalScroll,
    SelectChangedFile(PathBuf),
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(state: &mut State, message: Message, project: &ProjectData) {
    match message {
        Message::SelectChange(name) => {
            state.selected_change = Some(name);
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
            if let Some(content) = project.read_artifact(&id) {
                let title = id.rsplit('/').next().unwrap_or(&id).to_string();
                state.tabs.open(id, title, content);
            }
        }
        Message::SelectTab(idx) => state.tabs.select(idx),
        Message::CloseTab(idx) => state.tabs.close(idx),
        Message::TogglePin(idx) => state.tabs.toggle_pin(idx),
        Message::InteractionHandle(msg) => match msg {
            interaction_toggle::HandleMsg::Toggle => {
                state.interaction_visible = !state.interaction_visible;
            }
            interaction_toggle::HandleMsg::SetWidth(w) => {
                state.interaction_width = w;
            }
        },
        Message::TerminalScroll => {}
        Message::SelectChangedFile(path) => {
            if let Some(root) = &project.project_root {
                if let Some(diff) = vcs::file_diff(root, &path) {
                    let id = format!("vcs:{}", path.display());
                    let title = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    state.tabs.open_diff(id, title, diff);
                }
            }
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    state: &'a State,
    project: &'a ProjectData,
    terminal: Option<&'a crate::widget::terminal::TerminalState>,
) -> Element<'a, Message> {
    let list = view_list(state, project);
    let content = view_content(state);
    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let toggle =
        interaction_toggle::view(state.interaction_visible, state.interaction_width, Message::InteractionHandle);

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

    if state.interaction_visible {
        let interaction: Element<'a, Message> = if let Some(ts) = terminal {
            crate::widget::terminal::view_terminal(ts).map(|_: ()| Message::TerminalScroll)
        } else {
            view_interaction()
        };
        main_row = main_row.push(
            container(interaction)
                .width(state.interaction_width)
                .height(Length::Fill)
                .style(theme::surface),
        );
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
        selector = selector.push(
            button(text(&ch.name).size(13))
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
                    .size(13)
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
        items = items.push(text("No overview files").size(12).color(theme::TEXT_MUTED));
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
        column![text("No capability changes").size(12).color(theme::TEXT_MUTED)].into()
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
        items = items.push(text("No steps").size(12).color(theme::TEXT_MUTED));
    } else {
        for step in &change.steps {
            items = items.push(file_item(
                &format!("{:02}. {}", step.number, step.label),
                &step.id,
                state,
            ));
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
        items = items.push(text("No changes").size(12).color(theme::TEXT_MUTED));
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
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(color),
                text(cf.path.display().to_string()).size(13),
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

fn file_item<'a>(label: &str, id: &str, state: &State) -> Element<'a, Message> {
    let is_active = state.tabs.active_tab().map_or(false, |t| t.id == id);
    let style = if is_active {
        theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        theme::list_item
    };
    button(text(label.to_string()).size(13))
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
        |i| Message::TogglePin(i),
    );
    let body = tab_bar::view_content(&state.tabs);

    column![bar, body].height(Length::Fill).into()
}

fn view_interaction<'a>() -> Element<'a, Message> {
    container(
        column![
            text("Interaction").size(14).color(theme::TEXT_SECONDARY),
            Space::new().height(theme::SPACING_MD),
            text("Terminal and chat will appear here.")
                .size(13)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(theme::SPACING_SM)
        .padding(theme::SPACING_LG),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn find_change<'a>(state: &State, project: &'a ProjectData) -> Option<&'a ChangeData> {
    let name = state.selected_change.as_ref()?;
    project
        .active_changes
        .iter()
        .chain(project.archived_changes.iter())
        .find(|c| &c.name == name)
}
