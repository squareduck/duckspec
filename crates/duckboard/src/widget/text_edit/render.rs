//! Iced widget implementation for the custom text editor.

use std::cell::RefCell;
use std::sync::Arc;

use iced::advanced::layout;
use iced::advanced::mouse as adv_mouse;
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::text::{self, Paragraph as _, Renderer as TextRenderer};
use iced::advanced::widget::{self, Id, Tree, Widget, operation};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::keyboard::key::Named;
use iced::mouse;
use iced::{
    Border, Color, Element, Event, Length, Pixels, Point, Rectangle, Size, Theme, alignment,
    keyboard, window,
};

use linkify::LinkFinder;

use super::state::{
    CONTENT_PAD_Y, EditorAction, EditorState, HighlightRange, LINE_HEIGHT, Pos, block_header_color,
    block_kind_bg, line_bg_color,
};
use crate::path_link::{self, LinkTarget};
use crate::theme;
use crate::widget::autoscroll;
use crate::widget::md_table::{self, TableLayout};
use crate::widget::terminal::current_modifiers;

// ── Layout constants ───────────────────────────────────────────────────────

fn font_size() -> f32 {
    theme::content_size()
}
const GUTTER_PAD: f32 = 8.0;
const CONTENT_PAD: f32 = 8.0;
/// Width of the overlaid scrollbars drawn by this widget when content
/// overflows. Matches `theme::thin_scrollbar_direction`'s 4px rail so the
/// file viewer's scroll chrome reads identically to the list column.
const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLBAR_RADIUS: f32 = 2.0;
/// Minimum scroller length so the indicator stays grab-able on very tall or
/// very wide content.
const SCROLLBAR_MIN_SCROLLER: f32 = 20.0;

// ── Widget internal state (in iced tree) ───────────────────────────────────

/// Inputs that affect hybrid `EditorLayout`. Cache is valid only while this
/// key matches the current call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HybridLayoutKey {
    pane_chars: usize,
    word_wrap: bool,
    /// `Arc::as_ptr` of `EditorState::lines` — changes when the buffer Arc is
    /// replaced (e.g. rebuild). In-place mutation keeps the ptr and bumps
    /// `highlight_version` instead.
    lines_ptr: usize,
    highlight_version: u64,
    line_count: usize,
}

impl HybridLayoutKey {
    fn new(
        lines: &Arc<Vec<String>>,
        highlight_version: u64,
        pane_chars: usize,
        word_wrap: bool,
    ) -> Self {
        Self {
            pane_chars: pane_chars.max(1),
            word_wrap,
            lines_ptr: Arc::as_ptr(lines) as usize,
            highlight_version,
            line_count: lines.len(),
        }
    }
}

#[derive(Debug, Default)]
struct InternalState {
    focused: bool,
    dragging: bool,
    cell_width: f32,
    gutter_width: f32,
    /// URL currently under the mouse while a modifier is held. Drives the
    /// underline overlay and the pointer cursor; cleared when modifiers
    /// release or the mouse moves off the link.
    link_hover: Option<LinkHover>,
    /// Last frame instant we stepped on. iced re-dispatches the *same*
    /// RedrawRequested(Instant) several times per real frame; step once per
    /// distinct instant or we scroll multiple steps/frame and trip iced's
    /// layout-invalidation guard.
    last_autoscroll_frame: Option<std::time::Instant>,
    /// Whether the drag is currently auto-scrolling. We must re-request a
    /// redraw on *every* dispatch while true — not only the one that steps —
    /// or the loop stalls the instant the mouse stops.
    autoscrolling: bool,
    /// Caret position observed on the previous event, used by the capped
    /// input's caret-follow to detect that a keyboard action moved the caret.
    last_cursor: Option<Pos>,
    /// Set when a keyboard action that may move the caret was just dispatched;
    /// consumed on the next event to nudge `scroll_y`. Mouse clicks never set
    /// it, so clicking a scrolled input doesn't yank the view to the caret.
    follow_after_key: bool,
    /// Cached hybrid layout for the `md_tables` path. Shared by layout,
    /// update, and draw via `RefCell` so draw can fill on a cold miss without
    /// a mutable tree. Invalidated when `HybridLayoutKey` changes.
    hybrid_layout: RefCell<Option<(HybridLayoutKey, EditorLayout)>>,
}

/// A URL or file-path reference found at a click/hover position in the
/// editor's text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkHover {
    /// Logical line index in `EditorState::lines`.
    line: usize,
    /// Char offset of the link's first character within the line.
    char_start: usize,
    /// Char offset one past the link's last character.
    char_end: usize,
    target: LinkTarget,
}

impl operation::Focusable for InternalState {
    fn is_focused(&self) -> bool {
        self.focused
    }
    fn focus(&mut self) {
        self.focused = true;
    }
    fn unfocus(&mut self) {
        self.focused = false;
    }
}

// ── Word wrap ─────────────────────────────────────────────────────────────

/// Cached word-wrap layout for all lines.
#[derive(Debug, Clone)]
struct WrapLayout {
    /// For each logical line: the character offsets where each visual row starts.
    row_starts: Vec<Vec<usize>>,
    /// Total number of visual rows across all logical lines.
    total_visual_rows: usize,
    /// Cumulative visual row offset for each logical line.
    cum_rows: Vec<usize>,
}

impl WrapLayout {
    fn compute(lines: &[String], chars_per_row: usize) -> Self {
        let mut row_starts = Vec::with_capacity(lines.len());
        let mut cum_rows = Vec::with_capacity(lines.len());
        let mut total = 0usize;

        for line in lines {
            let starts = wrap_line_starts(line, chars_per_row);
            let n_rows = starts.len();
            cum_rows.push(total);
            total += n_rows;
            row_starts.push(starts);
        }

        Self {
            row_starts,
            total_visual_rows: total,
            cum_rows,
        }
    }

    /// Convert a visual row index to (logical_line, visual_row_within_line).
    fn visual_to_logical(&self, visual_row: usize) -> (usize, usize) {
        let line = match self.cum_rows.binary_search(&visual_row) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line = line.min(self.row_starts.len().saturating_sub(1));
        let sub_row = visual_row.saturating_sub(self.cum_rows[line]);
        (line, sub_row)
    }
}

/// Cursor's visual row + char column within that row, given a wrap layout.
fn cursor_visual_pos(state: &EditorState, wrap: &WrapLayout) -> (usize, usize) {
    let line = state
        .cursor
        .line
        .min(wrap.row_starts.len().saturating_sub(1));
    let line_str = &state.lines[line];
    let byte_col = state.cursor.col.min(line_str.len());
    let char_col = line_str[..byte_col].chars().count();
    let starts = &wrap.row_starts[line];
    let sub_row = starts.iter().rposition(|&s| char_col >= s).unwrap_or(0);
    let row_start = starts[sub_row];
    let visual_row = wrap.cum_rows[line] + sub_row;
    (visual_row, char_col - row_start)
}

/// Cursor visual placement when hybrid table layout is active.
fn cursor_visual_pos_hybrid(state: &EditorState, ed: &EditorLayout) -> (usize, usize) {
    let line = state.cursor.line.min(ed.line_kind.len().saturating_sub(1));
    match ed.line_kind.get(line) {
        Some(LineLayoutKind::TableRow { region, row }) => {
            if let Some(tv) = md_table::source_to_visual(
                &ed.tables,
                state.cursor,
            ) {
                // Map region-local visual row to editor-global via the row's
                // first source line cum_rows.
                let row_line = ed.tables.regions[*region].rows[*row].source_line;
                let base = ed.cum_rows.get(row_line).copied().unwrap_or(0);
                let within_row = tv
                    .visual_row_in_region
                    .saturating_sub(ed.table_visual_row_in_region(*region, *row, 0));
                (base + within_row, tv.char_col)
            } else {
                (ed.cum_rows.get(line).copied().unwrap_or(0), 0)
            }
        }
        Some(LineLayoutKind::TableSeparator { .. }) => {
            // Separator has no visual row; park on the next data row's start.
            (ed.cum_rows.get(line).copied().unwrap_or(0), 0)
        }
        _ => {
            let line_str = &state.lines[line];
            let byte_col = state.cursor.col.min(line_str.len());
            let char_col = line_str[..byte_col].chars().count();
            let starts = ed.prose_row_starts.get(line).map(|s| s.as_slice()).unwrap_or(&[0]);
            let sub_row = starts.iter().rposition(|&s| char_col >= s).unwrap_or(0);
            let row_start = starts.get(sub_row).copied().unwrap_or(0);
            let visual_row = ed.cum_rows.get(line).copied().unwrap_or(0) + sub_row;
            (visual_row, char_col.saturating_sub(row_start))
        }
    }
}

/// Translate a visual sub-row + char column-within-row back to a logical
/// `Pos` (byte-offset col), clamping to the row's visual width.
fn visual_to_logical_pos(
    state: &EditorState,
    wrap: &WrapLayout,
    line: usize,
    sub_row: usize,
    col_in_row: usize,
) -> Pos {
    let starts = &wrap.row_starts[line];
    let row_start = starts[sub_row];
    let row_end = if sub_row + 1 < starts.len() {
        starts[sub_row + 1]
    } else {
        state.lines[line].chars().count()
    };
    let char_col = (row_start + col_in_row).min(row_end);
    let line_str = &state.lines[line];
    let byte_col = line_str
        .char_indices()
        .nth(char_col)
        .map(|(b, _)| b)
        .unwrap_or(line_str.len());
    Pos::new(line, byte_col)
}

/// Pos one visual row above the cursor when wrap is enabled. `None` means
/// the cursor is already on the topmost visual row, so callers should fall
/// back to logical `MoveUp`.
fn visual_up_target(state: &EditorState, wrap: &WrapLayout) -> Option<Pos> {
    let (visual_row, col_in_row) = cursor_visual_pos(state, wrap);
    if visual_row == 0 {
        return None;
    }
    let (target_line, target_sub_row) = wrap.visual_to_logical(visual_row - 1);
    Some(visual_to_logical_pos(
        state,
        wrap,
        target_line,
        target_sub_row,
        col_in_row,
    ))
}

/// Mirror of `visual_up_target` for downward motion.
fn visual_down_target(state: &EditorState, wrap: &WrapLayout) -> Option<Pos> {
    let (visual_row, col_in_row) = cursor_visual_pos(state, wrap);
    if visual_row + 1 >= wrap.total_visual_rows {
        return None;
    }
    let (target_line, target_sub_row) = wrap.visual_to_logical(visual_row + 1);
    Some(visual_to_logical_pos(
        state,
        wrap,
        target_line,
        target_sub_row,
        col_in_row,
    ))
}

/// Translate hybrid visual coordinates (prose wrap or table band) back to a
/// source `Pos`, preserving preferred column within the visual row.
fn hybrid_visual_to_pos(
    state: &EditorState,
    ed: &EditorLayout,
    visual_row: usize,
    col_in_row: usize,
) -> Pos {
    let (line_idx, sub_row) = ed.visual_to_logical(visual_row);
    match ed.line_kind.get(line_idx) {
        Some(LineLayoutKind::TableRow { region, row }) => {
            let visual_in_region = ed.table_visual_row_in_region(*region, *row, sub_row);
            md_table::visual_to_source(&ed.tables, *region, visual_in_region, col_in_row)
                .unwrap_or_else(|| Pos::new(line_idx, 0))
        }
        Some(LineLayoutKind::TableSeparator { .. }) => Pos::new(line_idx, 0),
        _ => {
            let starts = ed
                .prose_row_starts
                .get(line_idx)
                .map(|s| s.as_slice())
                .unwrap_or(&[0]);
            let char_start = starts.get(sub_row).copied().unwrap_or(0);
            let char_end = if sub_row + 1 < starts.len() {
                starts[sub_row + 1]
            } else {
                state
                    .lines
                    .get(line_idx)
                    .map(|l| l.chars().count())
                    .unwrap_or(0)
            };
            let col = (char_start + col_in_row).min(char_end);
            let line = state.lines.get(line_idx).map(|s| s.as_str()).unwrap_or("");
            let byte_col = line
                .char_indices()
                .nth(col)
                .map(|(b, _)| b)
                .unwrap_or(line.len());
            Pos::new(line_idx, byte_col)
        }
    }
}

/// Pos one visual row above the cursor under hybrid prose+table layout.
fn visual_up_target_hybrid(state: &EditorState, ed: &EditorLayout) -> Option<Pos> {
    let (visual_row, col_in_row) = cursor_visual_pos_hybrid(state, ed);
    if visual_row == 0 {
        return None;
    }
    Some(hybrid_visual_to_pos(state, ed, visual_row - 1, col_in_row))
}

