//! Editor state: positions, blocks, text buffer, cursor, undo/redo, navigation.

use std::sync::Arc;

use crate::highlight::HighlightSpan;
use crate::theme;
use iced::Color;

// ── Layout constants (shared with render) ─────────────────────────────────

pub(crate) const LINE_HEIGHT: f32 = 20.0;
pub(crate) const CONTENT_PAD_Y: f32 = 4.0;

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
        BlockKind::User => theme::chat_bg_user(),
        BlockKind::Assistant => theme::chat_bg_assistant(),
        BlockKind::ToolUse => theme::chat_bg_tool_use(),
        BlockKind::ToolResult => theme::chat_bg_tool_result(),
        BlockKind::System => theme::chat_bg_system(),
    }
}

/// Semantic tag for a per-line background. Resolved to a Color at render time
/// so theme toggles are reflected without rebuilding the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineBgKind {
    Hunk,
    Added,
    Removed,
    Match,
}

pub(crate) fn line_bg_color(kind: LineBgKind) -> Color {
    match kind {
        LineBgKind::Hunk => theme::diff_hunk_bg(),
        LineBgKind::Added => theme::diff_added_bg(),
        LineBgKind::Removed => theme::diff_removed_bg(),
        LineBgKind::Match => theme::search_match_bg(),
    }
}

