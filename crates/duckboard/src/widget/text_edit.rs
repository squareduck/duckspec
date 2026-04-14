//! Custom monospace text editor widget with integrated line number gutter.
//!
//! Built on iced's advanced Widget API. Supports cursor movement, selection,
//! copy/cut/paste, undo/redo, and mouse click positioning.

use iced::advanced::layout;
use iced::advanced::mouse as adv_mouse;
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::text::{self, Paragraph as _, Renderer as TextRenderer};
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::keyboard::key::Named;
use iced::mouse;
use iced::{
    alignment, keyboard, Border, Color, Element, Event, Font, Length,
    Pixels, Point, Rectangle, Size, Theme,
};

use crate::highlight::HighlightSpan;
use crate::theme;

// ── Layout constants ───────────────────────────────────────────────────────

const FONT_SIZE: f32 = theme::FONT_MD;
const LINE_HEIGHT: f32 = 20.0;
const GUTTER_PAD: f32 = 8.0;
const CONTENT_PAD: f32 = 8.0;

// ── Blocks ────────────────────────────────────────────────────────────────

/// The kind of block in a block-aware editor (e.g. chat view).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    User,
    Assistant,
    ToolUse,
    ToolResult,
    System,
}

/// A content block within a block-aware editor.
#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub label: String,
    pub lines: Vec<String>,
}

/// Identifies what a visible line maps to within the block structure.
#[derive(Debug, Clone, Copy)]
pub struct BlockLineInfo {
    /// Index into `EditorState::blocks`.
    pub block_idx: usize,
    /// Whether this is the header line of the block.
    pub is_header: bool,
}

/// Background color for a block kind.
pub fn block_kind_bg(kind: BlockKind) -> Color {
    match kind {
        BlockKind::User => theme::BG_ELEVATED,
        BlockKind::Assistant => theme::BG_SURFACE,
        BlockKind::ToolUse => Color {
            r: theme::ACCENT_DIM.r * 0.15 + theme::BG_SURFACE.r * 0.85,
            g: theme::ACCENT_DIM.g * 0.15 + theme::BG_SURFACE.g * 0.85,
            b: theme::ACCENT_DIM.b * 0.15 + theme::BG_SURFACE.b * 0.85,
            a: 1.0,
        },
        BlockKind::ToolResult => Color {
            r: theme::SUCCESS.r * 0.1 + theme::BG_SURFACE.r * 0.9,
            g: theme::SUCCESS.g * 0.1 + theme::BG_SURFACE.g * 0.9,
            b: theme::SUCCESS.b * 0.1 + theme::BG_SURFACE.b * 0.9,
            a: 1.0,
        },
        BlockKind::System => theme::BG_BASE,
    }
}

/// Header label color for a block kind.
fn block_header_color(kind: BlockKind) -> Color {
    match kind {
        BlockKind::User => theme::ACCENT,
        BlockKind::Assistant => theme::TEXT_SECONDARY,
        BlockKind::ToolUse => theme::ACCENT_DIM,
        BlockKind::ToolResult => theme::SUCCESS,
        BlockKind::System => theme::TEXT_MUTED,
    }
}

// ── Position ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

// ── Undo ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum UndoOp {
    Insert {
        pos: Pos,
        text: String,
    },
    Delete {
        start: Pos,
        end: Pos,
        text: String,
    },
}

// ── Editor state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EditorState {
    pub lines: Vec<String>,
    pub cursor: Pos,
    pub anchor: Option<Pos>,
    pub scroll_y: f32,
    pub dirty: bool,
    undo_stack: Vec<UndoOp>,
    redo_stack: Vec<UndoOp>,
    /// Cached syntax-highlighted spans per line. `None` means stale/unset.
    pub highlight_spans: Option<Vec<Vec<HighlightSpan>>>,
    /// Block definitions for block-aware rendering (e.g. chat view).
    pub blocks: Vec<Block>,
    /// Maps each visible line index to its block and position within the block.
    pub block_line_map: Vec<BlockLineInfo>,
    /// True when the editor should stick to the bottom (set by scroll_to_bottom,
    /// cleared when the user scrolls up).
    pub pinned_to_bottom: bool,
}

impl EditorState {
    pub fn new(source: &str) -> Self {
        let lines: Vec<String> = if source.is_empty() {
            vec![String::new()]
        } else {
            source.lines().map(String::from).collect()
        };
        // Ensure at least one line.
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        Self {
            lines,
            cursor: Pos::new(0, 0),
            anchor: None,
            scroll_y: 0.0,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            highlight_spans: None,
            blocks: Vec::new(),
            block_line_map: Vec::new(),
            pinned_to_bottom: false,
        }
    }