/// Mirror of `visual_up_target_hybrid` for downward motion.
fn visual_down_target_hybrid(state: &EditorState, ed: &EditorLayout) -> Option<Pos> {
    let (visual_row, col_in_row) = cursor_visual_pos_hybrid(state, ed);
    if visual_row + 1 >= ed.total_visual_rows {
        return None;
    }
    Some(hybrid_visual_to_pos(state, ed, visual_row + 1, col_in_row))
}

/// Compute the character offsets where each visual row starts for a single line.
/// Thin wrapper over the shared soft-wrap primitive used by table cells too.
fn wrap_line_starts(line: &str, max_chars: usize) -> Vec<usize> {
    md_table::soft_wrap_starts(line, max_chars)
}

// ── Hybrid prose + table layout ────────────────────────────────────────────

/// How one logical source line participates in visual space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineLayoutKind {
    /// Ordinary line: 1 visual row, or wrap sub-rows via `prose_row_starts`.
    Prose,
    /// Separator line inside a table: 0 visual rows (aligns only).
    TableSeparator { region: usize },
    /// Header/body row: height from the table kernel's `TableRow`.
    TableRow { region: usize, row: usize },
}

/// Unified visual layout for one frame when `md_tables` is on.
#[derive(Debug, Clone)]
struct EditorLayout {
    /// Total visual rows across prose + tables.
    total_visual_rows: usize,
    /// Max content width in chars (pane for wrapped prose; may be larger when
    /// a table overflows at MIN_COL).
    content_width_chars: usize,
    /// Pane width in character cells used to build this layout.
    pane_chars: usize,
    /// Table kernel output (may be empty if no complete GFM tables).
    tables: TableLayout,
    /// For each logical line: how it participates in visual space.
    line_kind: Vec<LineLayoutKind>,
    /// Char-offset wrap starts for Prose lines; empty for table lines.
    prose_row_starts: Vec<Vec<usize>>,
    /// Cumulative visual row offset for each logical line.
    cum_rows: Vec<usize>,
}

impl EditorLayout {
    fn compute(lines: &[String], pane_chars: usize, word_wrap: bool) -> Self {
        let tables = md_table::layout_tables(lines, pane_chars.max(1));
        let mut line_kind = vec![LineLayoutKind::Prose; lines.len()];
        for (ri, region) in tables.regions.iter().enumerate() {
            for line in region.source_lines.clone() {
                if line >= lines.len() {
                    continue;
                }
                if let Some(row_i) = region.rows.iter().position(|r| r.source_line == line) {
                    line_kind[line] = LineLayoutKind::TableRow {
                        region: ri,
                        row: row_i,
                    };
                } else {
                    line_kind[line] = LineLayoutKind::TableSeparator { region: ri };
                }
            }
        }

        let mut prose_row_starts = Vec::with_capacity(lines.len());
        let mut cum_rows = Vec::with_capacity(lines.len());
        let mut total = 0usize;

        for (i, line) in lines.iter().enumerate() {
            cum_rows.push(total);
            match line_kind[i] {
                LineLayoutKind::TableSeparator { .. } => {
                    prose_row_starts.push(Vec::new());
                }
                LineLayoutKind::TableRow { region, row } => {
                    let h = tables.regions[region].rows[row].height.max(1);
                    prose_row_starts.push(Vec::new());
                    total += h;
                }
                LineLayoutKind::Prose => {
                    let starts = if word_wrap {
                        wrap_line_starts(line, pane_chars.max(1))
                    } else {
                        vec![0]
                    };
                    total += starts.len().max(1);
                    prose_row_starts.push(starts);
                }
            }
        }

        // Prose content width: pane when wrapping, else longest line.
        let mut content_width_chars = if word_wrap {
            pane_chars
        } else {
            lines
                .iter()
                .map(|l| l.chars().count())
                .max()
                .unwrap_or(0)
                .max(pane_chars)
        };
        for region in &tables.regions {
            content_width_chars = content_width_chars.max(region.total_width_chars);
        }

        Self {
            total_visual_rows: total.max(lines.len().min(1)),
            content_width_chars,
            pane_chars: pane_chars.max(1),
            tables,
            line_kind,
            prose_row_starts,
            cum_rows,
        }
    }
}

/// Get-or-compute hybrid layout, cached on `InternalState` until pane width,
/// wrap flag, or lines identity/version changes. Used by layout, update, and
/// draw so a frame only runs `layout_tables` once for matching keys.
fn cached_hybrid_layout(
    internal: &InternalState,
    lines: &Arc<Vec<String>>,
    highlight_version: u64,
    pane_chars: usize,
    word_wrap: bool,
) -> EditorLayout {
    let key = HybridLayoutKey::new(lines, highlight_version, pane_chars, word_wrap);
    let mut cache = internal.hybrid_layout.borrow_mut();
    if let Some((k, ed)) = cache.as_ref()
        && *k == key
    {
        return ed.clone();
    }
    let ed = EditorLayout::compute(lines, pane_chars, word_wrap);
    *cache = Some((key, ed.clone()));
    ed
}

// Keep `EditorLayout` methods in a second impl block below.
impl EditorLayout {

    /// Convert a visual row index to (logical_line, sub_row within that line).
    fn visual_to_logical(&self, visual_row: usize) -> (usize, usize) {
        if self.cum_rows.is_empty() {
            return (0, 0);
        }
        let line = match self.cum_rows.binary_search(&visual_row) {
            Ok(i) => {
                // Exact hit: skip zero-height lines (separators) that share
                // this cumulative offset with a later non-empty line.
                let mut i = i;
                while i + 1 < self.cum_rows.len() && self.cum_rows[i + 1] == visual_row {
                    i += 1;
                }
                i
            }
            Err(i) => i.saturating_sub(1),
        };
        let line = line.min(self.line_kind.len().saturating_sub(1));
        let sub_row = visual_row.saturating_sub(self.cum_rows[line]);
        (line, sub_row)
    }

    /// Region-local visual row for a table data row + fragment sub-row.
    fn table_visual_row_in_region(&self, region: usize, row: usize, sub_row: usize) -> usize {
        let Some(reg) = self.tables.regions.get(region) else {
            return sub_row;
        };
        let before: usize = reg.rows.iter().take(row).map(|r| r.height).sum();
        before + sub_row
    }
}

fn pane_chars_for(content_w: f32, cell_w: f32) -> usize {
    let usable = (content_w - CONTENT_PAD).max(0.0);
    (usable / cell_w.max(0.1)).floor().max(1.0) as usize
}

// ── Widget ─────────────────────────────────────────────────────────────────

pub struct TextEdit<'a, M> {
    state: &'a EditorState,
    on_action: Box<dyn Fn(EditorAction) -> M + 'a>,
    show_gutter: bool,
    fit_content: bool,
    /// When set (with `fit_content`), the editor grows to at most this many
    /// visual rows, then clips and scrolls internally to keep the cursor in
    /// view. Used by the auto-growing chat input so a long prompt doesn't
    /// push the caret off the bottom of the window.
    max_rows: Option<usize>,
    read_only: bool,
    word_wrap: bool,
    /// When true (typically with word_wrap), GFM tables use `TableLayout`.
    md_tables: bool,
    placeholder: Option<String>,
    on_submit: Option<M>,
    transparent_bg: bool,
    id: Option<Id>,
    /// When true the editor shows a fixed window into its content: no
    /// scrollbars are drawn and wheel events are ignored so the parent
    /// scrollable handles them. Used by the search-stack slices.
    static_viewport: bool,
    /// Match-range highlights to overlay in `theme::search_match_bg`. The
    /// "current" candidate (if any) gets a stronger accent fill on top.
    /// Owned so callers can compute highlights inside `view()` and pass them
    /// down without lifetime gymnastics — the Vec lives inside the widget
    /// builder, which lives inside the returned Element.
    highlight_ranges: Vec<HighlightRange>,
    current_highlight: Option<HighlightRange>,
}

impl<'a, M> TextEdit<'a, M> {
    pub fn new(state: &'a EditorState, on_action: impl Fn(EditorAction) -> M + 'a) -> Self {
        Self {
            state,
            on_action: Box::new(on_action),
            show_gutter: true,
            fit_content: false,
            max_rows: None,
            read_only: false,
            word_wrap: false,
            md_tables: false,
            placeholder: None,
            on_submit: None,
            transparent_bg: false,
            id: None,
            static_viewport: false,
            highlight_ranges: Vec::new(),
            current_highlight: None,
        }
    }

    /// Overlay match highlights drawn in the muted "search match" background
    /// color. Use `current_highlight` to mark one of these as the active
    /// candidate (stronger accent fill).
    pub fn highlights(
        mut self,
        ranges: Vec<HighlightRange>,
        current: Option<HighlightRange>,
    ) -> Self {
        self.highlight_ranges = ranges;
        self.current_highlight = current;
        self
    }

    /// Assign an [`Id`] so the editor can be targeted by focus operations
    /// like `iced::widget::operation::focus(id)`.
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn show_gutter(mut self, show: bool) -> Self {
        self.show_gutter = show;
        self
    }

    pub fn fit_content(mut self, fit: bool) -> Self {
        self.fit_content = fit;
        self
    }

    /// Cap the auto-grown height at `rows` visual rows. Beyond that the editor
    /// clips and scrolls internally, keeping the cursor visible. Only meaningful
    /// together with `fit_content`.
    pub fn max_rows(mut self, rows: usize) -> Self {
        self.max_rows = Some(rows);
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        self
    }

    /// When true, complete GFM pipe tables are laid out as fit-to-pane grids
    /// (hybrid with ordinary wrap for non-table lines). Default `false` so
    /// file tabs stay line-faithful.
    pub fn md_tables(mut self, enabled: bool) -> Self {
        self.md_tables = enabled;
        self
    }

    /// Text shown in muted color when the editor is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// When set, plain Enter (without Shift) fires this message instead of
    /// inserting a newline. Shift+Enter always inserts a newline.
    pub fn on_submit(mut self, msg: M) -> Self {
        self.on_submit = Some(msg);
        self
    }

    /// Skip painting the editor background — the parent container provides it.
    pub fn transparent_bg(mut self, transparent: bool) -> Self {
        self.transparent_bg = transparent;
        self
    }

    /// Render a fixed, non-scrollable view into the content: no scrollbars,
    /// and wheel events pass through to the parent scrollable. The caller is
    /// expected to pre-set `EditorState::scroll_y` to the desired window.
    pub fn static_viewport(mut self, v: bool) -> Self {
        self.static_viewport = v;
        self
    }

    /// Vertical scroll offset to render and hit-test at — the single source of
    /// truth so the painted frame and the click math never disagree. It is the
    /// user/keyboard-driven `scroll_y` clamped to range. The caret is kept in
    /// view by *persisting* nudges to `scroll_y` on keyboard edits (see the
    /// caret-follow block in `update`), never by re-deriving the offset here —
    /// otherwise a mouse click, which moves the caret, would yank the view.
    fn resolved_scroll_y(&self, viewport_height: f32, content_height: f32) -> f32 {
        let max_scroll = (content_height - viewport_height).max(0.0);
        self.state.scroll_y.clamp(0.0, max_scroll)
    }

    /// Once the caret has moved (via keyboard) past the visible window of a
    /// capped input, the persisted `scroll_y` it should hold to bring the
    /// caret's visual row back into view. Returns `None` when no change is
    /// needed (caret already visible, or input not capped/overflowing).
    fn caret_follow_scroll_y(
        &self,
        viewport_height: f32,
        content_height: f32,
        wrap: Option<&WrapLayout>,
        hybrid: Option<&EditorLayout>,
    ) -> Option<f32> {
        if self.max_rows.is_none() {
            return None;
        }
        let max_scroll = (content_height - viewport_height).max(0.0);
        if max_scroll <= 0.0 {
            return None;
        }
        let cursor_vrow = if let Some(ed) = hybrid {
            cursor_visual_pos_hybrid(self.state, ed).0
        } else {
            match wrap {
                Some(w) => cursor_visual_pos(self.state, w).0,
                None => self.state.cursor.line,
            }
        };
        let cursor_top = cursor_vrow as f32 * LINE_HEIGHT + CONTENT_PAD_Y;
        let cursor_bottom = cursor_top + LINE_HEIGHT;
        let cur = self.state.scroll_y.clamp(0.0, max_scroll);
        let target = if cursor_top < cur {
            cursor_top
        } else if cursor_bottom > cur + viewport_height {
            cursor_bottom - viewport_height
        } else {
            return None;
        };
        let target = target.clamp(0.0, max_scroll);
        ((target - cur).abs() > 0.5).then_some(target)
    }

