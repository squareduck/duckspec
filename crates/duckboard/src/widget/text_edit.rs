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
        }
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
    Scroll(f32),
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

// ── Widget ─────────────────────────────────────────────────────────────────

pub struct TextEdit<'a, M> {
    state: &'a EditorState,
    on_action: Box<dyn Fn(EditorAction) -> M + 'a>,
}

impl<'a, M> TextEdit<'a, M> {
    pub fn new(
        state: &'a EditorState,
        on_action: impl Fn(EditorAction) -> M + 'a,
    ) -> Self {
        Self {
            state,
            on_action: Box::new(on_action),
        }
    }
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for TextEdit<'a, M> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Fill);
        layout::Node::new(limits.resolve(Length::Fill, Length::Fill, Size::ZERO))
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
            let digits = digit_count(self.state.line_count());
            internal.gutter_width =
                (digits as f32) * measured + GUTTER_PAD * 2.0;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds) {
                    internal.focused = true;
                    internal.dragging = true;
                    let pos = cursor.position().unwrap();
                    let click_pos =
                        pixel_to_pos(pos, bounds, internal, self.state);
                    shell.publish((self.on_action)(EditorAction::Click(click_pos)));
                } else {
                    internal.focused = false;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if internal.dragging {
                    let drag_pos =
                        pixel_to_pos(*position, bounds, internal, self.state);
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
                    shell.publish((self.on_action)(EditorAction::Scroll(dy)));
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
                    keyboard::Key::Named(Named::Backspace) => {
                        shell.publish((self.on_action)(EditorAction::Backspace));
                    }
                    keyboard::Key::Named(Named::Delete) => {
                        shell.publish((self.on_action)(EditorAction::Delete));
                    }
                    keyboard::Key::Named(Named::Enter) => {
                        shell.publish((self.on_action)(EditorAction::Enter));
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "a" => {
                        shell.publish((self.on_action)(EditorAction::SelectAll));
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "c" => {
                        // Copy: write selection to clipboard.
                        if let Some(sel) = self.state.selection_text() {
                            clipboard.write(iced::advanced::clipboard::Kind::Standard, sel);
                        }
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "x" => {
                        // Cut: copy + delete.
                        if let Some(sel) = self.state.selection_text() {
                            clipboard.write(iced::advanced::clipboard::Kind::Standard, sel);
                            shell.publish((self.on_action)(EditorAction::Cut));
                        }
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "v" => {
                        // Paste.
                        if let Some(text) =
                            clipboard.read(iced::advanced::clipboard::Kind::Standard)
                        {
                            shell.publish((self.on_action)(EditorAction::Paste(text)));
                        }
                    }
                    keyboard::Key::Character(c)
                        if cmd && shift && c.as_str() == "z" =>
                    {
                        shell.publish((self.on_action)(EditorAction::Redo));
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "z" => {
                        shell.publish((self.on_action)(EditorAction::Undo));
                    }
                    keyboard::Key::Character(c) if cmd && c.as_str() == "s" => {
                        shell.publish((self.on_action)(EditorAction::SaveRequested));
                    }
                    _ => {
                        // Regular character input.
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
        let digits = digit_count(self.state.line_count());
        let gutter_w = (digits as f32) * cell_w + GUTTER_PAD * 2.0;
        let content_x = bounds.x + gutter_w;
        let content_w = bounds.width - gutter_w;

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

            let first_line = (self.state.scroll_y / LINE_HEIGHT).floor() as usize;
            let visible_lines =
                (bounds.height / LINE_HEIGHT).ceil() as usize + 1;
            let last_line =
                (first_line + visible_lines).min(self.state.line_count());

            let selection = self.state.selection_range();

            for i in first_line..last_line {
                let y = bounds.y + (i as f32) * LINE_HEIGHT - self.state.scroll_y;

                // Selection highlight.
                if let Some((sel_start, sel_end)) = selection {
                    if i >= sel_start.line && i <= sel_end.line {
                        let line_len = self.state.lines[i].len();
                        let sel_col_start = if i == sel_start.line {
                            sel_start.col
                        } else {
                            0
                        };
                        let sel_col_end = if i == sel_end.line {
                            sel_end.col
                        } else {
                            line_len
                        };
                        if sel_col_start < sel_col_end {
                            let sel_x =
                                content_x + CONTENT_PAD + sel_col_start as f32 * cell_w;
                            let sel_w = (sel_col_end - sel_col_start) as f32 * cell_w;
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
                                    ..theme::ACCENT
                                },
                            );
                        }
                    }
                }

                // Line number — right-aligned via format padding.
                let line_num = format!("{:>width$} ", i + 1, width = digits);
                let num_color = if i == self.state.cursor.line && internal.focused {
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

                // Line text — with syntax highlighting if available.
                let line = &self.state.lines[i];
                if !line.is_empty() {
                    let spans = self
                        .state
                        .highlight_spans
                        .as_ref()
                        .and_then(|cache| cache.get(i));

                    if let Some(spans) = spans {
                        let mut x_off = 0.0;
                        for span in spans {
                            let span_w = span.text.len() as f32 * cell_w;
                            renderer.fill_text(
                                iced::advanced::Text {
                                    content: span.text.clone(),
                                    bounds: Size::new(span_w + cell_w, LINE_HEIGHT),
                                    size: Pixels(FONT_SIZE),
                                    line_height: text::LineHeight::Absolute(
                                        Pixels(LINE_HEIGHT),
                                    ),
                                    font: Font::MONOSPACE,
                                    align_x: alignment::Horizontal::Left.into(),
                                    align_y: alignment::Vertical::Top,
                                    shaping: text::Shaping::Basic,
                                    wrapping: text::Wrapping::None,
                                },
                                Point::new(content_x + CONTENT_PAD + x_off, y),
                                span.color,
                                bounds,
                            );
                            x_off += span_w;
                        }
                    } else {
                        renderer.fill_text(
                            iced::advanced::Text {
                                content: line.clone(),
                                bounds: Size::new(content_w, LINE_HEIGHT),
                                size: Pixels(FONT_SIZE),
                                line_height: text::LineHeight::Absolute(
                                    Pixels(LINE_HEIGHT),
                                ),
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

            // Cursor.
            if internal.focused {
                let cursor_line = self.state.cursor.line;
                let cursor_col = self.state.cursor.col;
                let cy = bounds.y + cursor_line as f32 * LINE_HEIGHT
                    - self.state.scroll_y;
                let cx = content_x + CONTENT_PAD + cursor_col as f32 * cell_w;
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

fn pixel_to_pos(
    point: Point,
    bounds: Rectangle,
    internal: &InternalState,
    state: &EditorState,
) -> Pos {
    let cell_w = if internal.cell_width > 0.0 {
        internal.cell_width
    } else {
        7.8
    };
    let gutter_w = internal.gutter_width;
    let content_x = bounds.x + gutter_w + CONTENT_PAD;

    let line = ((point.y - bounds.y + state.scroll_y) / LINE_HEIGHT)
        .floor()
        .max(0.0) as usize;
    let line = line.min(state.lines.len() - 1);

    let col = if point.x > content_x {
        ((point.x - content_x) / cell_w).round() as usize
    } else {
        0
    };
    let col = col.min(state.lines[line].len());

    Pos::new(line, col)
}

/// Convenience: create the widget.
pub fn view<'a, M: Clone + 'a>(
    state: &'a EditorState,
    on_action: impl Fn(EditorAction) -> M + 'a,
) -> Element<'a, M> {
    TextEdit::new(state, on_action).into()
}