    /// Create an editor from blocks. Lines are rebuilt from block content and fold state.
    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        let mut state = Self {
            lines: vec![String::new()],
            cursor: Pos::new(0, 0),
            anchor: None,
            scroll_y: 0.0,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            highlight_spans: None,
            blocks,
            block_line_map: Vec::new(),
            pinned_to_bottom: false,
        };
        state.rebuild_from_blocks();
        state
    }

    /// Rebuild `lines` and `block_line_map` from the current blocks.
    pub fn rebuild_from_blocks(&mut self) {
        let was_at_bottom = self.is_at_bottom();

        self.lines.clear();
        self.block_line_map.clear();

        for (block_idx, block) in self.blocks.iter().enumerate() {
            // Header line.
            self.lines.push(block.label.clone());
            self.block_line_map.push(BlockLineInfo {
                block_idx,
                is_header: true,
            });

            for line in &block.lines {
                self.lines.push(line.clone());
                self.block_line_map.push(BlockLineInfo {
                    block_idx,
                    is_header: false,
                });
            }
        }

        if self.lines.is_empty() {
            self.lines.push(String::new());
        }

        // Clamp cursor.
        self.cursor.line = self.cursor.line.min(self.lines.len() - 1);
        self.cursor.col = self.cursor.col.min(self.lines[self.cursor.line].len());
        self.anchor = None;
        self.highlight_spans = None;

        if was_at_bottom {
            self.scroll_to_bottom();
        }
    }

    /// Whether the editor is scrolled to (or past) the bottom.
    /// Works by checking if scroll_y exceeds any realistic content height,
    /// meaning nobody has scrolled up from the sentinel value.
    pub fn is_at_bottom(&self) -> bool {
        // scroll_to_bottom sets scroll_y to a huge sentinel.
        // Any real user scroll clamps it to max_scroll (content - viewport).
        // If scroll_y is still larger than total content, we're pinned.
        let total = self.lines.len() as f32 * LINE_HEIGHT;
        self.scroll_y >= total
    }

    /// Scroll to the bottom. The exact value is clamped by the widget at render time.
    /// Uses a very large sentinel so is_at_bottom() can detect it.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_y = 1_000_000.0;
        self.pinned_to_bottom = true;
    }

    /// Maximum scroll offset given the viewport height.
    pub fn max_scroll(&self, viewport_height: f32) -> f32 {
        let total = self.lines.len() as f32 * LINE_HEIGHT;
        (total - viewport_height).max(0.0)
    }

    /// Clamp scroll_y to valid range for the given viewport height.
    pub fn clamp_scroll(&mut self, viewport_height: f32) {
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll(viewport_height));
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Clamp cursor to valid positions.
    /// Return (start, end) of current selection, ordered.
    pub fn selection_range(&self) -> Option<(Pos, Pos)> {
        self.anchor.map(|a| {
            if a <= self.cursor {
                (a, self.cursor)
            } else {
                (self.cursor, a)
            }
        })
    }

    /// Get selected text.
    pub fn selection_text(&self) -> Option<String> {
        let (start, end) = self.selection_range()?;
        if start == end {
            return None;
        }
        Some(self.text_in_range(start, end))
    }

    fn text_in_range(&self, start: Pos, end: Pos) -> String {
        if start.line == end.line {
            self.lines[start.line][start.col..end.col].to_string()
        } else {
            let mut result = self.lines[start.line][start.col..].to_string();
            result.push('\n');
            for line in &self.lines[start.line + 1..end.line] {
                result.push_str(line);
                result.push('\n');
            }
            result.push_str(&self.lines[end.line][..end.col]);
            result
        }
    }

    /// Delete the current selection, returning the deleted text.
    /// Pushes its own undo entry.
    pub fn delete_selection(&mut self) -> Option<String> {
        let (start, end) = self.selection_range()?;
        if start == end {
            self.anchor = None;
            return None;
        }
        let deleted = self.text_in_range(start, end);
        self.delete_range(start, end);
        self.cursor = start;
        self.anchor = None;
        self.push_undo(UndoOp::Delete {
            start,
            end,
            text: deleted.clone(),
        });
        Some(deleted)
    }

    fn delete_range(&mut self, start: Pos, end: Pos) {
        if start.line == end.line {
            self.lines[start.line].replace_range(start.col..end.col, "");
        } else {
            let tail = self.lines[end.line][end.col..].to_string();
            self.lines[start.line].truncate(start.col);
            self.lines[start.line].push_str(&tail);
            self.lines.drain(start.line + 1..=end.line);
        }
    }

    fn push_undo(&mut self, op: UndoOp) {
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.dirty = true;
    }

    // ── Editing operations ─────────────────────────────────────────────

    pub fn insert_char(&mut self, ch: char) {
        // delete_selection pushes its own undo entry if needed.
        self.delete_selection();
        let pos = self.cursor;
        if ch == '\n' {
            let tail = self.lines[pos.line][pos.col..].to_string();
            self.lines[pos.line].truncate(pos.col);
            self.lines.insert(pos.line + 1, tail);
            self.cursor = Pos::new(pos.line + 1, 0);
        } else {
            self.lines[pos.line].insert(pos.col, ch);
            self.cursor.col += ch.len_utf8();
        }
        self.push_undo(UndoOp::Insert {
            pos,
            text: ch.to_string(),
        });
    }

    pub fn insert_text(&mut self, s: &str) {
        self.delete_selection();
        let pos = self.cursor;
        let lines: Vec<&str> = s.split('\n').collect();
        if lines.len() == 1 {
            self.lines[pos.line].insert_str(pos.col, lines[0]);
            self.cursor.col += lines[0].len();
        } else {
            let tail = self.lines[pos.line][pos.col..].to_string();
            self.lines[pos.line].truncate(pos.col);
            self.lines[pos.line].push_str(lines[0]);
            for (i, line) in lines[1..].iter().enumerate() {
                if i == lines.len() - 2 {
                    // Last fragment — attach tail.
                    let mut combined = line.to_string();
                    combined.push_str(&tail);
                    self.lines.insert(pos.line + 1 + i, combined);
                    self.cursor = Pos::new(pos.line + 1 + i, line.len());
                } else {
                    self.lines.insert(pos.line + 1 + i, line.to_string());
                }
            }
        }
        self.push_undo(UndoOp::Insert {
            pos,
            text: s.to_string(),
        });
    }

    pub fn backspace(&mut self) {
        if self.delete_selection().is_some() {
            return;
        }
        if self.cursor.col > 0 {
            let ch = self.lines[self.cursor.line]
                .remove(self.cursor.col - 1);
            self.cursor.col -= ch.len_utf8();
            self.push_undo(UndoOp::Delete {
                start: self.cursor,
                end: Pos::new(self.cursor.line, self.cursor.col + ch.len_utf8()),
                text: ch.to_string(),
            });
        } else if self.cursor.line > 0 {
            let removed_line = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            let col = self.lines[self.cursor.line].len();
            self.lines[self.cursor.line].push_str(&removed_line);
            self.cursor.col = col;
            self.push_undo(UndoOp::Delete {
                start: self.cursor,
                end: Pos::new(self.cursor.line + 1, 0),
                text: "\n".to_string(),
            });
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection().is_some() {
            return;
        }
        let line = &self.lines[self.cursor.line];
        if self.cursor.col < line.len() {
            let ch = self.lines[self.cursor.line].remove(self.cursor.col);
            self.push_undo(UndoOp::Delete {
                start: self.cursor,
                end: Pos::new(self.cursor.line, self.cursor.col + ch.len_utf8()),
                text: ch.to_string(),
            });
        } else if self.cursor.line + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor.line + 1);
            self.lines[self.cursor.line].push_str(&next);
            self.push_undo(UndoOp::Delete {
                start: self.cursor,
                end: Pos::new(self.cursor.line + 1, 0),
                text: "\n".to_string(),
            });
        }
    }

    pub fn undo(&mut self) {
        let Some(op) = self.undo_stack.pop() else {
            return;
        };
        match &op {
            UndoOp::Insert { pos, text } => {
                // Undo insert → delete the inserted text.
                let end = self.pos_after_insert(*pos, text);
                self.delete_range(*pos, end);
                self.cursor = *pos;
            }
            UndoOp::Delete { start, text, .. } => {
                // Undo delete → re-insert the deleted text.
                self.cursor = *start;
                let lines: Vec<&str> = text.split('\n').collect();
                if lines.len() == 1 {
                    self.lines[start.line].insert_str(start.col, text);
                } else {
                    let tail = self.lines[start.line][start.col..].to_string();
                    self.lines[start.line].truncate(start.col);
                    self.lines[start.line].push_str(lines[0]);
                    for (i, line) in lines[1..].iter().enumerate() {
                        if i == lines.len() - 2 {
                            let mut combined = line.to_string();
                            combined.push_str(&tail);
                            self.lines.insert(start.line + 1 + i, combined);
                        } else {
                            self.lines.insert(start.line + 1 + i, line.to_string());
                        }
                    }
                }
                self.cursor = self.pos_after_insert(*start, text);
            }
        }
        self.anchor = None;
        self.redo_stack.push(op);
    }

    pub fn redo(&mut self) {
        let Some(op) = self.redo_stack.pop() else {
            return;
        };
        match &op {
            UndoOp::Insert { pos, text } => {
                self.cursor = *pos;
                let lines: Vec<&str> = text.split('\n').collect();
                if lines.len() == 1 {
                    self.lines[pos.line].insert_str(pos.col, text);
                    self.cursor.col += text.len();
                } else {
                    let tail = self.lines[pos.line][pos.col..].to_string();
                    self.lines[pos.line].truncate(pos.col);
                    self.lines[pos.line].push_str(lines[0]);
                    for (i, line) in lines[1..].iter().enumerate() {
                        if i == lines.len() - 2 {
                            let mut combined = line.to_string();
                            combined.push_str(&tail);
                            self.lines.insert(pos.line + 1 + i, combined);
                            self.cursor = Pos::new(pos.line + 1 + i, line.len());
                        } else {
                            self.lines.insert(pos.line + 1 + i, line.to_string());
                        }
                    }
                }
            }
            UndoOp::Delete { start, end, .. } => {
                self.delete_range(*start, *end);
                self.cursor = *start;
            }
        }
        self.anchor = None;
        self.undo_stack.push(op);
    }

    fn pos_after_insert(&self, start: Pos, text: &str) -> Pos {
        let lines: Vec<&str> = text.split('\n').collect();
        if lines.len() == 1 {
            Pos::new(start.line, start.col + text.len())
        } else {
            Pos::new(
                start.line + lines.len() - 1,
                lines.last().map_or(0, |l| l.len()),
            )
        }
    }

    // ── Navigation ─────────────────────────────────────────────────────

    pub fn move_left(&mut self, select: bool) {
        self.update_anchor(select);
        if self.cursor.col > 0 {
            // Move back by one character (handle UTF-8).
            let line = &self.lines[self.cursor.line];
            let prev = line[..self.cursor.col]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.cursor.col = prev;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
        }
        if !select {
            self.collapse_selection_if_no_anchor();
        }
    }

    pub fn move_right(&mut self, select: bool) {
        self.update_anchor(select);
        let line_len = self.lines[self.cursor.line].len();
        if self.cursor.col < line_len {
            let ch = self.lines[self.cursor.line][self.cursor.col..]
                .chars()
                .next()
                .unwrap();
            self.cursor.col += ch.len_utf8();
        } else if self.cursor.line + 1 < self.lines.len() {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
        if !select {
            self.collapse_selection_if_no_anchor();
        }
    }

    pub fn move_up(&mut self, select: bool) {
        self.update_anchor(select);
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.line].len());
        }
        if !select {
            self.collapse_selection_if_no_anchor();
        }
    }

    pub fn move_down(&mut self, select: bool) {
        self.update_anchor(select);
        if self.cursor.line + 1 < self.lines.len() {
            self.cursor.line += 1;
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.line].len());
        }
        if !select {
            self.collapse_selection_if_no_anchor();
        }
    }

    pub fn move_home(&mut self, select: bool) {
        self.update_anchor(select);
        self.cursor.col = 0;
        if !select {
            self.collapse_selection_if_no_anchor();
        }
    }

    pub fn move_end(&mut self, select: bool) {
        self.update_anchor(select);
        self.cursor.col = self.lines[self.cursor.line].len();
        if !select {
            self.collapse_selection_if_no_anchor();
        }
    }

    pub fn move_word_left(&mut self, select: bool) {
        self.update_anchor(select);
        if self.cursor.col == 0 {
            self.move_left(select);
            return;
        }
        let line = &self.lines[self.cursor.line];
        let before = &line[..self.cursor.col];
        // Skip whitespace, then skip word chars.
        let trimmed = before.trim_end();
        if trimmed.is_empty() {
            self.cursor.col = 0;
        } else {
            // Find start of last word.
            let word_start = trimmed
                .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                .map(|i| i + 1)
                .unwrap_or(0);
            self.cursor.col = word_start;
        }
        if !select {
            self.collapse_selection_if_no_anchor();
        }
    }

    pub fn move_word_right(&mut self, select: bool) {
        self.update_anchor(select);
        let line_len = self.lines[self.cursor.line].len();
        if self.cursor.col >= line_len {
            self.move_right(select);
            return;
        }
        let line = &self.lines[self.cursor.line];
        let after = &line[self.cursor.col..];
        // Skip word chars, then skip whitespace.
        let word_end = after
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(after.len());
        let rest = &after[word_end..];
        let space_end = rest
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(rest.len());
        self.cursor.col += word_end + space_end;
        if !select {
            self.collapse_selection_if_no_anchor();
        }
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(Pos::new(0, 0));
        let last = self.lines.len() - 1;
        self.cursor = Pos::new(last, self.lines[last].len());
    }

    fn update_anchor(&mut self, select: bool) {
        if select && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        } else if !select {
            self.anchor = None;
        }
    }

    fn collapse_selection_if_no_anchor(&mut self) {
        if self.anchor.is_none() {
            // Already collapsed.
        }
    }

    /// Ensure the cursor is visible given the viewport height.
    pub fn scroll_to_cursor(&mut self, viewport_height: f32) {
        let cursor_y = self.cursor.line as f32 * LINE_HEIGHT;
        if cursor_y < self.scroll_y {
            self.scroll_y = cursor_y;
        } else if cursor_y + LINE_HEIGHT > self.scroll_y + viewport_height {
            self.scroll_y = cursor_y + LINE_HEIGHT - viewport_height;
        }
    }
}