    fn drag_frame(
        &self,
        pos: Point,
        bounds: Rectangle,
        viewport: Rectangle,
        internal: &InternalState,
        wrap: Option<&WrapLayout>,
        hybrid: Option<&EditorLayout>,
        shell: &mut Shell<'_, M>,
    ) -> bool {
        let cell_w = if internal.cell_width > 0.0 {
            internal.cell_width
        } else {
            7.8
        };
        let content_height = if let Some(ed) = hybrid {
            ed.total_visual_rows as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0
        } else {
            wrap.map_or(self.state.lines.len() as f32, |w| w.total_visual_rows as f32)
                * LINE_HEIGHT
                + CONTENT_PAD_Y * 2.0
        };

        // Hit-test Y + autoscroll velocity against bounds ∩ viewport. When the
        // intersection is empty (chat block just past the outer fold while a
        // drag is still live), the helper pins to the nearest bounds edge and
        // still drives scroll toward bringing the block back on screen — never
        // `f32::clamp` with min > max.
        let (drag_y, velocity) = drag_pointer_y_and_velocity(pos.y, bounds, viewport);
        let scroll_y = self.resolved_scroll_y(bounds.height, content_height);

        // Selection tracks the (possibly edge-pinned) drag line so a big
        // overshoot doesn't snap selection to the doc end.
        let drag_pos = pixel_to_pos_wrapped(
            Point::new(pos.x, drag_y),
            bounds,
            internal,
            self.state,
            wrap,
            hybrid,
            scroll_y,
        );
        shell.publish((self.on_action)(EditorAction::Drag(drag_pos)));

        if velocity == 0.0 || self.static_viewport {
            return false;
        }

        if content_height > bounds.height {
            // The editor owns the hidden content: self-scroll in pixels, but
            // only while there is room to move in the pointer's direction so
            // the loop stops at the content's end instead of spinning forever.
            let max_scroll = (content_height - bounds.height).max(0.0);
            let has_room = if velocity > 0.0 {
                scroll_y < max_scroll
            } else {
                scroll_y > 0.0
            };
            if !has_room {
                return false;
            }
            let content_width = content_width_px(self.word_wrap, hybrid, self.state, cell_w);
            shell.publish((self.on_action)(EditorAction::Scroll {
                dy: velocity,
                dx: 0.0,
                viewport_height: bounds.height,
                content_height,
                viewport_width: bounds.width - internal.gutter_width,
                content_width,
            }));
        } else {
            // The editor fits its content but is clipped by an outer scrollable
            // (a chat message body): it has no overflow of its own to move, so
            // the host scrolls the outer container.
            shell.publish((self.on_action)(EditorAction::AutoScroll { dy: velocity }));
        }
        shell.request_redraw();
        true
    }
}

/// Resolve drag hit-test Y and signed auto-scroll velocity for one drag frame.
///
/// When `bounds ∩ viewport` is non-empty, the pointer is clamped into that
/// visible span (so overshoot selects the edge line, not the document end) and
/// velocity is measured against the same span.
///
/// When the intersection is empty — a nested chat message whose layout rect
/// has scrolled just past the outer fold while the drag is still live — a
/// naive `pointer.clamp(top, bottom)` panics (`min > max`). Instead:
/// - pin hit-test Y to the bounds edge nearest the viewport
/// - sample velocity against the *viewport* using a Y forced past that edge
///   by the off-screen block, so a still pointer still drives outer scroll
///   toward bringing the block back on screen
fn drag_pointer_y_and_velocity(
    pointer_y: f32,
    bounds: Rectangle,
    viewport: Rectangle,
) -> (f32, f32) {
    let bounds_bottom = bounds.y + bounds.height;
    let viewport_bottom = viewport.y + viewport.height;
    let top = bounds.y.max(viewport.y);
    let bottom = bounds_bottom.min(viewport_bottom);

    if top <= bottom {
        let drag_y = pointer_y.clamp(top, bottom);
        let velocity = autoscroll::edge_velocity(pointer_y, top, bottom);
        return (drag_y, velocity);
    }

    // Empty intersection: block fully above or below the clip rect.
    if bounds.y >= viewport_bottom {
        // Entirely below the viewport — pin to block top, scroll down.
        let drag_y = bounds.y;
        let sample_y = pointer_y.max(bounds.y);
        let velocity = autoscroll::edge_velocity(sample_y, viewport.y, viewport_bottom);
        (drag_y, velocity)
    } else {
        // Entirely above the viewport — pin to block bottom, scroll up.
        let drag_y = bounds_bottom;
        let sample_y = pointer_y.min(bounds_bottom);
        let velocity = autoscroll::edge_velocity(sample_y, viewport.y, viewport_bottom);
        (drag_y, velocity)
    }
}