/// Header label color for a block kind.
pub(crate) fn block_header_color(kind: BlockKind) -> Color {
    match kind {
        BlockKind::User => theme::accent(),
        BlockKind::Assistant => theme::text_secondary(),
        BlockKind::ToolUse => theme::accent_dim(),
        BlockKind::ToolResult => theme::success(),
        BlockKind::System => theme::text_muted(),
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
    Insert { pos: Pos, text: String },
    Delete { start: Pos, end: Pos, text: String },
}

// ── Editor state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EditorState {
    /// Shared via `Arc` so read-only consumers (e.g. search-stack slices
    /// materialised from the same file) can hold cheap clones. Mutating
    /// paths use `Arc::make_mut` — free for editable tabs where the
    /// refcount stays at 1, copy-on-write otherwise.
    pub lines: Arc<Vec<String>>,
    pub cursor: Pos,
    pub anchor: Option<Pos>,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub dirty: bool,
    undo_stack: Vec<UndoOp>,
    redo_stack: Vec<UndoOp>,
    /// Cached syntax-highlighted spans per line. `None` means stale/unset.
    pub highlight_spans: Option<Vec<Vec<HighlightSpan>>>,
    /// Bumped on every content-mutating `apply_action` (and every
    /// `rebuild_from_blocks`). Async highlighters read this at spawn time and
    /// the `TabHighlighted` handler drops stale results whose version no
    /// longer matches.
    pub highlight_version: u64,
    /// Per-line background tags (e.g. diff added/removed). Empty = no
    /// backgrounds. Resolved to a Color at render time so theme toggles
    /// take effect without rebuilding the editor.
    pub line_backgrounds: Vec<Option<LineBgKind>>,
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
            lines: Arc::new(lines),
            cursor: Pos::new(0, 0),
            anchor: None,
            scroll_x: 0.0,
            scroll_y: 0.0,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            highlight_spans: None,
            highlight_version: 0,
            line_backgrounds: Vec::new(),
            blocks: Vec::new(),
            block_line_map: Vec::new(),
            pinned_to_bottom: false,
        }
    }

    /// Create an editor from blocks. Lines are rebuilt from block content and fold state.
    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        let mut state = Self {
            lines: Arc::new(vec![String::new()]),
            cursor: Pos::new(0, 0),
            anchor: None,
            scroll_x: 0.0,
            scroll_y: 0.0,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            highlight_spans: None,
            highlight_version: 0,
            line_backgrounds: Vec::new(),
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

        let lines = Arc::make_mut(&mut self.lines);
        lines.clear();
        self.block_line_map.clear();

        for (block_idx, block) in self.blocks.iter().enumerate() {
            lines.push(block.label.clone());
            self.block_line_map.push(BlockLineInfo {
                block_idx,
                is_header: true,
            });

            for line in &block.lines {
                lines.push(line.clone());
                self.block_line_map.push(BlockLineInfo {
                    block_idx,
                    is_header: false,
                });
            }
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        // Clamp cursor.
        self.cursor.line = self.cursor.line.min(self.lines.len() - 1);
        self.cursor.col = self.cursor.col.min(self.lines[self.cursor.line].len());
        self.anchor = None;
        self.highlight_spans = None;
        self.highlight_version = self.highlight_version.wrapping_add(1);

        if was_at_bottom {
            self.scroll_to_bottom();
        }
    }

    /// Whether the editor is scrolled to (or past) the bottom.
    pub fn is_at_bottom(&self) -> bool {
        let total = self.lines.len() as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0;
        self.scroll_y >= total
    }

    /// Scroll to the bottom. Uses a large sentinel so is_at_bottom() can detect it.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_y = 1_000_000.0;
        self.pinned_to_bottom = true;
    }

    /// Maximum scroll offset given the viewport height.
    pub fn max_scroll(&self, viewport_height: f32) -> f32 {
        let total = self.lines.len() as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0;
        (total - viewport_height).max(0.0)
    }

    /// Clamp scroll_y to valid range for the given viewport height.
    pub fn clamp_scroll(&mut self, viewport_height: f32) {
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll(viewport_height));
    }

    /// Apply an editor action, returning `true` if the text content was mutated
    /// (i.e. the caller should re-run syntax highlighting).
    pub fn apply_action(&mut self, action: EditorAction) -> bool {
        let mutates = action.is_mutating();
        if mutates {
            // Keep `highlight_spans` in place so edited lines retain their
            // previous colors while the async re-highlight is in flight.
            // The renderer sources text from the current line (not
            // `span.text`), so characters stay correct; colors near the
            // edit point may be briefly misaligned until new spans land.
            self.highlight_version = self.highlight_version.wrapping_add(1);
        }

        match action {
            EditorAction::Insert(ch) => self.insert_char(ch),
            EditorAction::Paste(text) => self.insert_text(&text),
            EditorAction::Backspace => self.backspace(),
            EditorAction::Delete => self.delete(),
            EditorAction::Enter => self.insert_char('\n'),
            EditorAction::MoveLeft(sel) => self.move_left(sel),
            EditorAction::MoveRight(sel) => self.move_right(sel),
            EditorAction::MoveUp(sel) => self.move_up(sel),
            EditorAction::MoveDown(sel) => self.move_down(sel),
            EditorAction::MoveHome(sel) => self.move_home(sel),
            EditorAction::MoveEnd(sel) => self.move_end(sel),
            EditorAction::MoveWordLeft(sel) => self.move_word_left(sel),
            EditorAction::MoveWordRight(sel) => self.move_word_right(sel),
            EditorAction::SelectAll => self.select_all(),
            EditorAction::Copy => {}
            EditorAction::Cut => {
                self.delete_selection();
            }
            EditorAction::Undo => self.undo(),
            EditorAction::Redo => self.redo(),
            EditorAction::Click(pos) => {
                self.cursor = pos;
                self.anchor = None;
            }
            EditorAction::Drag(pos) => {
                if self.anchor.is_none() {
                    self.anchor = Some(self.cursor);
                }
                self.cursor = pos;
            }
            EditorAction::Scroll {
                dy,
                dx,
                viewport_height,
                content_height,
                viewport_width,
                content_width,
            } => {
                let max_y = (content_height - viewport_height).max(0.0);
                self.scroll_y = (self.scroll_y + dy).clamp(0.0, max_y);
                let max_x = (content_width - viewport_width).max(0.0);
                self.scroll_x = (self.scroll_x + dx).clamp(0.0, max_x);
            }
            EditorAction::SaveRequested => {
                // Handled upstream by `handle_editor_action`; no-op here so
                // that editors without a file path (e.g. chat input) simply
                // ignore Cmd+S.
            }
            EditorAction::OpenUrl(_) => {
                // Handled upstream — see `handle_editor_action` and chat input
                // handlers, which dispatch to the system opener.
            }
        }

        mutates
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

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
        let lines = Arc::make_mut(&mut self.lines);
        if start.line == end.line {
            lines[start.line].replace_range(start.col..end.col, "");
        } else {
            let tail = lines[end.line][end.col..].to_string();
            lines[start.line].truncate(start.col);
            lines[start.line].push_str(&tail);
            lines.drain(start.line + 1..=end.line);
        }
    }

    fn push_undo(&mut self, op: UndoOp) {
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.dirty = true;
    }

    // ── Editing operations ─────────────────────────────────────────────

    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        let pos = self.cursor;
        let lines = Arc::make_mut(&mut self.lines);
        if ch == '\n' {
            let tail = lines[pos.line][pos.col..].to_string();
            lines[pos.line].truncate(pos.col);
            lines.insert(pos.line + 1, tail);
            self.cursor = Pos::new(pos.line + 1, 0);
        } else {
            lines[pos.line].insert(pos.col, ch);
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
        let parts: Vec<&str> = s.split('\n').collect();
        let lines = Arc::make_mut(&mut self.lines);
        if parts.len() == 1 {
            lines[pos.line].insert_str(pos.col, parts[0]);
            self.cursor.col += parts[0].len();
        } else {
            let tail = lines[pos.line][pos.col..].to_string();
            lines[pos.line].truncate(pos.col);
            lines[pos.line].push_str(parts[0]);
            for (i, line) in parts[1..].iter().enumerate() {
                if i == parts.len() - 2 {
                    let mut combined = line.to_string();
                    combined.push_str(&tail);
                    lines.insert(pos.line + 1 + i, combined);
                    self.cursor = Pos::new(pos.line + 1 + i, line.len());
                } else {
                    lines.insert(pos.line + 1 + i, line.to_string());
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
            let lines = Arc::make_mut(&mut self.lines);
            let ch = lines[self.cursor.line].remove(self.cursor.col - 1);
            self.cursor.col -= ch.len_utf8();
            self.push_undo(UndoOp::Delete {
                start: self.cursor,
                end: Pos::new(self.cursor.line, self.cursor.col + ch.len_utf8()),
                text: ch.to_string(),
            });
        } else if self.cursor.line > 0 {
            let lines = Arc::make_mut(&mut self.lines);
            let removed_line = lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            let col = lines[self.cursor.line].len();
            lines[self.cursor.line].push_str(&removed_line);
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
        let line_len = self.lines[self.cursor.line].len();
        if self.cursor.col < line_len {
            let lines = Arc::make_mut(&mut self.lines);
            let ch = lines[self.cursor.line].remove(self.cursor.col);
            self.push_undo(UndoOp::Delete {
                start: self.cursor,
                end: Pos::new(self.cursor.line, self.cursor.col + ch.len_utf8()),
                text: ch.to_string(),
            });
        } else if self.cursor.line + 1 < self.lines.len() {
            let lines = Arc::make_mut(&mut self.lines);
            let next = lines.remove(self.cursor.line + 1);
            lines[self.cursor.line].push_str(&next);
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
                let end = self.pos_after_insert(*pos, text);
                self.delete_range(*pos, end);
                self.cursor = *pos;
            }
            UndoOp::Delete { start, text, .. } => {
                self.cursor = *start;
                let parts: Vec<&str> = text.split('\n').collect();
                let lines = Arc::make_mut(&mut self.lines);
                if parts.len() == 1 {
                    lines[start.line].insert_str(start.col, text);
                } else {
                    let tail = lines[start.line][start.col..].to_string();
                    lines[start.line].truncate(start.col);
                    lines[start.line].push_str(parts[0]);
                    for (i, line) in parts[1..].iter().enumerate() {
                        if i == parts.len() - 2 {
                            let mut combined = line.to_string();
                            combined.push_str(&tail);
                            lines.insert(start.line + 1 + i, combined);
                        } else {
                            lines.insert(start.line + 1 + i, line.to_string());
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
                let parts: Vec<&str> = text.split('\n').collect();
                let lines = Arc::make_mut(&mut self.lines);
                if parts.len() == 1 {
                    lines[pos.line].insert_str(pos.col, text);
                    self.cursor.col += text.len();
                } else {
                    let tail = lines[pos.line][pos.col..].to_string();
                    lines[pos.line].truncate(pos.col);
                    lines[pos.line].push_str(parts[0]);
                    for (i, line) in parts[1..].iter().enumerate() {
                        if i == parts.len() - 2 {
                            let mut combined = line.to_string();
                            combined.push_str(&tail);
                            lines.insert(pos.line + 1 + i, combined);
                            self.cursor = Pos::new(pos.line + 1 + i, line.len());
                        } else {
                            lines.insert(pos.line + 1 + i, line.to_string());
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
        let trimmed = before.trim_end();
        if trimmed.is_empty() {
            self.cursor.col = 0;
        } else {
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
        let cursor_y = self.cursor.line as f32 * LINE_HEIGHT + CONTENT_PAD_Y;
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
    /// Scroll by `dy`/`dx` pixels, with viewport and content dimensions for clamping.
    Scroll {
        dy: f32,
        dx: f32,
        viewport_height: f32,
        content_height: f32,
        viewport_width: f32,
        content_width: f32,
    },
    SaveRequested,
    /// User cmd-clicked a hyperlink in the editor. Handled by the caller;
    /// `apply_action` is a no-op for this variant.
    OpenUrl(String),
}

impl EditorAction {
    /// Whether this action mutates text content (insert, delete, undo/redo).
    pub fn is_mutating(&self) -> bool {
        matches!(
            self,
            EditorAction::Insert(_)
                | EditorAction::Paste(_)
                | EditorAction::Backspace
                | EditorAction::Delete
                | EditorAction::Enter
                | EditorAction::Cut
                | EditorAction::Undo
                | EditorAction::Redo
        )
    }
}