// ── Messages ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum EditorAction {
    Insert(char),
    Paste(String),
    Backspace,
    Delete,
    Enter,
    MoveLeft(bool),
    MoveRight(bool),
    MoveUp(bool),
    MoveDown(bool),
    MoveHome(bool),
    MoveEnd(bool),
    MoveWordLeft(bool),
    MoveWordRight(bool),
    SelectAll,
    Copy,
    Cut,
    Undo,
    Redo,
    Click(Pos),
    Drag(Pos),
    /// Scroll by `dy` pixels, with viewport and content heights for clamping.
    Scroll { dy: f32, viewport_height: f32, content_height: f32 },
    SaveRequested,
}

// ── Widget internal state (in iced tree) ───────────────────────────────────

#[derive(Debug, Default)]
struct InternalState {
    focused: bool,
    dragging: bool,
    cell_width: f32,
    gutter_width: f32,
}

// ── Word wrap ─────────────────────────────────────────────────────────────

/// Cached word-wrap layout for all lines.
#[derive(Debug, Clone)]
struct WrapLayout {
    /// For each logical line: the character offsets where each visual row starts.
    /// E.g. `[[0, 60, 120]]` means line 0 wraps into 3 visual rows.
    row_starts: Vec<Vec<usize>>,
    /// Total number of visual rows across all logical lines.
    total_visual_rows: usize,
    /// Cumulative visual row offset for each logical line.
    /// `cum_rows[i]` = total visual rows for lines 0..i.
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
        // Binary search for the logical line.
        let line = match self.cum_rows.binary_search(&visual_row) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line = line.min(self.row_starts.len().saturating_sub(1));
        let sub_row = visual_row.saturating_sub(self.cum_rows[line]);
        (line, sub_row)
    }

    /// Convert a logical (line, col) to a visual row index.
    fn logical_to_visual(&self, line: usize, col: usize) -> usize {
        if line >= self.row_starts.len() {
            return self.total_visual_rows.saturating_sub(1);
        }
        let base = self.cum_rows[line];
        let starts = &self.row_starts[line];
        // Find which sub-row this col falls in.
        let sub = starts
            .iter()
            .rposition(|&s| col >= s)
            .unwrap_or(0);
        base + sub
    }

}