/// Horizontal content width in px for scroll math. Zero means "no x scroll"
/// (classic word-wrap). Hybrid tables may exceed the pane and enable `scroll_x`
/// even when word wrap is on.
fn content_width_px(
    word_wrap: bool,
    hybrid: Option<&EditorLayout>,
    state: &EditorState,
    cell_w: f32,
) -> f32 {
    if let Some(ed) = hybrid {
        if ed.content_width_chars > ed.pane_chars || !word_wrap {
            return ed.content_width_chars as f32 * cell_w + CONTENT_PAD * 2.0;
        }
        return 0.0;
    }
    if word_wrap {
        return 0.0;
    }
    let max_chars = state
        .lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    max_chars as f32 * cell_w + CONTENT_PAD * 2.0
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for TextEdit<'a, M> {
    fn size(&self) -> Size<Length> {
        let h = if self.fit_content {
            Length::Shrink
        } else {
            Length::Fill
        };
        Size::new(Length::Fill, h)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        if self.fit_content {
            let row_count = if self.md_tables || self.word_wrap {
                let max_w = limits.max().width;
                // Match the cosmic-text measurement used by draw/update so the
                // height we report to iced equals the height actually painted.
                // A hardcoded 7.8 px/char overestimates rows for wider fonts,
                // leaving phantom empty space below long wrapped messages.
                let cell_w = theme::content_cell_width();
                let content_area = max_w - if self.show_gutter { 50.0 } else { 0.0 };
                let cpr = pane_chars_for(content_area, cell_w);
                if self.md_tables {
                    let internal = tree.state.downcast_ref::<InternalState>();
                    cached_hybrid_layout(
                        internal,
                        &self.state.lines,
                        self.state.highlight_version,
                        cpr,
                        self.word_wrap,
                    )
                    .total_visual_rows
                } else {
                    WrapLayout::compute(&self.state.lines, cpr).total_visual_rows
                }
            } else {
                self.state.line_count()
            };
            // Cap the reported height so a tall prompt stops growing and
            // instead scrolls internally (see `resolved_scroll_y`).
            let rows = self.max_rows.map_or(row_count, |m| row_count.min(m.max(1)));
            let height = rows.max(1) as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0;
            let limits = limits.width(Length::Fill);
            layout::Node::new(limits.resolve(Length::Fill, Length::Fixed(height), Size::ZERO))
        } else {
            let limits = limits.width(Length::Fill).height(Length::Fill);
            layout::Node::new(limits.resolve(Length::Fill, Length::Fill, Size::ZERO))
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<InternalState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(InternalState::default())
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let internal = tree.state.downcast_mut::<InternalState>();
        operation.focusable(self.id.as_ref(), layout.bounds(), internal);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let internal = tree.state.downcast_mut::<InternalState>();

        // Measure cell width on first event if not set.
        if internal.cell_width == 0.0 {
            internal.cell_width = measure_cell_width(renderer);
        }
        // Refresh gutter width from live state every event. The draw path
        // formats line numbers from the current `line_count()`, so a cached
        // gutter — e.g. left over from a previously-mounted file with a
        // different digit count — would put click math out of sync with the
        // visible gutter and offset the cursor by a fixed number of cells.
        internal.gutter_width = if self.show_gutter {
            let digits = digit_count(self.state.line_count());
            (digits as f32) * internal.cell_width + GUTTER_PAD * 2.0
        } else {
            0.0
        };

        // Compute wrap / hybrid layout if enabled (hybrid path is cached).
        let cell_w = if internal.cell_width > 0.0 {
            internal.cell_width
        } else {
            7.8
        };
        let content_area = bounds.width - internal.gutter_width;
        let hybrid = if self.md_tables {
            let cpr = pane_chars_for(content_area, cell_w);
            Some(cached_hybrid_layout(
                internal,
                &self.state.lines,
                self.state.highlight_version,
                cpr,
                self.word_wrap,
            ))
        } else {
            None
        };
        let wrap = if self.md_tables {
            // Hybrid owns visual geometry when tables are enabled.
            None
        } else if self.word_wrap {
            let cpr = pane_chars_for(content_area, cell_w);
            Some(WrapLayout::compute(&self.state.lines, cpr))
        } else {
            None
        };

        let content_height = if let Some(ref ed) = hybrid {
            ed.total_visual_rows as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0
        } else if let Some(ref w) = wrap {
            w.total_visual_rows as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0
        } else {
            self.state.lines.len() as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0
        };
        // Caret-follow for a capped input: when a keyboard action moved the
        // caret last frame, persist a scroll nudge so the caret stays visible.
        // Gated on `follow_after_key` so mouse clicks (which also move the
        // caret) never trigger it — that would yank the view and desync the
        // click/drag hit-test, producing a phantom selection.
        if internal.follow_after_key && internal.last_cursor != Some(self.state.cursor) {
            if let Some(target) = self.caret_follow_scroll_y(
                bounds.height,
                content_height,
                wrap.as_ref(),
                hybrid.as_ref(),
            ) {
                shell.publish((self.on_action)(EditorAction::Scroll {
                    dy: target - self.state.scroll_y,
                    dx: 0.0,
                    viewport_height: bounds.height,
                    content_height,
                    viewport_width: bounds.width - internal.gutter_width,
                    content_width: 0.0,
                }));
                shell.request_redraw();
            }
            internal.follow_after_key = false;
        }
        internal.last_cursor = Some(self.state.cursor);

        // Same resolved offset the draw path uses, so click math lands on the
        // line the user actually sees in a scrolled, capped input.
        let scroll_y = self.resolved_scroll_y(bounds.height, content_height);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds) {
                    let pos = cursor.position().unwrap();
                    // Cmd-click on a hovered link opens it instead of moving
                    // the caret. Don't focus or start a drag.
                    if current_modifiers().command()
                        && let Some(hover) = internal.link_hover.clone()
                        && pos_in_hover(
                            pos,
                            bounds,
                            internal,
                            self.state,
                            wrap.as_ref(),
                            hybrid.as_ref(),
                            scroll_y,
                            &hover,
                        )
                    {
                        let action = match hover.target {
                            LinkTarget::Url(url) => EditorAction::OpenUrl(url),
                            LinkTarget::Path { path, line, .. } => {
                                EditorAction::OpenPath { path, line }
                            }
                        };
                        shell.publish((self.on_action)(action));
                        shell.capture_event();
                        return;
                    }

                    internal.focused = true;
                    let click_pos = pixel_to_pos_wrapped(
                        pos,
                        bounds,
                        internal,
                        self.state,
                        wrap.as_ref(),
                        hybrid.as_ref(),
                        scroll_y,
                    );

                    internal.dragging = true;
                    shell.publish((self.on_action)(EditorAction::Click(click_pos)));
                } else {
                    internal.focused = false;
                    internal.dragging = false;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // `cursor.land().position()` so a pointer dragged past an
                // enclosing scrollable's fold (arrives Levitating, with
                // `position() == None`) still yields a point — exactly when a
                // nested chat message needs to auto-scroll.
                if internal.dragging
                    && internal.focused
                    && let Some(pos) = cursor.land().position()
                {
                    self.drag_frame(
                        pos,
                        bounds,
                        *viewport,
                        internal,
                        wrap.as_ref(),
                        hybrid.as_ref(),
                        shell,
                    );
                } else {
                    // Hover detection while cmd is held.
                    let new_hover = if current_modifiers().command()
                        && let Some(pos) = cursor.position()
                        && bounds.contains(pos)
                    {
                        let click_pos = pixel_to_pos_wrapped(
                            pos,
                            bounds,
                            internal,
                            self.state,
                            wrap.as_ref(),
                            hybrid.as_ref(),
                            scroll_y,
                        );
                        detect_link_at(self.state, click_pos)
                    } else {
                        None
                    };
                    if internal.link_hover != new_hover {
                        internal.link_hover = new_hover;
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let was_dragging = internal.dragging;
                internal.dragging = false;
                // Publish a final action so callers can distinguish "drag
                // still extending" from "user released the mouse and the
                // selection is now stable". Used by the agent-chat
                // selection-attachment feature to defer chip rendering
                // until the drag ends — chips appearing mid-drag would
                // reflow the chat area under the user's cursor.
                if was_dragging {
                    shell.publish((self.on_action)(EditorAction::DragEnd));
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                // Skip when static_viewport is set so the parent scrollable
                // receives the event instead.
                if !self.static_viewport && cursor.is_over(bounds) {
                    let (dy, dx) = match delta {
                        mouse::ScrollDelta::Lines { x, y } => {
                            (-*y * LINE_HEIGHT * 3.0, -*x * cell_w * 3.0)
                        }
                        mouse::ScrollDelta::Pixels { x, y } => (-*y, -*x),
                    };
                    let content_w_px =
                        content_width_px(self.word_wrap, hybrid.as_ref(), self.state, cell_w);
                    let viewport_w = bounds.width - internal.gutter_width;
                    shell.publish((self.on_action)(EditorAction::Scroll {
                        dy,
                        dx,
                        viewport_height: bounds.height,
                        content_height,
                        viewport_width: viewport_w,
                        content_width: content_w_px,
                    }));
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text: key_text,
                ..
            }) if internal.focused => {
                let cmd = modifiers.command();
                let shift = modifiers.shift();
                let mut handled = true;

                match key {
                    // Navigation — always allowed.
                    keyboard::Key::Named(Named::ArrowLeft) if cmd => {
                        shell.publish((self.on_action)(EditorAction::MoveWordLeft(shift)));
                    }
                    keyboard::Key::Named(Named::ArrowRight) if cmd => {
                        shell.publish((self.on_action)(EditorAction::MoveWordRight(shift)));
                    }
                    keyboard::Key::Named(Named::ArrowLeft) => {
                        shell.publish((self.on_action)(EditorAction::MoveLeft(shift)));
                    }
                    keyboard::Key::Named(Named::ArrowRight) => {
                        shell.publish((self.on_action)(EditorAction::MoveRight(shift)));
                    }
                    keyboard::Key::Named(Named::ArrowUp) => {
                        let target = if let Some(ref ed) = hybrid {
                            visual_up_target_hybrid(self.state, ed)
                        } else {
                            wrap.as_ref().and_then(|w| visual_up_target(self.state, w))
                        };
                        let action = match target {
                            Some(pos) if shift => EditorAction::Drag(pos),
                            Some(pos) => EditorAction::Click(pos),
                            None => EditorAction::MoveUp(shift),
                        };
                        shell.publish((self.on_action)(action));
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        let target = if let Some(ref ed) = hybrid {
                            visual_down_target_hybrid(self.state, ed)
                        } else {
                            wrap.as_ref()
                                .and_then(|w| visual_down_target(self.state, w))
                        };
                        let action = match target {
                            Some(pos) if shift => EditorAction::Drag(pos),
                            Some(pos) => EditorAction::Click(pos),
                            None => EditorAction::MoveDown(shift),
                        };
                        shell.publish((self.on_action)(action));
                    }
                    keyboard::Key::Named(Named::Home) => {
                        shell.publish((self.on_action)(EditorAction::MoveHome(shift)));
                    }
                    keyboard::Key::Named(Named::End) => {
                        shell.publish((self.on_action)(EditorAction::MoveEnd(shift)));
                    }
                    // Select all + copy — always allowed.
                    keyboard::Key::Character(c) if cmd && c.as_str() == "a" => {
                        shell.publish((self.on_action)(EditorAction::SelectAll));
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "c" => {
                        if let Some(sel) = self.state.selection_text() {
                            clipboard.write(iced::advanced::clipboard::Kind::Standard, sel);
                        }
                    }
                    // Edit actions — skip in read-only mode.
                    keyboard::Key::Named(Named::Backspace) if !self.read_only => {
                        shell.publish((self.on_action)(EditorAction::Backspace));
                    }
                    keyboard::Key::Named(Named::Delete) if !self.read_only => {
                        shell.publish((self.on_action)(EditorAction::Delete));
                    }
                    keyboard::Key::Named(Named::Enter) if !self.read_only => {
                        // Plain Enter → on_submit (or newline). ⌘/Ctrl+Enter is
                        // left uncaptured so app-level handlers can activate the
                        // obvious bubble without also firing empty-submit.
                        if cmd {
                            handled = false;
                        } else if !shift && let Some(msg) = self.on_submit.as_ref() {
                            shell.publish(msg.clone());
                        } else {
                            shell.publish((self.on_action)(EditorAction::Enter));
                        }
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "x" && !self.read_only => {
                        if let Some(sel) = self.state.selection_text() {
                            clipboard.write(iced::advanced::clipboard::Kind::Standard, sel);
                            shell.publish((self.on_action)(EditorAction::Cut));
                        }
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "v" && !self.read_only => {
                        if let Some(action) = read_paste_action() {
                            shell.publish((self.on_action)(action));
                        } else if let Some(text) =
                            clipboard.read(iced::advanced::clipboard::Kind::Standard)
                        {
                            shell.publish((self.on_action)(EditorAction::Paste(text)));
                        }
                    }
                    keyboard::Key::Character(c)
                        if cmd && shift && c.as_str() == "z" && !self.read_only =>
                    {
                        shell.publish((self.on_action)(EditorAction::Redo));
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "z" && !self.read_only => {
                        shell.publish((self.on_action)(EditorAction::Undo));
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "s" && !self.read_only => {
                        shell.publish((self.on_action)(EditorAction::SaveRequested));
                    }
                    _ if !self.read_only
                        && !cmd
                        && !modifiers.control()
                        && key_text
                            .as_ref()
                            .is_some_and(|t| t.chars().any(|c| !c.is_control())) =>
                    {
                        if let Some(txt) = key_text {
                            for ch in txt.chars() {
                                if !ch.is_control() {
                                    shell.publish((self.on_action)(EditorAction::Insert(ch)));
                                }
                            }
                        }
                    }
                    _ => {
                        handled = false;
                    }
                }

                // Mark events we consumed as captured so app-level keyboard
                // handlers (agent chat, etc.) don't also react to them.
                if handled {
                    // A consumed key may have moved the caret; arm the
                    // caret-follow (capped input only) so next frame nudges
                    // scroll if the caret left view.
                    if self.max_rows.is_some() {
                        internal.follow_after_key = true;
                        shell.request_redraw();
                    }
                    shell.capture_event();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                // Self-driven auto-scroll loop: step the drag once per distinct
                // frame instant (iced re-dispatches the same instant several
                // times per real frame), then re-request a redraw on *every*
                // dispatch while auto-scrolling so the loop keeps spinning with
                // the mouse held still.
                if internal.dragging
                    && internal.focused
                    && let Some(pos) = cursor.land().position()
                {
                    if internal.last_autoscroll_frame != Some(*now) {
                        internal.last_autoscroll_frame = Some(*now);
                        internal.autoscrolling = self.drag_frame(
                            pos,
                            bounds,
                            *viewport,
                            internal,
                            wrap.as_ref(),
                            hybrid.as_ref(),
                            shell,
                        );
                    }
                    if internal.autoscrolling {
                        shell.request_redraw();
                    }
                } else {
                    internal.autoscrolling = false;
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        if !cursor.is_over(bounds) {
            return mouse::Interaction::default();
        }
        let internal = tree.state.downcast_ref::<InternalState>();
        if internal.link_hover.is_some() {
            return mouse::Interaction::Pointer;
        }
        mouse::Interaction::Text
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: adv_mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let internal = tree.state.downcast_ref::<InternalState>();
        let link_hover = internal.link_hover.as_ref();
        let cell_w = if internal.cell_width > 0.0 {
            internal.cell_width
        } else {
            7.8
        };
        let gutter_w = if self.show_gutter {
            let digits = digit_count(self.state.line_count());
            (digits as f32) * cell_w + GUTTER_PAD * 2.0
        } else {
            0.0
        };
        let content_x = bounds.x + gutter_w;
        let content_w = bounds.width - gutter_w;

        // Compute wrap / hybrid layout if needed (hybrid path is cached).
        let cpr = pane_chars_for(content_w, cell_w);
        let hybrid = if self.md_tables {
            Some(cached_hybrid_layout(
                internal,
                &self.state.lines,
                self.state.highlight_version,
                cpr,
                self.word_wrap,
            ))
        } else {
            None
        };
        let wrap = if self.md_tables {
            None
        } else if self.word_wrap {
            Some(WrapLayout::compute(&self.state.lines, cpr))
        } else {
            None
        };
        let total_visual_rows = if let Some(ref ed) = hybrid {
            ed.total_visual_rows
        } else {
            wrap.as_ref()
                .map_or(self.state.line_count(), |w| w.total_visual_rows)
        };
        let content_height = total_visual_rows as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0;

        // Resolve the vertical offset — the user/keyboard-driven `scroll_y`
        // clamped to range (see `resolved_scroll_y`).
        let scroll_y = self.resolved_scroll_y(bounds.height, content_height);

        // Horizontal scroll: classic word-wrap locks x; hybrid tables may
        // overflow at MIN_COL and enable scroll_x even with wrap on.
        let total_content_w_chars = if let Some(ref ed) = hybrid {
            ed.content_width_chars
        } else if self.word_wrap {
            0
        } else {
            self.state
                .lines
                .iter()
                .map(|l| l.chars().count())
                .max()
                .unwrap_or(0)
        };
        let scroll_x = if total_content_w_chars == 0 {
            0.0
        } else {
            let total_content_w = total_content_w_chars as f32 * cell_w + CONTENT_PAD * 2.0;
            let max_scroll_x = (total_content_w - content_w).max(0.0);
            self.state.scroll_x.clamp(0.0, max_scroll_x)
        };

        // Clip to the intersection of layout bounds and the visible viewport.
        // If the widget is fully outside the viewport (e.g. scrolled well
        // past), skip drawing entirely so `fill_text`/`fill_quad` can't bleed
        // beyond the scrollable.
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        renderer.with_layer(clip, |renderer: &mut iced::Renderer| {
            // Background.
            if !self.transparent_bg {
                renderer::Renderer::fill_quad(
                    renderer,
                    renderer::Quad {
                        bounds,
                        border: Border::default(),
                        ..renderer::Quad::default()
                    },
                    theme::bg_base(),
                );
            }

            // Placeholder: drawn when the editor is empty and a placeholder
            // was configured. Rendered before content, so the cursor still
            // paints on top.
            let is_empty = self.state.lines.len() <= 1
                && self.state.lines.first().is_none_or(|l| l.is_empty());
            if is_empty
                && let Some(ph) = self.placeholder.as_ref()
                && !ph.is_empty()
            {
                let px = content_x + CONTENT_PAD - scroll_x;
                let py = bounds.y + CONTENT_PAD_Y;
                renderer.fill_text(
                    iced::advanced::Text {
                        content: ph.clone(),
                        bounds: Size::new(content_w, LINE_HEIGHT),
                        size: Pixels(font_size()),
                        line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                        font: theme::content_font(),
                        align_x: alignment::Horizontal::Left.into(),
                        align_y: alignment::Vertical::Top,
                        shaping: text::Shaping::Basic,
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(px, py),
                    theme::text_muted(),
                    clip,
                );
            }

            let first_vrow = (scroll_y / LINE_HEIGHT).floor() as usize;
            let visible_vrows = (bounds.height / LINE_HEIGHT).ceil() as usize + 1;
            let last_vrow = (first_vrow + visible_vrows).min(total_visual_rows);

            let selection = self.state.selection_range();
            let has_blocks = !self.state.blocks.is_empty();

            // Content area clipping rectangle (excludes gutter). Intersected
            // with the scrollable's visible viewport so tall editors scrolled
            // partially off-screen can't render text past the scrollable's
            // bounds (fill_text's own clip is not hierarchical with
            // with_layer for text rendering).
            let content_clip = Rectangle {
                x: content_x,
                y: bounds.y,
                width: content_w,
                height: bounds.height,
            }
            .intersection(&clip)
            .unwrap_or(clip);

            for vrow in first_vrow..last_vrow {
                let y = bounds.y + CONTENT_PAD_Y + (vrow as f32) * LINE_HEIGHT - scroll_y;

                // Map visual row to logical line + sub-row (+ prose char range).
                let (line_idx, sub_row, char_start, char_end, table_paint) =
                    if let Some(ref ed) = hybrid {
                        let (li, sr) = ed.visual_to_logical(vrow);
                        match ed.line_kind.get(li) {
                            Some(LineLayoutKind::TableRow { region, row }) => {
                                (li, sr, 0, 0, Some((*region, *row)))
                            }
                            Some(LineLayoutKind::TableSeparator { .. }) => {
                                // Zero-height lines should not appear as visual rows.
                                continue;
                            }
                            _ => {
                                let starts = ed
                                    .prose_row_starts
                                    .get(li)
                                    .map(|s| s.as_slice())
                                    .unwrap_or(&[0]);
                                let cs = starts.get(sr).copied().unwrap_or(0);
                                let ce = if sr + 1 < starts.len() {
                                    starts[sr + 1]
                                } else {
                                    self.state.lines.get(li).map(|l| l.chars().count()).unwrap_or(0)
                                };
                                (li, sr, cs, ce, None)
                            }
                        }
                    } else if let Some(ref w) = wrap {
                        let (li, sr) = w.visual_to_logical(vrow);
                        let starts = &w.row_starts[li];
                        let cs = starts[sr];
                        let ce = if sr + 1 < starts.len() {
                            starts[sr + 1]
                        } else {
                            self.state.lines[li].chars().count()
                        };
                        (li, sr, cs, ce, None)
                    } else {
                        let len = self.state.lines[vrow].chars().count();
                        (vrow, 0, 0, len, None)
                    };

                // Table band: row bg → match highlights → selection → cell
                // text → rules → link underline (no `|`).
                if let (Some(ref ed), Some((region_i, row_i))) = (hybrid.as_ref(), table_paint) {
                    if let Some(region) = ed.tables.regions.get(region_i)
                        && let Some(trow) = region.rows.get(row_i)
                    {
                        let table_w =
                            region.total_width_chars as f32 * cell_w;
                        let table_x = content_x + CONTENT_PAD - scroll_x;

                        // 1. Row background from role (header / zebra).
                        if let Some(bg) = table_row_bg(trow.role) {
                            renderer::Renderer::fill_quad(
                                renderer,
                                renderer::Quad {
                                    bounds: Rectangle {
                                        x: table_x,
                                        y,
                                        width: table_w.max(cell_w),
                                        height: LINE_HEIGHT,
                                    },
                                    border: Border::default(),
                                    ..renderer::Quad::default()
                                },
                                bg,
                            );
                        }

                        // 2. Fragment-clipped find/search match highlights.
                        paint_table_highlights(
                            renderer,
                            trow,
                            region,
                            sub_row,
                            line_idx,
                            &self.highlight_ranges,
                            self.current_highlight.as_ref(),
                            table_x,
                            y,
                            cell_w,
                        );

                        // 3. Fragment-clipped selection quads.
                        if let Some((sel_start, sel_end)) = selection {
                            paint_table_selection(
                                renderer,
                                trow,
                                region,
                                sub_row,
                                line_idx,
                                sel_start,
                                sel_end,
                                table_x,
                                y,
                                cell_w,
                            );
                        }

                        // 4. Cell fragment text (aligned; no pipe glyphs).
                        for (ci, cell) in trow.cells.iter().enumerate() {
                            let Some(frag) = cell.fragments.get(sub_row) else {
                                continue;
                            };
                            let frag_len = frag.char_end.saturating_sub(frag.char_start);
                            if frag_len == 0 {
                                continue;
                            }
                            let text: String = cell
                                .text
                                .chars()
                                .skip(frag.char_start)
                                .take(frag_len)
                                .collect();
                            if text.is_empty() {
                                continue;
                            }
                            let col_w = region.col_widths.get(ci).copied().unwrap_or(0);
                            let align = region
                                .aligns
                                .get(ci)
                                .copied()
                                .unwrap_or(md_table::ColAlign::Left);
                            let pad = md_table::align_pad(align, col_w, frag_len);
                            let origin = md_table::col_origin(&region.col_widths, ci);
                            let tx = table_x + (origin + pad) as f32 * cell_w;
                            let tw = frag_len as f32 * cell_w;
                            renderer.fill_text(
                                iced::advanced::Text {
                                    content: text,
                                    bounds: Size::new(tw + cell_w, LINE_HEIGHT),
                                    size: Pixels(font_size()),
                                    line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                                    font: theme::content_font(),
                                    align_x: alignment::Horizontal::Left.into(),
                                    align_y: alignment::Vertical::Top,
                                    shaping: text::Shaping::Basic,
                                    wrapping: text::Wrapping::None,
                                },
                                Point::new(tx, y),
                                theme::text_primary(),
                                content_clip,
                            );
                        }

                        // 5. Column and row rules at cell edges.
                        paint_table_rules(
                            renderer,
                            region,
                            trow,
                            row_i,
                            sub_row,
                            table_x,
                            y,
                            cell_w,
                            table_w,
                        );

                        // 6. Cmd-hover link underline (same segments as prose).
                        if let Some(hover) = link_hover {
                            paint_table_link_underline(
                                renderer,
                                trow,
                                region,
                                sub_row,
                                line_idx,
                                &self.state.lines[line_idx],
                                hover,
                                table_x,
                                y,
                                cell_w,
                            );
                        }
                    }
                    continue;
                }

                // Block background.
                if has_blocks
                    && let Some(info) = self.state.block_line_map.get(line_idx)
                    && let Some(block) = self.state.blocks.get(info.block_idx)
                {
                    let bg = block_kind_bg(block.kind);
                    renderer::Renderer::fill_quad(
                        renderer,
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x,
                                y,
                                width: bounds.width,
                                height: LINE_HEIGHT,
                            },
                            border: Border::default(),
                            ..renderer::Quad::default()
                        },
                        bg,
                    );
                }

                // Per-line background (e.g. diff added/removed). Resolve the
                // color here so theme toggles take effect without rebuild.
                if let Some(Some(kind)) = self.state.line_backgrounds.get(line_idx) {
                    renderer::Renderer::fill_quad(
                        renderer,
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x,
                                y,
                                width: bounds.width,
                                height: LINE_HEIGHT,
                            },
                            border: Border::default(),
                            ..renderer::Quad::default()
                        },
                        line_bg_color(*kind),
                    );
                }

                // Match-range highlights (find feature). Drawn behind the
                // selection so an active selection over a match still reads
                // as selected. Two passes: dim background for every match,
                // then a stronger accent fill on top for the current
                // candidate so the user can tell prev/next apart.
                let line = &self.state.lines[line_idx];
                let line_char_count = line.chars().count();
                let draw_match = |renderer: &mut iced::Renderer,
                                  range: &HighlightRange,
                                  bg: Color,
                                  border: Option<Color>| {
                    if range.line != line_idx {
                        return;
                    }
                    let safe_start = snap_byte_boundary(line, range.byte_start);
                    let safe_end = snap_byte_boundary(line, range.byte_end);
                    if safe_end <= safe_start {
                        return;
                    }
                    let char_start_abs = line[..safe_start].chars().count();
                    let char_end_abs = line[..safe_end].chars().count();
                    let vis_lo = char_start_abs.max(char_start);
                    let vis_hi = char_end_abs.min(char_end);
                    if vis_hi <= vis_lo {
                        return;
                    }
                    let rel_start = vis_lo - char_start;
                    let rel_end = vis_hi - char_start;
                    let rel_end = rel_end.min(line_char_count.saturating_sub(char_start));
                    if rel_end <= rel_start {
                        return;
                    }
                    let mx = content_x + CONTENT_PAD + rel_start as f32 * cell_w - scroll_x;
                    let mw = (rel_end - rel_start) as f32 * cell_w;
                    let border = border
                        .map(|c| Border {
                            color: c,
                            width: 1.0,
                            radius: 2.0.into(),
                        })
                        .unwrap_or_default();
                    renderer::Renderer::fill_quad(
                        renderer,
                        renderer::Quad {
                            bounds: Rectangle {
                                x: mx,
                                y,
                                width: mw,
                                height: LINE_HEIGHT,
                            },
                            border,
                            ..renderer::Quad::default()
                        },
                        bg,
                    );
                };
                for range in self.highlight_ranges.iter() {
                    draw_match(renderer, range, theme::search_match_bg(), None);
                }
                if let Some(cur) = self.current_highlight.as_ref() {
                    draw_match(
                        renderer,
                        cur,
                        Color {
                            a: 0.55,
                            ..theme::accent()
                        },
                        Some(theme::accent()),
                    );
                }

                // Selection highlight.
                if let Some((sel_start, sel_end)) = selection
                    && line_idx >= sel_start.line
                    && line_idx <= sel_end.line
                {
                    let abs_col_start = char_start;
                    let abs_col_end = char_end;
                    let sel_col_start = if line_idx == sel_start.line {
                        sel_start.col.max(abs_col_start)
                    } else {
                        abs_col_start
                    };
                    let sel_col_end = if line_idx == sel_end.line {
                        sel_end.col.min(abs_col_end)
                    } else {
                        abs_col_end
                    };
                    if sel_col_start < sel_col_end
                        && sel_col_start < abs_col_end
                        && sel_col_end > abs_col_start
                    {
                        let vis_start = sel_col_start.saturating_sub(abs_col_start);
                        let vis_end = sel_col_end.saturating_sub(abs_col_start);
                        let sel_x = content_x + CONTENT_PAD + vis_start as f32 * cell_w - scroll_x;
                        let sel_w = (vis_end - vis_start) as f32 * cell_w;
                        renderer::Renderer::fill_quad(
                            renderer,
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: sel_x,
                                    y,
                                    width: sel_w,
                                    height: LINE_HEIGHT,
                                },
                                border: Border::default(),
                                ..renderer::Quad::default()
                            },
                            Color {
                                a: 0.3,
                                ..theme::accent()
                            },
                        );
                    }
                }

                // Extract the sub-string for this visual row.
                let line = &self.state.lines[line_idx];
                let row_text: String = line
                    .chars()
                    .skip(char_start)
                    .take(char_end - char_start)
                    .collect();

                if !row_text.is_empty() {
                    // Block header/more lines get special coloring (only on first sub-row).
                    let block_override_color = if has_blocks && sub_row == 0 {
                        self.state.block_line_map.get(line_idx).and_then(|info| {
                            if !info.is_header {
                                return None;
                            }
                            let block = self.state.blocks.get(info.block_idx)?;
                            Some(block_header_color(block.kind))
                        })
                    } else {
                        None
                    };

                    if let Some(color) = block_override_color {
                        renderer.fill_text(
                            iced::advanced::Text {
                                content: row_text,
                                bounds: Size::new(content_w, LINE_HEIGHT),
                                size: Pixels(font_size()),
                                line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                                font: theme::content_font(),
                                align_x: alignment::Horizontal::Left.into(),
                                align_y: alignment::Vertical::Top,
                                shaping: text::Shaping::Basic,
                                wrapping: text::Wrapping::None,
                            },
                            Point::new(content_x + CONTENT_PAD - scroll_x, y),
                            color,
                            content_clip,
                        );
                    } else {
                        // Syntax highlighting spans — need to slice for this visual row.
                        // Text is sliced from the CURRENT line (`row_text`),
                        // not from `span.text`, so that stale spans left over
                        // from the pre-edit highlight still render the
                        // current characters. Colors can be slightly
                        // misaligned near the edit point until the async
                        // re-highlight completes, but the visible content
                        // always matches the buffer.
                        let spans = self
                            .state
                            .highlight_spans
                            .as_ref()
                            .and_then(|cache| cache.get(line_idx));

                        if let Some(spans) = spans {
                            let row_chars = row_text.chars().count();
                            let mut col = 0usize;
                            for span in spans {
                                let span_chars = span.text.chars().count();
                                let span_end = col + span_chars;
                                if span_end > char_start && col < char_end {
                                    let vis_start = col.max(char_start) - char_start;
                                    let vis_end = span_end.min(char_end) - char_start;
                                    let take_end = vis_end.min(row_chars);
                                    if take_end > vis_start {
                                        let slice: String = row_text
                                            .chars()
                                            .skip(vis_start)
                                            .take(take_end - vis_start)
                                            .collect();
                                        if !slice.is_empty() {
                                            let sw = slice.chars().count() as f32 * cell_w;
                                            let sx =
                                                content_x + CONTENT_PAD + vis_start as f32 * cell_w
                                                    - scroll_x;
                                            renderer.fill_text(
                                                iced::advanced::Text {
                                                    content: slice,
                                                    bounds: Size::new(sw + cell_w, LINE_HEIGHT),
                                                    size: Pixels(font_size()),
                                                    line_height: text::LineHeight::Absolute(
                                                        Pixels(LINE_HEIGHT),
                                                    ),
                                                    font: theme::content_font(),
                                                    align_x: alignment::Horizontal::Left.into(),
                                                    align_y: alignment::Vertical::Top,
                                                    shaping: text::Shaping::Basic,
                                                    wrapping: text::Wrapping::None,
                                                },
                                                Point::new(sx, y),
                                                span.color,
                                                content_clip,
                                            );
                                        }
                                    }
                                }
                                col = span_end;
                            }
                            // Any chars the user has typed past where the
                            // stale spans ended get a default-color paint so
                            // inserts at end-of-line aren't invisible.
                            if row_chars > col.saturating_sub(char_start) {
                                let tail_start = col.saturating_sub(char_start);
                                let tail_start = tail_start.min(row_chars);
                                let tail: String = row_text.chars().skip(tail_start).collect();
                                if !tail.is_empty() {
                                    let tw = tail.chars().count() as f32 * cell_w;
                                    let tx = content_x + CONTENT_PAD + tail_start as f32 * cell_w
                                        - scroll_x;
                                    renderer.fill_text(
                                        iced::advanced::Text {
                                            content: tail,
                                            bounds: Size::new(tw + cell_w, LINE_HEIGHT),
                                            size: Pixels(font_size()),
                                            line_height: text::LineHeight::Absolute(Pixels(
                                                LINE_HEIGHT,
                                            )),
                                            font: theme::content_font(),
                                            align_x: alignment::Horizontal::Left.into(),
                                            align_y: alignment::Vertical::Top,
                                            shaping: text::Shaping::Basic,
                                            wrapping: text::Wrapping::None,
                                        },
                                        Point::new(tx, y),
                                        theme::text_primary(),
                                        content_clip,
                                    );
                                }
                            }
                        } else {
                            renderer.fill_text(
                                iced::advanced::Text {
                                    content: row_text,
                                    bounds: Size::new(content_w, LINE_HEIGHT),
                                    size: Pixels(font_size()),
                                    line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                                    font: theme::content_font(),
                                    align_x: alignment::Horizontal::Left.into(),
                                    align_y: alignment::Vertical::Top,
                                    shaping: text::Shaping::Basic,
                                    wrapping: text::Wrapping::None,
                                },
                                Point::new(content_x + CONTENT_PAD - scroll_x, y),
                                theme::text_primary(),
                                content_clip,
                            );
                        }
                    }
                }

                // Link hover underline — drawn per visual row so a wrapped URL
                // gets a segmented underline that follows the wrap.
                if let Some(hover) = link_hover
                    && hover.line == line_idx
                    && hover.char_start < char_end
                    && hover.char_end > char_start
                {
                    let vis_start = hover.char_start.max(char_start) - char_start;
                    let vis_end = hover.char_end.min(char_end) - char_start;
                    let ux = content_x + CONTENT_PAD + vis_start as f32 * cell_w - scroll_x;
                    let uw = (vis_end - vis_start) as f32 * cell_w;
                    let uy = y + LINE_HEIGHT - 2.0;
                    // Solid underline = direct open (URL or resolved path);
                    // dashed = unresolved path that opens the fuzzy finder.
                    for (dx, dw) in path_link::underline_segments(uw, hover.target.opens_directly())
                    {
                        renderer::Renderer::fill_quad(
                            renderer,
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: ux + dx,
                                    y: uy,
                                    width: dw,
                                    height: 1.0,
                                },
                                border: Border::default(),
                                ..renderer::Quad::default()
                            },
                            theme::accent(),
                        );
                    }
                }
            }

            // Cursor — paint when the widget owns focus, OR when a find
            // candidate is currently highlighted in an *editable* editor.
            // Read-only editors (chat blocks) suppress the decoration
            // cursor: per-block highlights mark the match, but a caret
            // would suggest editability that doesn't exist.
            let force_cursor = self.current_highlight.is_some() && !self.read_only;
            if internal.focused || force_cursor {
                let cursor_line = self.state.cursor.line;
                let line_str = &self.state.lines[cursor_line];
                let byte_col = self.state.cursor.col.min(line_str.len());
                let char_col = line_str[..byte_col].chars().count();
                let (cy, cx) = if let Some(ref ed) = hybrid {
                    let (vrow, col_in_row) = cursor_visual_pos_hybrid(self.state, ed);
                    (
                        bounds.y + CONTENT_PAD_Y + vrow as f32 * LINE_HEIGHT - scroll_y,
                        content_x + CONTENT_PAD + col_in_row as f32 * cell_w - scroll_x,
                    )
                } else if let Some(ref w) = wrap {
                    let (vrow, col_in_row) = cursor_visual_pos(self.state, w);
                    (
                        bounds.y + CONTENT_PAD_Y + vrow as f32 * LINE_HEIGHT - scroll_y,
                        content_x + CONTENT_PAD + col_in_row as f32 * cell_w,
                    )
                } else {
                    (
                        bounds.y + CONTENT_PAD_Y + cursor_line as f32 * LINE_HEIGHT - scroll_y,
                        content_x + CONTENT_PAD + char_col as f32 * cell_w - scroll_x,
                    )
                };
                renderer::Renderer::fill_quad(
                    renderer,
                    renderer::Quad {
                        bounds: Rectangle {
                            x: cx,
                            y: cy,
                            width: 2.0,
                            height: LINE_HEIGHT,
                        },
                        border: Border::default(),
                        ..renderer::Quad::default()
                    },
                    theme::accent(),
                );
            }

            // Gutter overlay — drawn last so it covers horizontally-scrolled content.
            if self.show_gutter {
                renderer::Renderer::fill_quad(
                    renderer,
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x,
                            y: bounds.y,
                            width: gutter_w,
                            height: bounds.height,
                        },
                        border: Border::default(),
                        ..renderer::Quad::default()
                    },
                    theme::bg_surface(),
                );

                for vrow in first_vrow..last_vrow {
                    let y = bounds.y + CONTENT_PAD_Y + (vrow as f32) * LINE_HEIGHT - scroll_y;
                    let (line_idx, sub_row) = if let Some(ref ed) = hybrid {
                        ed.visual_to_logical(vrow)
                    } else if let Some(ref w) = wrap {
                        w.visual_to_logical(vrow)
                    } else {
                        (vrow, 0)
                    };

                    if sub_row == 0 {
                        let digits = digit_count(self.state.line_count());
                        let line_num = format!("{:>width$} ", line_idx + 1, width = digits);
                        let num_color = if line_idx == self.state.cursor.line && internal.focused {
                            theme::text_secondary()
                        } else {
                            theme::text_muted()
                        };
                        renderer.fill_text(
                            iced::advanced::Text {
                                content: line_num,
                                bounds: Size::new(gutter_w, LINE_HEIGHT),
                                size: Pixels(font_size()),
                                line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                                font: theme::content_font(),
                                align_x: alignment::Horizontal::Left.into(),
                                align_y: alignment::Vertical::Top,
                                shaping: text::Shaping::Basic,
                                wrapping: text::Wrapping::None,
                            },
                            Point::new(bounds.x + GUTTER_PAD, y),
                            num_color,
                            clip,
                        );
                    }
                }
            }

            // Scrollbars — thin overlaid indicators matching the list-column
            // rail. Skipped in plain `fit_content` mode because such editors
            // never overflow internally; their parent scrollable handles
            // scrolling. A capped auto-grow input (`max_rows`) is the
            // exception — it does overflow internally, so it gets the rail.
            // Also skipped in `static_viewport` mode (search-stack slices).
            if (!self.fit_content || self.max_rows.is_some()) && !self.static_viewport {
                let scroller_color = theme::text_muted();

                if content_height > bounds.height && bounds.height > 0.0 {
                    let track_h = bounds.height;
                    let ratio = (track_h / content_height).clamp(0.0, 1.0);
                    let scroller_h = (track_h * ratio).max(SCROLLBAR_MIN_SCROLLER).min(track_h);
                    let max_scroll_y = content_height - track_h;
                    let t = if max_scroll_y > 0.0 {
                        (scroll_y / max_scroll_y).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let scroller_y = bounds.y + (track_h - scroller_h) * t;
                    renderer::Renderer::fill_quad(
                        renderer,
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x + bounds.width - SCROLLBAR_WIDTH,
                                y: scroller_y,
                                width: SCROLLBAR_WIDTH,
                                height: scroller_h,
                            },
                            border: Border {
                                radius: SCROLLBAR_RADIUS.into(),
                                ..Border::default()
                            },
                            ..renderer::Quad::default()
                        },
                        scroller_color,
                    );
                }

                // Horizontal scrollbar when content is wider than the pane
                // (unwrapped lines, or hybrid tables that overflow at MIN_COL).
                let h_scroll_chars = if let Some(ref ed) = hybrid {
                    if ed.content_width_chars > ed.pane_chars {
                        ed.content_width_chars
                    } else if !self.word_wrap {
                        ed.content_width_chars
                    } else {
                        0
                    }
                } else if !self.word_wrap {
                    self.state
                        .lines
                        .iter()
                        .map(|l| l.chars().count())
                        .max()
                        .unwrap_or(0)
                } else {
                    0
                };
                if h_scroll_chars > 0 && content_w > 0.0 {
                    let total_content_w = h_scroll_chars as f32 * cell_w + CONTENT_PAD * 2.0;
                    if total_content_w > content_w {
                        let track_w = content_w;
                        let ratio = (track_w / total_content_w).clamp(0.0, 1.0);
                        let scroller_w = (track_w * ratio).max(SCROLLBAR_MIN_SCROLLER).min(track_w);
                        let max_scroll_x = total_content_w - track_w;
                        let t = if max_scroll_x > 0.0 {
                            (scroll_x / max_scroll_x).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let scroller_x = content_x + (track_w - scroller_w) * t;
                        renderer::Renderer::fill_quad(
                            renderer,
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: scroller_x,
                                    y: bounds.y + bounds.height - SCROLLBAR_WIDTH,
                                    width: scroller_w,
                                    height: SCROLLBAR_WIDTH,
                                },
                                border: Border {
                                    radius: SCROLLBAR_RADIUS.into(),
                                    ..Border::default()
                                },
                                ..renderer::Quad::default()
                            },
                            scroller_color,
                        );
                    }
                }
            }
        });
    }
}

