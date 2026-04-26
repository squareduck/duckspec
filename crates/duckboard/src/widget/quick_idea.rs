//! Quick Idea overlay (cmd-i) — capture a new idea or jump into an existing
//! one without leaving the keyboard. Single-line input acts as a substring
//! query over title + body + tags; multi-line commits to creating/editing,
//! revealing a tag toolbar instead of the matches list.

use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::text::Span;
use iced::widget::{
    Space, button, column, container, rich_text, row, scrollable, span, text, text_input,
};
use iced::{Border, Center, Color, Element, Font, Length};

use crate::area::interaction;
use crate::highlight::SyntaxHighlighter;
use crate::idea_store::{self, Idea};
use crate::theme;
use crate::widget::text_edit::{self, EditorAction, EditorState};

pub const INPUT_ID: &str = "quick-idea-editor";
pub const TAG_INPUT_ID: &str = "quick-idea-tag-input";

const MAX_VISIBLE: usize = 8;
const PREVIEW_TRIM: usize = 200;

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    Open,
    Close,
    EditorAction(EditorAction),
    /// Plain Enter on the editor. Loads the highlighted match if one is
    /// selected; otherwise the host saves the current buffer + tags.
    Submit,
    SelectNext,
    SelectPrev,

    OpenTagInput,
    CancelTagInput,
    TagInputChanged(String),
    SubmitTagInput,
    RemoveTag(usize),
    /// Chip body click; main.rs reads modifier state and dispatches into
    /// `promote_tag` (shift held) or `edit_tag` directly on this state.
    ChipClick(usize),
}

// ── Corpus & matches ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CorpusEntry {
    path: PathBuf,
    title: String,
    tags: Vec<String>,
    body: String,
    body_lines: Vec<String>,
    second_nonblank: String,
}

#[derive(Debug, Clone)]
pub struct Match {
    pub path: PathBuf,
    pub title: String,
    pub tags: Vec<String>,
    pub preview: String,
    /// Byte range within `title` covered by the match. None when title was
    /// not the source of the hit.
    pub title_hit: Option<(usize, usize)>,
    /// Index into `tags` whose value contains the query, if any.
    pub tag_hit: Option<usize>,
    /// Byte range within `preview` covered by the match. None when the
    /// preview is the second-non-empty fallback.
    pub preview_hit: Option<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct LoadedIdea {
    pub path: PathBuf,
    pub title: String,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct QuickIdeaState {
    pub visible: bool,
    pub editor: EditorState,
    pub matches: Vec<Match>,
    pub selected: Option<usize>,
    pub loaded: Option<LoadedIdea>,
    pub tags: Vec<String>,
    pub tag_input: Option<String>,
    pub tag_input_editing: Option<usize>,
    /// Snapshot of all ideas with bodies, taken at open time. Substring
    /// search runs over this in-memory corpus; live disk edits while the
    /// modal is up are not reflected, but the modal is short-lived so the
    /// tradeoff is fine.
    corpus: Vec<CorpusEntry>,
}

impl Default for QuickIdeaState {
    fn default() -> Self {
        Self {
            visible: false,
            editor: EditorState::new(""),
            matches: Vec::new(),
            selected: None,
            loaded: None,
            tags: Vec::new(),
            tag_input: None,
            tag_input_editing: None,
            corpus: Vec::new(),
        }
    }
}

impl std::fmt::Debug for QuickIdeaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuickIdeaState")
            .field("visible", &self.visible)
            .field("matches", &self.matches.len())
            .field("selected", &self.selected)
            .field("loaded", &self.loaded.as_ref().map(|l| &l.path))
            .field("tags", &self.tags)
            .finish()
    }
}

impl QuickIdeaState {
    pub fn open(&mut self, ideas: &[Idea], highlighter: &SyntaxHighlighter) {
        self.visible = true;
        self.matches.clear();
        self.selected = None;
        self.loaded = None;
        self.tags.clear();
        self.tag_input = None;
        self.tag_input_editing = None;
        self.editor = EditorState::new("");
        let syntax = highlighter.find_syntax("md");
        self.editor.highlight_spans = Some(highlighter.highlight_lines(&self.editor.lines, syntax));

        self.corpus = ideas.iter().filter_map(build_corpus_entry).collect();
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.matches.clear();
        self.selected = None;
        self.loaded = None;
        self.tags.clear();
        self.tag_input = None;
        self.tag_input_editing = None;
        self.editor = EditorState::new("");
        self.corpus.clear();
    }

    pub fn is_single_line(&self) -> bool {
        self.editor.lines.len() <= 1
    }