/// Compute the character offsets where each visual row starts for a single line.
/// Word-wraps at spaces when possible, falls back to hard break.
fn wrap_line_starts(line: &str, max_chars: usize) -> Vec<usize> {
    if max_chars == 0 {
        return vec![0];
    }
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    if len <= max_chars {
        return vec![0];
    }

    let mut starts = vec![0usize];
    let mut pos = 0;
    while pos < len {
        let remaining = len - pos;
        if remaining <= max_chars {
            break;
        }
        let end = pos + max_chars;
        // Look back from end for a space to break at.
        let break_at = (pos..end)
            .rev()
            .find(|&i| chars[i] == ' ')
            .map(|i| i + 1) // break after the space
            .unwrap_or(end); // hard break if no space found
        // Avoid zero-width rows.
        let break_at = if break_at <= pos { end } else { break_at };
        starts.push(break_at);
        pos = break_at;
    }
    starts
}

// ── Widget ─────────────────────────────────────────────────────────────────

pub struct TextEdit<'a, M> {
    state: &'a EditorState,
    on_action: Box<dyn Fn(EditorAction) -> M + 'a>,
    show_gutter: bool,
    fit_content: bool,
    read_only: bool,
    word_wrap: bool,
}

impl<'a, M> TextEdit<'a, M> {
    pub fn new(
        state: &'a EditorState,
        on_action: impl Fn(EditorAction) -> M + 'a,
    ) -> Self {
        Self {
            state,
            on_action: Box::new(on_action),
            show_gutter: true,
            fit_content: false,
            read_only: false,
            word_wrap: false,
        }
    }