impl<'a, M: Clone + 'a> From<TextEdit<'a, M>> for Element<'a, M> {
    fn from(edit: TextEdit<'a, M>) -> Self {
        Self::new(edit)
    }
}

// ── Helper functions ───────────────────────────────────────────────────────

/// Background fill for a table row role. Header is stronger; zebra body rows
/// alternate a light elevated tint; non-zebra body is transparent.
fn table_row_bg(role: md_table::RowRole) -> Option<Color> {
    match role {
        md_table::RowRole::Header => {
            let mut c = theme::bg_section_header();
            c.a = 0.85;
            Some(c)
        }
        md_table::RowRole::Body { zebra: true } => {
            let mut c = theme::bg_elevated();
            c.a = 0.45;
            Some(c)
        }
        md_table::RowRole::Body { zebra: false } => None,
    }
}

/// Char spans (from table left, width) for fragments on this visual sub-row
/// whose source bytes intersect `[line_byte_start, line_byte_end)`.
///
/// Shared by selection, find-match highlights, and link underlines so paint
/// and hit-test geometry stay aligned.
fn table_byte_range_spans(
    trow: &md_table::TableRow,
    region: &md_table::TableRegion,
    sub_row: usize,
    line_byte_start: usize,
    line_byte_end: usize,
) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    if line_byte_end <= line_byte_start {
        return spans;
    }
    for (ci, cell) in trow.cells.iter().enumerate() {
        let Some(frag) = cell.fragments.get(sub_row) else {
            continue;
        };
        let frag_len = frag.char_end.saturating_sub(frag.char_start);
        // Empty fragment still can hold a zero-width caret, but no fill.
        if frag_len == 0 {
            continue;
        }

        // Byte range of this fragment inside the source line.
        let frag_byte_start = cell.source_byte.start
            + cell
                .text
                .char_indices()
                .nth(frag.char_start)
                .map(|(b, _)| b)
                .unwrap_or(0);
        let frag_byte_end = cell.source_byte.start
            + cell
                .text
                .char_indices()
                .nth(frag.char_end)
                .map(|(b, _)| b)
                .unwrap_or(cell.text.len());

        let ov_start = frag_byte_start.max(line_byte_start);
        let ov_end = frag_byte_end.min(line_byte_end);
        if ov_end <= ov_start {
            continue;
        }

        // Map overlapping bytes back to char offsets within the fragment.
        let cell_text = &cell.text;
        let lo_byte = snap_byte_boundary(
            cell_text,
            ov_start.saturating_sub(cell.source_byte.start),
        );
        let hi_byte = snap_byte_boundary(
            cell_text,
            ov_end.saturating_sub(cell.source_byte.start),
        );
        let char_lo = cell_text[..lo_byte]
            .chars()
            .count()
            .saturating_sub(frag.char_start);
        let char_hi = cell_text[..hi_byte]
            .chars()
            .count()
            .saturating_sub(frag.char_start)
            .min(frag_len);
        if char_hi <= char_lo {
            continue;
        }

        let col_w = region.col_widths.get(ci).copied().unwrap_or(0);
        let align = region
            .aligns
            .get(ci)
            .copied()
            .unwrap_or(md_table::ColAlign::Left);
        let pad = md_table::align_pad(align, col_w, frag_len);
        let origin = md_table::col_origin(&region.col_widths, ci);
        spans.push((origin + pad + char_lo, char_hi - char_lo));
    }
    spans
}