    /// True when the matches list should be visible — single-line input and
    /// no idea is currently loaded into the editor.
    pub fn search_active(&self) -> bool {
        self.is_single_line() && self.loaded.is_none()
    }

    /// Apply an editor action and (when search is active) refresh the match
    /// list against the current line-0 contents. Selection is cleared on any
    /// edit so navigation never silently moves under typing.
    pub fn apply_editor_action(&mut self, action: EditorAction, highlighter: &SyntaxHighlighter) {
        let mutated = self.editor.apply_action(action);
        if mutated {
            let syntax = highlighter.find_syntax("md");
            self.editor.highlight_spans =
                Some(highlighter.highlight_lines(&self.editor.lines, syntax));
        }
        self.recompute_search();
    }

    fn recompute_search(&mut self) {
        self.selected = None;
        if !self.search_active() {
            self.matches.clear();
            return;
        }
        let q = self
            .editor
            .lines
            .first()
            .cloned()
            .unwrap_or_default();
        if q.trim().is_empty() {
            self.matches.clear();
            return;
        }
        self.matches = run_search(&self.corpus, &q);
    }

    pub fn select_next(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(i) => (i + 1).min(self.matches.len() - 1),
        });
    }

    pub fn select_prev(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(0) => 0,
            Some(i) => i - 1,
        });
    }

    /// Load the currently-highlighted match into the editor. Body is replaced
    /// verbatim (no extra newline), tags are populated from the match. After
    /// this, search freezes — `search_active()` returns false because
    /// `loaded` is `Some`.
    pub fn load_selected(&mut self, highlighter: &SyntaxHighlighter) {
        let Some(idx) = self.selected else { return };
        let Some(m) = self.matches.get(idx).cloned() else {
            return;
        };
        let Some(entry) = self.corpus.iter().find(|e| e.path == m.path).cloned() else {
            return;
        };
        self.editor = EditorState::new(&entry.body);
        let syntax = highlighter.find_syntax("md");
        self.editor.highlight_spans = Some(highlighter.highlight_lines(&self.editor.lines, syntax));
        self.tags = entry.tags.clone();
        self.loaded = Some(LoadedIdea {
            path: entry.path.clone(),
            title: entry.title.clone(),
        });
        self.matches.clear();
        self.selected = None;
        self.tag_input = None;
        self.tag_input_editing = None;
    }

    /// Current editor body as a single string.
    pub fn body(&self) -> String {
        self.editor.lines.join("\n")
    }

    pub fn header_text(&self) -> String {
        match self.loaded.as_ref() {
            Some(l) => format!("Editing: {}", l.title),
            None => "New idea".to_string(),
        }
    }

    // ── Tag mutations ───────────────────────────────────────────────────

    pub fn open_tag_input(&mut self) {
        self.tag_input = Some(String::new());
        self.tag_input_editing = None;
    }

    pub fn cancel_tag_input(&mut self) {
        self.tag_input = None;
        self.tag_input_editing = None;
    }

    pub fn set_tag_input(&mut self, value: String) {
        if self.tag_input.is_some() {
            self.tag_input = Some(value);
        }
    }

    pub fn submit_tag_input(&mut self) {
        let raw = self.tag_input.take().unwrap_or_default();
        let editing = self.tag_input_editing.take();
        let cleaned: String = raw.trim().trim_start_matches('#').trim().to_string();
        match editing {
            Some(idx) => {
                if idx >= self.tags.len() {
                    return;
                }
                if cleaned.is_empty() {
                    self.tags.remove(idx);
                    return;
                }
                self.tags[idx] = cleaned;
                let mut seen = HashSet::new();
                self.tags.retain(|t| seen.insert(t.clone()));
            }
            None => {
                if cleaned.is_empty() {
                    return;
                }
                if !self.tags.iter().any(|t| t == &cleaned) {
                    self.tags.push(cleaned);
                }
            }
        }
    }

    pub fn remove_tag(&mut self, idx: usize) {
        if idx < self.tags.len() {
            self.tags.remove(idx);
        }
    }

    pub fn promote_tag(&mut self, idx: usize) {
        if idx > 0 && idx < self.tags.len() {
            let t = self.tags.remove(idx);
            self.tags.insert(0, t);
        }
    }

    pub fn edit_tag(&mut self, idx: usize) {
        if let Some(t) = self.tags.get(idx) {
            self.tag_input = Some(t.clone());
            self.tag_input_editing = Some(idx);
        }
    }
}

// ── Corpus building & search ────────────────────────────────────────────────