    pub fn show_gutter(mut self, show: bool) -> Self {
        self.show_gutter = show;
        self
    }

    pub fn fit_content(mut self, fit: bool) -> Self {
        self.fit_content = fit;
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
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        if self.fit_content {
            let row_count = if self.word_wrap {
                // Estimate wrap using limits and fallback cell width.
                let max_w = limits.max().width;
                let cell_w = 7.8f32; // fallback
                let content_w = max_w - if self.show_gutter { 50.0 } else { 0.0 } - CONTENT_PAD;
                let cpr = (content_w / cell_w).floor().max(1.0) as usize;
                let wrap = WrapLayout::compute(&self.state.lines, cpr);
                wrap.total_visual_rows
            } else {
                self.state.line_count()
            };
            let height = row_count.max(1) as f32 * LINE_HEIGHT;
            let limits = limits.width(Length::Fill);
            layout::Node::new(limits.resolve(
                Length::Fill,
                Length::Fixed(height),
                Size::ZERO,
            ))
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

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let internal = tree.state.downcast_mut::<InternalState>();

        // Measure cell width on first event if not set.
        if internal.cell_width == 0.0 {
            let measured = measure_cell_width(renderer);
            internal.cell_width = measured;
            // Gutter width: enough for line numbers + padding.
            if self.show_gutter {
                let digits = digit_count(self.state.line_count());
                internal.gutter_width =
                    (digits as f32) * measured + GUTTER_PAD * 2.0;
            } else {
                internal.gutter_width = 0.0;
            }
        }

        // Compute wrap layout if enabled.
        let wrap = if self.word_wrap {
            let cell_w = if internal.cell_width > 0.0 { internal.cell_width } else { 7.8 };
            let content_w = bounds.width - internal.gutter_width - CONTENT_PAD;
            let cpr = (content_w / cell_w).floor().max(1.0) as usize;
            Some(WrapLayout::compute(&self.state.lines, cpr))
        } else {
            None
        };

        let content_height = if let Some(ref w) = wrap {
            w.total_visual_rows as f32 * LINE_HEIGHT
        } else {
            self.state.lines.len() as f32 * LINE_HEIGHT
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds) {
                    internal.focused = true;
                    let pos = cursor.position().unwrap();
                    let click_pos =
                        pixel_to_pos_wrapped(pos, bounds, internal, self.state, wrap.as_ref(), content_height);

                    internal.dragging = true;
                    shell.publish((self.on_action)(EditorAction::Click(click_pos)));
                } else {
                    internal.focused = false;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if internal.dragging {
                    let drag_pos =
                        pixel_to_pos_wrapped(*position, bounds, internal, self.state, wrap.as_ref(), content_height);
                    shell.publish((self.on_action)(EditorAction::Drag(drag_pos)));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                internal.dragging = false;
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.is_over(bounds) {
                    let dy = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => -*y * LINE_HEIGHT * 3.0,
                        mouse::ScrollDelta::Pixels { y, .. } => -*y,
                    };
                    shell.publish((self.on_action)(EditorAction::Scroll {
                        dy,
                        viewport_height: bounds.height,
                        content_height,
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
                        shell.publish((self.on_action)(EditorAction::MoveUp(shift)));
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        shell.publish((self.on_action)(EditorAction::MoveDown(shift)));
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
                        shell.publish((self.on_action)(EditorAction::Enter));
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "x" && !self.read_only => {
                        if let Some(sel) = self.state.selection_text() {
                            clipboard.write(iced::advanced::clipboard::Kind::Standard, sel);
                            shell.publish((self.on_action)(EditorAction::Cut));
                        }
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "v" && !self.read_only => {
                        if let Some(text) =
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
                    _ if !self.read_only => {
                        if !cmd && !modifiers.control() {
                            if let Some(txt) = key_text {
                                for ch in txt.chars() {
                                    if !ch.is_control() {
                                        shell.publish(
                                            (self.on_action)(EditorAction::Insert(ch)),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: adv_mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let internal = tree.state.downcast_ref::<InternalState>();
        let cell_w = if internal.cell_width > 0.0 {
            internal.cell_width
        } else {
            7.8 // fallback
        };
        let gutter_w = if self.show_gutter {
            let digits = digit_count(self.state.line_count());
            (digits as f32) * cell_w + GUTTER_PAD * 2.0
        } else {
            0.0
        };
        let content_x = bounds.x + gutter_w;
        let content_w = bounds.width - gutter_w;

        // Compute wrap layout if needed.
        let wrap = if self.word_wrap {
            let cpr = ((content_w - CONTENT_PAD) / cell_w).floor().max(1.0) as usize;
            Some(WrapLayout::compute(&self.state.lines, cpr))
        } else {
            None
        };
        let total_visual_rows = wrap.as_ref().map_or(self.state.line_count(), |w| w.total_visual_rows);
        let content_height = total_visual_rows as f32 * LINE_HEIGHT;

        // Clamp scroll so we don't render past the content.
        let max_scroll = (content_height - bounds.height).max(0.0);
        let scroll_y = self.state.scroll_y.clamp(0.0, max_scroll);

        // Clip to bounds.
        renderer.with_layer(bounds, |renderer: &mut iced::Renderer| {
            // Background.
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds,
                    border: Border::default(),
                    ..renderer::Quad::default()
                },
                theme::BG_BASE,
            );

            // Gutter background.
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
                    theme::BG_SURFACE,
                );
            }

            let first_vrow = (scroll_y / LINE_HEIGHT).floor() as usize;
            let visible_vrows = (bounds.height / LINE_HEIGHT).ceil() as usize + 1;
            let last_vrow = (first_vrow + visible_vrows).min(total_visual_rows);

            let selection = self.state.selection_range();
            let has_blocks = !self.state.blocks.is_empty();

            for vrow in first_vrow..last_vrow {
                let y = bounds.y + (vrow as f32) * LINE_HEIGHT - scroll_y;

                // Map visual row to logical line + sub-row.
                let (line_idx, sub_row, char_start, char_end) = if let Some(ref w) = wrap {
                    let (li, sr) = w.visual_to_logical(vrow);
                    let starts = &w.row_starts[li];
                    let cs = starts[sr];
                    let ce = if sr + 1 < starts.len() {
                        starts[sr + 1]
                    } else {
                        self.state.lines[li].chars().count()
                    };
                    (li, sr, cs, ce)
                } else {
                    let len = self.state.lines[vrow].chars().count();
                    (vrow, 0, 0, len)
                };

                // Block background.
                if has_blocks {
                    if let Some(info) = self.state.block_line_map.get(line_idx) {
                        if let Some(block) = self.state.blocks.get(info.block_idx) {
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
                    }
                }

                // Selection highlight.
                if let Some((sel_start, sel_end)) = selection {
                    if line_idx >= sel_start.line && line_idx <= sel_end.line {
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
                        if sel_col_start < sel_col_end && sel_col_start < abs_col_end && sel_col_end > abs_col_start {
                            let vis_start = sel_col_start.saturating_sub(abs_col_start);
                            let vis_end = sel_col_end.saturating_sub(abs_col_start);
                            let sel_x = content_x + CONTENT_PAD + vis_start as f32 * cell_w;
                            let sel_w = (vis_end - vis_start) as f32 * cell_w;
                            renderer::Renderer::fill_quad(
                                renderer,
                                renderer::Quad {
                                    bounds: Rectangle { x: sel_x, y, width: sel_w, height: LINE_HEIGHT },
                                    border: Border::default(),
                                    ..renderer::Quad::default()
                                },
                                Color { a: 0.3, ..theme::ACCENT },
                            );
                        }
                    }
                }

                // Line number — only on first sub-row.
                if self.show_gutter && sub_row == 0 {
                    let digits = digit_count(self.state.line_count());
                    let line_num = format!("{:>width$} ", line_idx + 1, width = digits);
                    let num_color = if line_idx == self.state.cursor.line && internal.focused {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_MUTED
                    };
                    renderer.fill_text(
                        iced::advanced::Text {
                            content: line_num,
                            bounds: Size::new(gutter_w, LINE_HEIGHT),
                            size: Pixels(FONT_SIZE),
                            line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                            font: Font::MONOSPACE,
                            align_x: alignment::Horizontal::Left.into(),
                            align_y: alignment::Vertical::Top,
                            shaping: text::Shaping::Basic,
                            wrapping: text::Wrapping::None,
                        },
                        Point::new(bounds.x + GUTTER_PAD, y),
                        num_color,
                        bounds,
                    );
                }

                // Extract the sub-string for this visual row.
                let line = &self.state.lines[line_idx];
                let row_text: String = line.chars().skip(char_start).take(char_end - char_start).collect();

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
                                size: Pixels(FONT_SIZE),
                                line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                                font: Font::MONOSPACE,
                                align_x: alignment::Horizontal::Left.into(),
                                align_y: alignment::Vertical::Top,
                                shaping: text::Shaping::Basic,
                                wrapping: text::Wrapping::None,
                            },
                            Point::new(content_x + CONTENT_PAD, y),
                            color,
                            bounds,
                        );
                    } else {
                        // Syntax highlighting spans — need to slice for this visual row.
                        let spans = self.state.highlight_spans.as_ref().and_then(|cache| cache.get(line_idx));

                        if let Some(spans) = spans {
                            // Draw only the portion of spans that falls within char_start..char_end.
                            let mut col = 0usize; // character column in the logical line
                            let mut x_off = 0.0f32;
                            for span in spans {
                                let span_chars = span.text.chars().count();
                                let span_end = col + span_chars;
                                // Check overlap with [char_start, char_end).
                                if span_end > char_start && col < char_end {
                                    let vis_start = col.max(char_start) - char_start;
                                    let vis_end = span_end.min(char_end) - char_start;
                                    let slice: String = span.text.chars()
                                        .skip(col.max(char_start) - col)
                                        .take(vis_end - vis_start)
                                        .collect();
                                    if !slice.is_empty() {
                                        let sw = slice.chars().count() as f32 * cell_w;
                                        let sx = content_x + CONTENT_PAD + vis_start as f32 * cell_w;
                                        renderer.fill_text(
                                            iced::advanced::Text {
                                                content: slice,
                                                bounds: Size::new(sw + cell_w, LINE_HEIGHT),
                                                size: Pixels(FONT_SIZE),
                                                line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                                                font: Font::MONOSPACE,
                                                align_x: alignment::Horizontal::Left.into(),
                                                align_y: alignment::Vertical::Top,
                                                shaping: text::Shaping::Basic,
                                                wrapping: text::Wrapping::None,
                                            },
                                            Point::new(sx, y),
                                            span.color,
                                            bounds,
                                        );
                                    }
                                }
                                col = span_end;
                                let _ = x_off; // suppress unused
                                x_off = (col.saturating_sub(char_start)) as f32 * cell_w;
                            }
                        } else {
                            renderer.fill_text(
                                iced::advanced::Text {
                                    content: row_text,
                                    bounds: Size::new(content_w, LINE_HEIGHT),
                                    size: Pixels(FONT_SIZE),
                                    line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
                                    font: Font::MONOSPACE,
                                    align_x: alignment::Horizontal::Left.into(),
                                    align_y: alignment::Vertical::Top,
                                    shaping: text::Shaping::Basic,
                                    wrapping: text::Wrapping::None,
                                },
                                Point::new(content_x + CONTENT_PAD, y),
                                theme::TEXT_PRIMARY,
                                bounds,
                            );
                        }
                    }
                }
            }

            // Cursor.
            if internal.focused {
                let cursor_line = self.state.cursor.line;
                let cursor_col = self.state.cursor.col;
                let (cy, cx) = if let Some(ref w) = wrap {
                    let vrow = w.logical_to_visual(cursor_line, cursor_col);
                    let starts = &w.row_starts[cursor_line];
                    let sub = starts.iter().rposition(|&s| cursor_col >= s).unwrap_or(0);
                    let col_in_row = cursor_col - starts[sub];
                    (
                        bounds.y + vrow as f32 * LINE_HEIGHT - scroll_y,
                        content_x + CONTENT_PAD + col_in_row as f32 * cell_w,
                    )
                } else {
                    (
                        bounds.y + cursor_line as f32 * LINE_HEIGHT - scroll_y,
                        content_x + CONTENT_PAD + cursor_col as f32 * cell_w,
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
                    theme::ACCENT,
                );
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

/// Measure the width of a single monospace character using cosmic-text.
fn measure_cell_width(_renderer: &iced::Renderer) -> f32 {
    use iced::advanced::graphics::text::Paragraph;
    let para = Paragraph::with_text(iced::advanced::Text {
        content: "M",
        bounds: Size::new(f32::INFINITY, LINE_HEIGHT),
        size: Pixels(FONT_SIZE),
        line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
        font: Font::MONOSPACE,
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    });
    let w = para.min_bounds().width;
    if w > 0.0 {
        w
    } else {
        7.8 // fallback
    }
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
    content_height: f32,
) -> Pos {
    let cell_w = if internal.cell_width > 0.0 {
        internal.cell_width
    } else {
        7.8
    };
    let gutter_w = internal.gutter_width;
    let content_x = bounds.x + gutter_w + CONTENT_PAD;
    let max_scroll = (content_height - bounds.height).max(0.0);
    let scroll_y = state.scroll_y.clamp(0.0, max_scroll);

    let vrow = ((point.y - bounds.y + scroll_y) / LINE_HEIGHT)
        .floor()
        .max(0.0) as usize;

    let col_in_row = if point.x > content_x {
        ((point.x - content_x) / cell_w).round() as usize
    } else {
        0
    };

    if let Some(w) = wrap {
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

/// Convenience: create the widget.
pub fn view<'a, M: Clone + 'a>(
    state: &'a EditorState,
    on_action: impl Fn(EditorAction) -> M + 'a,
) -> Element<'a, M> {
    TextEdit::new(state, on_action).into()
}