/// Paint selection as quads clipped to the cell fragment(s) on this visual
/// sub-row that intersect the source selection.
fn paint_table_selection(
    renderer: &mut iced::Renderer,
    trow: &md_table::TableRow,
    region: &md_table::TableRegion,
    sub_row: usize,
    line_idx: usize,
    sel_start: Pos,
    sel_end: Pos,
    table_x: f32,
    y: f32,
    cell_w: f32,
) {
    // Line fully outside the ordered selection range → nothing.
    if line_idx < sel_start.line || line_idx > sel_end.line {
        return;
    }
    let line_sel_start = if line_idx == sel_start.line {
        sel_start.col
    } else {
        0
    };
    let line_sel_end = if line_idx == sel_end.line {
        sel_end.col
    } else {
        // Past end of cell text is enough; use a large sentinel so full
        // intermediate lines select every fragment.
        usize::MAX
    };
    let sel_color = Color {
        a: 0.3,
        ..theme::accent()
    };
    for (x_chars, w_chars) in
        table_byte_range_spans(trow, region, sub_row, line_sel_start, line_sel_end)
    {
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds: Rectangle {
                    x: table_x + x_chars as f32 * cell_w,
                    y,
                    width: w_chars as f32 * cell_w,
                    height: LINE_HEIGHT,
                },
                border: Border::default(),
                ..renderer::Quad::default()
            },
            sel_color,
        );
    }
}