fn build_corpus_entry(idea: &Idea) -> Option<CorpusEntry> {
    let body = idea_store::read_body(&idea.abs_path).ok()?;
    let body_lines: Vec<String> = body.lines().map(String::from).collect();
    let second_nonblank = body_lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    Some(CorpusEntry {
        path: idea.abs_path.clone(),
        title: idea.display_title(),
        tags: idea.frontmatter.tags.clone(),
        body,
        body_lines,
        second_nonblank,
    })
}

fn find_ci(haystack: &str, needle_lc: &str) -> Option<(usize, usize)> {
    if needle_lc.is_empty() {
        return None;
    }
    let hay_lc = haystack.to_lowercase();
    let start = hay_lc.find(needle_lc)?;
    Some((start, start + needle_lc.len()))
}

fn run_search(corpus: &[CorpusEntry], query: &str) -> Vec<Match> {
    let q_lc = query.trim().to_lowercase();
    if q_lc.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<Match> = Vec::new();
    for entry in corpus {
        let title_hit = find_ci(&entry.title, &q_lc);
        let tag_hit = entry
            .tags
            .iter()
            .position(|t| t.to_lowercase().contains(&q_lc));
        let body_line = entry
            .body_lines
            .iter()
            .find(|l| l.to_lowercase().contains(&q_lc));

        if title_hit.is_none() && tag_hit.is_none() && body_line.is_none() {
            continue;
        }

        let (preview, preview_hit) = if let Some(line) = body_line {
            let trimmed = trim_preview(line);
            let hit = find_ci(&trimmed, &q_lc);
            (trimmed, hit)
        } else {
            (trim_preview(&entry.second_nonblank), None)
        };

        hits.push(Match {
            path: entry.path.clone(),
            title: entry.title.clone(),
            tags: entry.tags.clone(),
            preview,
            title_hit,
            tag_hit,
            preview_hit,
        });
    }
    hits.truncate(MAX_VISIBLE);
    hits
}

fn trim_preview(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.chars().count() <= PREVIEW_TRIM {
        trimmed.to_string()
    } else {
        let cut: String = trimmed.chars().take(PREVIEW_TRIM).collect();
        format!("{cut}…")
    }
}

// ── View ────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a QuickIdeaState) -> Element<'a, Msg> {
    let header = container(
        text(state.header_text())
            .size(theme::font_md())
            .color(theme::text_primary()),
    )
    .padding([theme::SPACING_SM, theme::SPACING_MD])
    .width(Length::Fill)
    .style(header_style);

    let editor = text_edit::TextEdit::new(&state.editor, Msg::EditorAction)
        .id(INPUT_ID)
        .show_gutter(false)
        .word_wrap(true)
        .fit_content(true)
        .transparent_bg(true)
        .placeholder("Type to search ideas. Shift+Enter for newline.")
        .on_submit(Msg::Submit);
    let editor_box = container(editor)
        .padding([theme::SPACING_XS, theme::SPACING_SM])
        .width(Length::Fill);

    let divider = container(Space::new().height(1.0).width(Length::Fill)).style(divider_style);

    let body: Element<'a, Msg> = if state.is_single_line() {
        if state.loaded.is_some() {
            // Search frozen — show a faint hint rather than the matches list.
            container(
                text("Editing existing idea — Esc to start over.")
                    .size(theme::font_sm())
                    .color(theme::text_muted()),
            )
            .padding([theme::SPACING_SM, theme::SPACING_MD])
            .width(Length::Fill)
            .into()
        } else {
            view_matches(state)
        }
    } else {
        view_tag_toolbar(state)
    };

    // Show a divider between the editor and the body section only when the
    // body has visible content, so a bare modal with no matches doesn't show
    // a stray rule.
    let editor_divider: Element<'a, Msg> =
        container(Space::new().height(1.0).width(Length::Fill))
            .style(divider_style)
            .into();

    let panel = container(
        column![header, divider, editor_box, editor_divider, body]
            .spacing(0.0)
            .max_width(640.0),
    )
    .padding(1)
    .style(panel_style)
    .max_width(640.0);

    container(column![Space::new().height(80.0), panel].align_x(Center))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .style(backdrop_style)
        .into()
}

fn view_matches<'a>(state: &'a QuickIdeaState) -> Element<'a, Msg> {
    if state.matches.is_empty() {
        let hint_text = if state.editor.lines.first().is_some_and(|l| l.trim().is_empty()) {
            "Type to search. Enter saves a new idea."
        } else {
            "No matches. Press Enter to save as a new idea."
        };
        return container(
            text(hint_text)
                .size(theme::font_sm())
                .color(theme::text_muted()),
        )
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .into();
    }
    let mut list = column![].spacing(0.0);
    for (i, m) in state.matches.iter().enumerate() {
        let is_selected = state.selected == Some(i);
        if i > 0 {
            list = list.push(
                container(Space::new().height(1.0).width(Length::Fill)).style(row_divider_style),
            );
        }
        list = list.push(view_match_row(m, is_selected));
    }
    scrollable(list)
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .height(Length::Shrink)
        .into()
}

fn view_match_row<'a>(m: &'a Match, is_selected: bool) -> Element<'a, Msg> {
    let title_color = if is_selected {
        theme::text_primary()
    } else {
        theme::text_secondary()
    };
    let title_spans = highlight_spans(&m.title, m.title_hit, title_color, theme::accent());
    let title_el: Element<'a, Msg> = rich_text(title_spans)
        .size(theme::font_md())
        .font(theme::content_font())
        .on_link_click(|_: ()| unreachable!("quick idea match has no links"))
        .into();

    let mut tag_row = row![].spacing(theme::SPACING_XS).align_y(Center);
    for (i, t) in m.tags.iter().enumerate() {
        let is_match = m.tag_hit == Some(i);
        let color = if is_match {
            theme::accent()
        } else {
            theme::text_muted()
        };
        tag_row = tag_row.push(
            text(format!("#{t}"))
                .size(theme::font_sm())
                .font(theme::content_font())
                .color(color),
        );
    }

    let preview_color = if is_selected {
        theme::text_secondary()
    } else {
        theme::text_muted()
    };
    let preview_spans = highlight_spans(&m.preview, m.preview_hit, preview_color, theme::accent());
    let preview_el: Element<'a, Msg> = rich_text(preview_spans)
        .size(theme::font_sm())
        .font(theme::content_font())
        .on_link_click(|_: ()| unreachable!("quick idea preview has no links"))
        .into();

    let row_content = column![
        row![title_el, Space::new().width(Length::Fill), tag_row].align_y(Center),
        preview_el,
    ]
    .spacing(2.0);

    let style: fn(&iced::Theme) -> container::Style = if is_selected {
        match_row_selected_style
    } else {
        match_row_style
    };
    container(row_content)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(style)
        .into()
}

fn view_tag_toolbar<'a>(state: &'a QuickIdeaState) -> Element<'a, Msg> {
    let mut tag_row = row![].spacing(theme::SPACING_XS).align_y(Center);
    let editing_idx = state.tag_input_editing;
    for (i, tag) in state.tags.iter().enumerate() {
        if editing_idx == Some(i)
            && let Some(value) = state.tag_input.as_ref()
        {
            tag_row = tag_row.push(view_tag_input(value));
        } else {
            tag_row = tag_row.push(view_tag_chip(i, tag, i == 0));
        }
    }
    if state.tag_input.is_some() && editing_idx.is_none() {
        tag_row = tag_row.push(view_tag_input(
            state.tag_input.as_deref().unwrap_or_default(),
        ));
    } else if state.tag_input.is_none() {
        tag_row = tag_row.push(
            button(text("+ Tag").size(theme::font_sm()))
                .on_press(Msg::OpenTagInput)
                .padding([2.0, theme::SPACING_SM])
                .style(theme::session_bar_button),
        );
    }

    let hint = text("Enter saves. Shift+Enter adds a newline.")
        .size(theme::font_sm())
        .color(theme::text_muted());

    container(
        row![tag_row, Space::new().width(Length::Fill), hint]
            .spacing(theme::SPACING_MD)
            .align_y(Center),
    )
    .padding([theme::SPACING_XS, theme::SPACING_MD])
    .width(Length::Fill)
    .into()
}

const TAG_INPUT_PLACEHOLDER: &str = "tag (or parent/child)";
const CARET_HEADROOM: f32 = 5.0;

fn view_tag_input<'a>(value: &str) -> Element<'a, Msg> {
    let measured = if value.is_empty() {
        interaction::measure_text_advanced(TAG_INPUT_PLACEHOLDER, theme::font_md())
    } else {
        interaction::measure_text_advanced(value, theme::font_md())
    }
    .ceil();
    let visual_pad = theme::SPACING_SM;
    let padding = iced::Padding {
        top: 1.0,
        right: visual_pad,
        bottom: 1.0,
        left: visual_pad + CARET_HEADROOM,
    };
    let width = measured + visual_pad * 2.0 + CARET_HEADROOM * 2.0;
    text_input(TAG_INPUT_PLACEHOLDER, value)
        .id(TAG_INPUT_ID)
        .on_input(Msg::TagInputChanged)
        .on_submit(Msg::SubmitTagInput)
        .size(theme::font_md())
        .padding(padding)
        .width(width)
        .style(tag_input_style)
        .into()
}