/// Paint find/search match quads clipped to cell fragments on this visual
/// sub-row. Mirrors the prose path: muted bg for every match, stronger accent
/// fill (+ optional border) for the current candidate.
fn paint_table_highlights(
    renderer: &mut iced::Renderer,
    trow: &md_table::TableRow,
    region: &md_table::TableRegion,
    sub_row: usize,
    line_idx: usize,
    ranges: &[HighlightRange],
    current: Option<&HighlightRange>,
    table_x: f32,
    y: f32,
    cell_w: f32,
) {
    let paint_range = |renderer: &mut iced::Renderer,
                       range: &HighlightRange,
                       bg: Color,
                       border: Option<Color>| {
        if range.line != line_idx {
            return;
        }
        let border = border
            .map(|c| Border {
                color: c,
                width: 1.0,
                radius: 2.0.into(),
            })
            .unwrap_or_default();
        for (x_chars, w_chars) in
            table_byte_range_spans(trow, region, sub_row, range.byte_start, range.byte_end)
        {
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: Rectangle {
                        x: table_x + x_chars as f32 * cell_w,
                        y,
                        width: w_chars as f32 * cell_w,
                        height: LINE_HEIGHT,
                    },
                    border,
                    ..renderer::Quad::default()
                },
                bg,
            );
        }
    };

    for range in ranges {
        paint_range(renderer, range, theme::search_match_bg(), None);
    }
    if let Some(cur) = current {
        paint_range(
            renderer,
            cur,
            Color {
                a: 0.55,
                ..theme::accent()
            },
            Some(theme::accent()),
        );
    }
}

/// Draw cmd-hover link underline under fragment slices that overlap the
/// hovered link's char range on this visual row.
fn paint_table_link_underline(
    renderer: &mut iced::Renderer,
    trow: &md_table::TableRow,
    region: &md_table::TableRegion,
    sub_row: usize,
    line_idx: usize,
    line: &str,
    hover: &LinkHover,
    table_x: f32,
    y: f32,
    cell_w: f32,
) {
    if hover.line != line_idx || hover.char_end <= hover.char_start {
        return;
    }
    // LinkHover stores char offsets; convert to line-local bytes so we share
    // the same fragment intersection as selection/highlights.
    let byte_start = line
        .char_indices()
        .nth(hover.char_start)
        .map(|(b, _)| b)
        .unwrap_or(line.len());
    let byte_end = line
        .char_indices()
        .nth(hover.char_end)
        .map(|(b, _)| b)
        .unwrap_or(line.len());
    if byte_end <= byte_start {
        return;
    }

    let uy = y + LINE_HEIGHT - 2.0;
    for (x_chars, w_chars) in table_byte_range_spans(trow, region, sub_row, byte_start, byte_end)
    {
        let ux = table_x + x_chars as f32 * cell_w;
        let uw = w_chars as f32 * cell_w;
        // Solid underline = direct open (URL or resolved path);
        // dashed = unresolved path that opens the fuzzy finder.
        for (dx, dw) in path_link::underline_segments(uw, hover.target.opens_directly()) {
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: Rectangle {
                        x: ux + dx,
                        y: uy,
                        width: dw,
                        height: 1.0,
                    },
                    border: Border::default(),
                    ..renderer::Quad::default()
                },
                theme::accent(),
            );
        }
    }
}

/// Draw vertical column rules and a horizontal bottom edge for the current
/// visual band of a table row.
fn paint_table_rules(
    renderer: &mut iced::Renderer,
    region: &md_table::TableRegion,
    trow: &md_table::TableRow,
    row_i: usize,
    sub_row: usize,
    table_x: f32,
    y: f32,
    cell_w: f32,
    table_w: f32,
) {
    let mut rule = theme::border_color();
    rule.a = 0.55;

    // Vertical rules at every column origin and the table right edge.
    let mut xs = Vec::with_capacity(region.col_widths.len() + 1);
    for ci in 0..region.col_widths.len() {
        xs.push(md_table::col_origin(&region.col_widths, ci));
    }
    xs.push(region.total_width_chars);

    for x_chars in xs {
        let rx = table_x + x_chars as f32 * cell_w;
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds: Rectangle {
                    x: rx,
                    y,
                    width: 1.0,
                    height: LINE_HEIGHT,
                },
                border: Border::default(),
                ..renderer::Quad::default()
            },
            rule,
        );
    }

    // Horizontal rule under the header band and between body rows (on the
    // last visual fragment of each logical row so multi-line cells get one
    // bottom edge).
    let is_last_frag = sub_row + 1 >= trow.height;
    let draw_h = matches!(trow.role, md_table::RowRole::Header) || is_last_frag;
    // Always draw a top edge on the first visual row of the first data row
    // (after header) is covered by header's bottom; first row of region
    // (header) gets a top edge.
    let is_first_band = row_i == 0 && sub_row == 0;
    if is_first_band {
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds: Rectangle {
                    x: table_x,
                    y,
                    width: table_w.max(1.0),
                    height: 1.0,
                },
                border: Border::default(),
                ..renderer::Quad::default()
            },
            rule,
        );
    }
    if draw_h {
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds: Rectangle {
                    x: table_x,
                    y: y + LINE_HEIGHT - 1.0,
                    width: table_w.max(1.0),
                    height: 1.0,
                },
                border: Border::default(),
                ..renderer::Quad::default()
            },
            rule,
        );
    }
}

/// Measure the width of a single monospace character using cosmic-text.
fn measure_cell_width(_renderer: &iced::Renderer) -> f32 {
    use iced::advanced::graphics::text::Paragraph;
    let para = Paragraph::with_text(iced::advanced::Text {
        content: "M",
        bounds: Size::new(f32::INFINITY, LINE_HEIGHT),
        size: Pixels(font_size()),
        line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
        font: theme::content_font(),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    });
    let w = para.min_bounds().width;
    if w > 0.0 { w } else { 7.8 }
}

/// Round `i` down to the nearest UTF-8 char boundary inside `s`. Used by
/// the highlight overlay so a malformed range (mid-multibyte) renders an
/// adjusted box rather than panicking on `&line[i..]`.
fn snap_byte_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn digit_count(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    ((n as f64).log10().floor() as usize) + 1
}

fn pixel_to_pos_wrapped(
    point: Point,
    bounds: Rectangle,
    internal: &InternalState,
    state: &EditorState,
    wrap: Option<&WrapLayout>,
    hybrid: Option<&EditorLayout>,
    scroll_y: f32,
) -> Pos {
    let cell_w = if internal.cell_width > 0.0 {
        internal.cell_width
    } else {
        7.8
    };
    let gutter_w = internal.gutter_width;
    let content_x = bounds.x + gutter_w + CONTENT_PAD;
    // Allow horizontal scroll when hybrid tables overflow the pane, even with
    // word wrap. Classic wrap (no tables) still locks scroll_x at 0.
    let scroll_x = match hybrid {
        Some(ed) if ed.content_width_chars > ed.pane_chars || wrap.is_none() => {
            state.scroll_x.max(0.0)
        }
        None if wrap.is_none() => state.scroll_x.max(0.0),
        _ => 0.0,
    };

    let vrow = ((point.y - bounds.y - CONTENT_PAD_Y + scroll_y) / LINE_HEIGHT)
        .floor()
        .max(0.0) as usize;

    let col_in_row = if point.x + scroll_x > content_x {
        ((point.x + scroll_x - content_x) / cell_w).round() as usize
    } else {
        0
    };

    if let Some(ed) = hybrid {
        let vrow = vrow.min(ed.total_visual_rows.saturating_sub(1));
        let (line_idx, sub_row) = ed.visual_to_logical(vrow);
        match ed.line_kind.get(line_idx) {
            Some(LineLayoutKind::TableRow { region, row }) => {
                let visual_in_region = ed.table_visual_row_in_region(*region, *row, sub_row);
                md_table::visual_to_source(&ed.tables, *region, visual_in_region, col_in_row)
                    .unwrap_or_else(|| Pos::new(line_idx, 0))
            }
            Some(LineLayoutKind::TableSeparator { .. }) => {
                // No visual band; map to start of the separator source line.
                Pos::new(line_idx, 0)
            }
            _ => {
                let starts = ed
                    .prose_row_starts
                    .get(line_idx)
                    .map(|s| s.as_slice())
                    .unwrap_or(&[0]);
                let char_start = starts.get(sub_row).copied().unwrap_or(0);
                let char_end = if sub_row + 1 < starts.len() {
                    starts[sub_row + 1]
                } else {
                    state.lines.get(line_idx).map(|l| l.chars().count()).unwrap_or(0)
                };
                let col = (char_start + col_in_row).min(char_end);
                // Convert char index to byte col when possible.
                let line = state.lines.get(line_idx).map(|s| s.as_str()).unwrap_or("");
                let byte_col = line
                    .char_indices()
                    .nth(col)
                    .map(|(b, _)| b)
                    .unwrap_or(line.len());
                Pos::new(line_idx, byte_col)
            }
        }
    } else if let Some(w) = wrap {
        let vrow = vrow.min(w.total_visual_rows.saturating_sub(1));
        let (line_idx, sub_row) = w.visual_to_logical(vrow);
        let starts = &w.row_starts[line_idx];
        let char_start = starts[sub_row];
        let char_end = if sub_row + 1 < starts.len() {
            starts[sub_row + 1]
        } else {
            state.lines[line_idx].chars().count()
        };
        let col = (char_start + col_in_row).min(char_end);
        Pos::new(line_idx, col)
    } else {
        let line = vrow.min(state.lines.len().saturating_sub(1));
        let col = col_in_row.min(state.lines[line].len());
        Pos::new(line, col)
    }
}

/// Find a URL or file-path reference on `pos`'s logical line that contains
/// `pos.col`. Used to arm hover state and decide whether cmd-click opens a
/// link or moves the caret. URLs win over path references when both match.
fn detect_link_at(state: &EditorState, pos: Pos) -> Option<LinkHover> {
    let line = state.lines.get(pos.line)?;
    let finder = LinkFinder::new();
    for link in finder.links(line) {
        let start_col = line[..link.start()].chars().count();
        let end_col = line[..link.end()].chars().count();
        if pos.col >= start_col && pos.col < end_col {
            return Some(LinkHover {
                line: pos.line,
                char_start: start_col,
                char_end: end_col,
                target: LinkTarget::Url(link.as_str().to_string()),
            });
        }
    }
    let hit = path_link::detect_path_at(line, pos.col)?;
    Some(LinkHover {
        line: pos.line,
        char_start: hit.char_start,
        char_end: hit.char_end,
        target: LinkTarget::Path {
            path: hit.path,
            line: hit.line,
            exists: hit.exists,
        },
    })
}