fn view_tag_chip<'a>(idx: usize, tag: &'a str, is_primary: bool) -> Element<'a, Msg> {
    let label_style: fn(&iced::Theme, button::Status) -> button::Style = if is_primary {
        chip_label_primary_style
    } else {
        chip_label_style
    };
    let display_tag = tag.replace('/', " / ");
    let label_btn = button(text(format!("# {display_tag}")).size(theme::font_md()))
        .on_press(Msg::ChipClick(idx))
        .padding([1.0, theme::SPACING_SM])
        .style(label_style);
    let remove_btn = button(text("×").size(theme::font_md()))
        .on_press(Msg::RemoveTag(idx))
        .padding([1.0, theme::SPACING_XS])
        .style(chip_remove_style);
    container(row![label_btn, remove_btn].align_y(Center))
        .style(chip_style)
        .into()
}

// ── Highlight helpers ───────────────────────────────────────────────────────

fn highlight_spans<'a>(
    s: &str,
    hit: Option<(usize, usize)>,
    base: Color,
    match_color: Color,
) -> Vec<Span<'a, (), Font>> {
    let mut spans: Vec<Span<'a, (), Font>> = Vec::new();
    let Some((start, end)) = hit else {
        spans.push(span(s.to_string()).color(base));
        return spans;
    };
    let start = start.min(s.len());
    let end = end.min(s.len());
    if start > 0 {
        spans.push(span(s[..start].to_string()).color(base));
    }
    if end > start {
        spans.push(span(s[start..end].to_string()).color(match_color));
    }
    if end < s.len() {
        spans.push(span(s[end..].to_string()).color(base));
    }
    spans
}

// ── Styles ──────────────────────────────────────────────────────────────────

fn backdrop_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(
            iced::Color {
                a: 0.5,
                ..theme::bg_base()
            }
            .into(),
        ),
        ..Default::default()
    }
}

fn panel_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_surface().into()),
        border: Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn header_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_section_header().into()),
        ..Default::default()
    }
}

fn divider_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::border_color().into()),
        ..Default::default()
    }
}

/// Lighter rule between match rows — uses the section-header background tone
/// so neighbouring rows read as separate cards without competing with the
/// panel's outer border.
fn row_divider_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::border_color().scale_alpha(0.5).into()),
        ..Default::default()
    }
}

fn match_row_style(_theme: &iced::Theme) -> container::Style {
    container::Style::default()
}

fn match_row_selected_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::accent_dim().scale_alpha(0.2).into()),
        ..Default::default()
    }
}

fn tag_input_style(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    use iced::widget::text_input::Status;
    let border_color = match status {
        Status::Focused { .. } => theme::accent(),
        _ => theme::border_color(),
    };
    iced::widget::text_input::Style {
        background: theme::bg_section_header().into(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: theme::BORDER_RADIUS.into(),
        },
        icon: theme::text_muted(),
        placeholder: theme::text_muted(),
        value: theme::text_primary(),
        selection: theme::bg_list_selected(),
    }
}

fn chip_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_section_header().into()),
        border: Border {
            color: theme::border_color(),
            width: 1.0,
            radius: theme::BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

fn chip_label_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: match status {
            button::Status::Hovered => theme::accent(),
            _ => theme::text_primary(),
        },
        border: Border::default(),
        ..Default::default()
    }
}

fn chip_label_primary_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: match status {
            button::Status::Hovered => theme::text_primary(),
            _ => theme::accent(),
        },
        border: Border::default(),
        ..Default::default()
    }
}

fn chip_remove_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: match status {
            button::Status::Hovered => theme::error(),
            _ => theme::text_muted(),
        },
        border: Border::default(),
        ..Default::default()
    }
}

// ── Save flow ──────────────────────────────────────────────────────────────

/// Package the modal's buffer + tags into the shape the host needs to call
/// `idea_store::save_idea`. The host owns `state.ideas` so it performs the
/// actual mutation; this is purely a snapshot of editor state.
pub fn build_save_payload(state: &QuickIdeaState) -> SavePayload {
    SavePayload {
        loaded_path: state.loaded.as_ref().map(|l| l.path.clone()),
        body: state.body(),
        tags: state.tags.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct SavePayload {
    pub loaded_path: Option<PathBuf>,
    pub body: String,
    pub tags: Vec<String>,
}

/// True if `body` is meaningful enough to persist. Avoids creating empty
/// `idea.md` files when the user opens the modal and immediately hits Enter.
pub fn body_is_savable(body: &str) -> bool {
    !body.trim().is_empty()
}