/// True when canvas-local `point` falls within the cells underlined for `hover`.
fn pos_in_hover(
    point: Point,
    bounds: Rectangle,
    internal: &InternalState,
    state: &EditorState,
    wrap: Option<&WrapLayout>,
    hybrid: Option<&EditorLayout>,
    scroll_y: f32,
    hover: &LinkHover,
) -> bool {
    let pos = pixel_to_pos_wrapped(point, bounds, internal, state, wrap, hybrid, scroll_y);
    pos.line == hover.line && pos.col >= hover.char_start && pos.col < hover.char_end
}

/// Try to read an image off the system clipboard, encode it as PNG, and
/// return an `AttachImage` action for the host to register and link into
/// the editor. Returns `None` when the clipboard has no image (or any
/// other failure) — the caller should then fall through to plain-text
/// paste.
fn read_paste_action() -> Option<EditorAction> {
    let mut clip = arboard::Clipboard::new().ok()?;
    let img = clip.get_image().ok()?;

    let mut bytes: Vec<u8> = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut bytes, img.width as u32, img.height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(&img.bytes).ok()?;
    }

    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let label = format!(
        "clip-{:04}-{:02}-{:02}-{:02}-{:02}-{:02}.png",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    );

    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{:x}", d.as_nanos()))
        .unwrap_or_else(|_| "00000000".to_string());

    Some(EditorAction::AttachImage {
        id,
        label,
        media_type: "image/png".to_string(),
        bytes,
    })
}

#[cfg(test)]
mod drag_edge_tests {
    use super::*;

    fn rect(y: f32, height: f32) -> Rectangle {
        Rectangle {
            x: 0.0,
            y,
            width: 100.0,
            height,
        }
    }

    #[test]
    fn intersecting_clamps_into_visible_span() {
        // Viewport [0, 100], bounds [20, 120] → visible [20, 100].
        let bounds = rect(20.0, 100.0);
        let viewport = rect(0.0, 100.0);
        let (y, v) = drag_pointer_y_and_velocity(150.0, bounds, viewport);
        assert_eq!(y, 100.0);
        assert!(v > 0.0, "past bottom of visible span → scroll down");
        let (y2, v2) = drag_pointer_y_and_velocity(50.0, bounds, viewport);
        assert_eq!(y2, 50.0);
        assert_eq!(v2, 0.0);
    }

    #[test]
    fn empty_intersection_below_viewport_does_not_panic() {
        // Regression: chat block just past the outer fold while drag is live.
        // Crash was `f32::clamp` with min=6709.3, max=6705.2.
        let viewport = rect(0.0, 6705.2);
        let bounds = rect(6709.3, 200.0);
        let pointer_inside = 6700.0;
        let (y, v) = drag_pointer_y_and_velocity(pointer_inside, bounds, viewport);
        assert_eq!(y, 6709.3, "pin to top of off-screen block");
        assert!(v > 0.0, "still pointer must drive scroll toward the block");
    }

    #[test]
    fn empty_intersection_above_viewport_scrolls_up() {
        let viewport = rect(500.0, 200.0); // [500, 700]
        let bounds = rect(100.0, 50.0); // [100, 150] fully above
        let (y, v) = drag_pointer_y_and_velocity(600.0, bounds, viewport);
        assert_eq!(y, 150.0, "pin to bottom of block above the fold");
        assert!(v < 0.0, "scroll up to reveal the block");
    }

    #[test]
    fn zero_height_touching_edge_is_safe() {
        // top == bottom is a valid clamp range (not min > max).
        let viewport = rect(0.0, 100.0);
        let bounds = rect(100.0, 50.0); // touches viewport bottom at y=100
        // Intersection is a point at y=100 when bounds.y == viewport_bottom.
        // bounds.y.max(viewport.y) = 100, bounds_bottom.min(viewport_bottom) = 100.
        let (y, _v) = drag_pointer_y_and_velocity(80.0, bounds, viewport);
        assert_eq!(y, 100.0);
    }
}

#[cfg(test)]
mod hybrid_layout_tests {
    use super::*;

    fn lines(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn separator_contributes_zero_visual_rows() {
        let src = lines(&[
            "before",
            "| A | B |",
            "| - | - |",
            "| 1 | 2 |",
            "after",
        ]);
        let ed = EditorLayout::compute(&src, 80, true);
        assert_eq!(ed.tables.regions.len(), 1);
        // Line 2 is the separator.
        assert!(matches!(
            ed.line_kind[2],
            LineLayoutKind::TableSeparator { .. }
        ));
        // Header + body each at least 1 visual row; separator 0; two prose lines.
        // total = 1 (before) + 1 (header) + 0 (sep) + 1 (body) + 1 (after) = 4
        assert_eq!(ed.total_visual_rows, 4);
        assert_eq!(ed.cum_rows[2], ed.cum_rows[1] + 1); // sep shares offset with body start
        assert_eq!(ed.cum_rows[3], ed.cum_rows[2]); // body starts where sep "starts"
    }

    #[test]
    fn table_overflow_widens_content_width() {
        let src = lines(&[
            "| AAAAAAAAAA | BBBBBBBBBB | CCCCCCCCCC |",
            "| ---------- | ---------- | ---------- |",
            "| aaaaaaaaaa | bbbbbbbbbb | cccccccccc |",
        ]);
        let pane = md_table::MIN_COL_CHARS * 2; // too narrow for 3 min cols
        let ed = EditorLayout::compute(&src, pane, true);
        assert_eq!(ed.tables.regions.len(), 1);
        assert!(ed.content_width_chars > pane);
        assert!(ed.content_width_chars >= ed.tables.regions[0].total_width_chars);
    }

    #[test]
    fn md_tables_false_path_unchanged_by_default() {
        // Sanity: EditorLayout is only used when the flag is on; this just
        // ensures compute is callable for prose-only docs.
        let src = lines(&["hello world that wraps"]);
        let ed = EditorLayout::compute(&src, 8, true);
        assert!(ed.tables.regions.is_empty());
        assert!(ed.total_visual_rows >= 2);
        assert!(matches!(ed.line_kind[0], LineLayoutKind::Prose));
    }

    #[test]
    fn hybrid_visual_up_from_body_lands_on_header() {
        let text = "intro\n| A | B |\n| - | - |\n| 1 | 2 |\noutra";
        let src = lines(&[
            "intro",
            "| A | B |",
            "| - | - |",
            "| 1 | 2 |",
            "outra",
        ]);
        let ed = EditorLayout::compute(&src, 80, true);
        // Body row is logical line 3; put cursor at start of first cell.
        let body = &ed.tables.regions[0].rows[1];
        let cell = &body.cells[0];
        let mut state = EditorState::new(text);
        state.cursor = Pos::new(body.source_line, cell.source_byte.start);

        let up = visual_up_target_hybrid(&state, &ed).expect("up from body");
        let header = &ed.tables.regions[0].rows[0];
        assert_eq!(up.line, header.source_line);
        assert!(
            up.col >= header.cells[0].source_byte.start
                && up.col <= header.cells[0].source_byte.end,
            "up col {} not in header cell {:?}",
            up.col,
            header.cells[0].source_byte
        );
    }

    #[test]
    fn hybrid_visual_down_from_header_skips_separator() {
        let text = "| A | B |\n| - | - |\n| 1 | 2 |";
        let src = lines(&["| A | B |", "| - | - |", "| 1 | 2 |"]);
        let ed = EditorLayout::compute(&src, 80, true);
        let header = &ed.tables.regions[0].rows[0];
        let cell = &header.cells[0];
        let mut state = EditorState::new(text);
        state.cursor = Pos::new(header.source_line, cell.source_byte.start);

        let down = visual_down_target_hybrid(&state, &ed).expect("down from header");
        let body = &ed.tables.regions[0].rows[1];
        assert_eq!(down.line, body.source_line);
        // Separator is line 1 and has zero visual height — never a target.
        assert_ne!(down.line, 1);
    }

    #[test]
    fn hybrid_pixel_to_pos_in_table_band_maps_to_cell() {
        let src = lines(&[
            "| Name | Value |",
            "| ---- | ----- |",
            "| alpha | beta |",
        ]);
        let ed = EditorLayout::compute(&src, 80, true);
        let region = &ed.tables.regions[0];
        let body = &region.rows[1];
        // Visual row of the body row's first fragment.
        let body_vrow = ed.cum_rows[body.source_line];
        let origin = md_table::col_origin(&region.col_widths, 0);
        // Click roughly on the second character of the first body cell.
        let pos = md_table::visual_to_source(&ed.tables, 0, body_vrow, origin + 1)
            .expect("visual_to_source");
        assert_eq!(pos.line, body.source_line);
        assert!(
            pos.col >= body.cells[0].source_byte.start
                && pos.col <= body.cells[0].source_byte.end
        );
    }

    #[test]
    fn table_byte_range_spans_clip_to_cell_fragment() {
        // Body cell "alpha" should produce a non-empty span for a match
        // covering those source bytes; a range on another line yields none.
        let src = lines(&[
            "| Name | Value |",
            "| ---- | ----- |",
            "| alpha | beta |",
        ]);
        let ed = EditorLayout::compute(&src, 80, true);
        let region = &ed.tables.regions[0];
        let body = &region.rows[1];
        let cell = &body.cells[0];
        assert_eq!(cell.text, "alpha");

        let hit = table_byte_range_spans(
            body,
            region,
            0,
            cell.source_byte.start,
            cell.source_byte.end,
        );
        assert_eq!(hit.len(), 1);
        let (x_chars, w_chars) = hit[0];
        assert_eq!(w_chars, 5); // "alpha"
        // Span starts at the cell's padded origin.
        let origin = md_table::col_origin(&region.col_widths, 0);
        let pad = md_table::align_pad(
            region.aligns[0],
            region.col_widths[0],
            5,
        );
        assert_eq!(x_chars, origin + pad);

        // Partial: only last 2 chars of "alpha".
        let mid = cell.source_byte.start + "alp".len();
        let partial = table_byte_range_spans(body, region, 0, mid, cell.source_byte.end);
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].1, 2);

        // Byte range outside any cell (pipe between cells) → empty.
        let pipe = cell.source_byte.end; // first byte past cell text (space or |)
        let miss = table_byte_range_spans(body, region, 0, pipe, pipe + 1);
        assert!(miss.is_empty() || miss.iter().all(|(_, w)| *w == 0));
    }

    #[test]
    fn table_byte_range_spans_ignore_non_overlapping_line() {
        let src = lines(&[
            "| A | B |",
            "| - | - |",
            "| 1 | 2 |",
        ]);
        let ed = EditorLayout::compute(&src, 80, true);
        let region = &ed.tables.regions[0];
        let body = &region.rows[1];
        // Selection-style full intermediate line uses 0..MAX.
        let all = table_byte_range_spans(body, region, 0, 0, usize::MAX);
        assert_eq!(all.len(), body.cells.len());
        // Empty range → nothing.
        assert!(table_byte_range_spans(body, region, 0, 5, 5).is_empty());
    }

    #[test]
    fn cached_hybrid_layout_reuses_until_key_changes() {
        let src = Arc::new(lines(&[
            "| A | B |",
            "| - | - |",
            "| 1 | 2 |",
        ]));
        let internal = InternalState::default();
        let a = cached_hybrid_layout(&internal, &src, 0, 80, true);
        let b = cached_hybrid_layout(&internal, &src, 0, 80, true);
        assert_eq!(a.total_visual_rows, b.total_visual_rows);
        assert_eq!(a.tables.regions.len(), 1);
        // Same key → single entry in the cache cell.
        assert!(internal.hybrid_layout.borrow().is_some());

        // Pane change invalidates.
        let narrow = cached_hybrid_layout(&internal, &src, 0, 20, true);
        assert_eq!(narrow.pane_chars, 20);

        // Version bump invalidates even with the same Arc ptr.
        let bumped = cached_hybrid_layout(&internal, &src, 1, 20, true);
        assert_eq!(bumped.total_visual_rows, narrow.total_visual_rows);

        // Direct compute still matches cached results.
        let direct = EditorLayout::compute(&src, 80, true);
        let via_cache = cached_hybrid_layout(&internal, &src, 0, 80, true);
        assert_eq!(via_cache.total_visual_rows, direct.total_visual_rows);
        assert_eq!(
            via_cache.content_width_chars,
            direct.content_width_chars
        );
    }
}
