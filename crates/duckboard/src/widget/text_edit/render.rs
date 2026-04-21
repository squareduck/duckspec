//! Iced widget implementation for the custom text editor.

use iced::advanced::layout;
use iced::advanced::mouse as adv_mouse;
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::text::{self, Paragraph as _, Renderer as TextRenderer};
use iced::advanced::widget::{self, operation, Id, Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::keyboard::key::Named;
use iced::mouse;
use iced::{
    alignment, keyboard, Border, Color, Element, Event, Length,
    Pixels, Point, Rectangle, Size, Theme,
};

use crate::theme;
use super::state::{
    block_header_color, block_kind_bg, line_bg_color,
    EditorAction, EditorState, Pos,
    CONTENT_PAD_Y, LINE_HEIGHT,
};

// ── Layout constants ───────────────────────────────────────────────────────

fn font_size() -> f32 { theme::content_size() }
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

#[derive(Debug, Default)]
struct InternalState {
    focused: bool,
    dragging: bool,
    cell_width: f32,
    gutter_width: f32,
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

    /// Convert a logical (line, col) to a visual row index.
    fn logical_to_visual(&self, line: usize, col: usize) -> usize {
        if line >= self.row_starts.len() {
            return self.total_visual_rows.saturating_sub(1);
        }
        let base = self.cum_rows[line];
        let starts = &self.row_starts[line];
        let sub = starts
            .iter()
            .rposition(|&s| col >= s)
            .unwrap_or(0);
        base + sub
    }
}

/// Compute the character offsets where each visual row starts for a single line.
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
        let break_at = (pos..end)
            .rev()
            .find(|&i| chars[i] == ' ')
            .map(|i| i + 1)
            .unwrap_or(end);
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
    placeholder: Option<String>,
    on_submit: Option<M>,
    transparent_bg: bool,
    id: Option<Id>,
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
            placeholder: None,
            on_submit: None,
            transparent_bg: false,
            id: None,
        }
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

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
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
                let max_w = limits.max().width;
                let cell_w = 7.8f32;
                let content_w = max_w - if self.show_gutter { 50.0 } else { 0.0 } - CONTENT_PAD;
                let cpr = (content_w / cell_w).floor().max(1.0) as usize;
                let wrap = WrapLayout::compute(&self.state.lines, cpr);
                wrap.total_visual_rows
            } else {
                self.state.line_count()
            };
            let height = row_count.max(1) as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0;
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
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let internal = tree.state.downcast_mut::<InternalState>();

        // Measure cell width on first event if not set.
        if internal.cell_width == 0.0 {
            let measured = measure_cell_width(renderer);
            internal.cell_width = measured;
            if self.show_gutter {
                let digits = digit_count(self.state.line_count());
                internal.gutter_width =
                    (digits as f32) * measured + GUTTER_PAD * 2.0;
            } else {
                internal.gutter_width = 0.0;
            }
        }

        // Compute wrap layout if enabled.
        let cell_w = if internal.cell_width > 0.0 { internal.cell_width } else { 7.8 };
        let wrap = if self.word_wrap {
            let content_w = bounds.width - internal.gutter_width - CONTENT_PAD;
            let cpr = (content_w / cell_w).floor().max(1.0) as usize;
            Some(WrapLayout::compute(&self.state.lines, cpr))
        } else {
            None
        };

        let content_height = if let Some(ref w) = wrap {
            w.total_visual_rows as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0
        } else {
            self.state.lines.len() as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0
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
                    internal.dragging = false;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if internal.dragging && internal.focused && let Some(pos) = cursor.position() {
                    let drag_pos =
                        pixel_to_pos_wrapped(pos, bounds, internal, self.state, wrap.as_ref(), content_height);
                    shell.publish((self.on_action)(EditorAction::Drag(drag_pos)));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                internal.dragging = false;
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.is_over(bounds) {
                    let (dy, dx) = match delta {
                        mouse::ScrollDelta::Lines { x, y } => (
                            -*y * LINE_HEIGHT * 3.0,
                            -*x * cell_w * 3.0,
                        ),
                        mouse::ScrollDelta::Pixels { x, y } => (-*y, -*x),
                    };
                    let content_w_px = if self.word_wrap {
                        0.0
                    } else {
                        let max_chars = self.state.lines.iter()
                            .map(|l| l.chars().count())
                            .max()
                            .unwrap_or(0);
                        max_chars as f32 * cell_w + CONTENT_PAD * 2.0
                    };
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
                        if !shift && let Some(msg) = self.on_submit.as_ref() {
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
                    _ if !self.read_only
                        && !cmd
                        && !modifiers.control()
                        && key_text.as_ref().is_some_and(|t| t.chars().any(|c| !c.is_control())) =>
                    {
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
                    _ => {
                        handled = false;
                    }
                }

                // Mark events we consumed as captured so app-level keyboard
                // handlers (agent chat, etc.) don't also react to them.
                if handled {
                    shell.capture_event();
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
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let internal = tree.state.downcast_ref::<InternalState>();
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

        // Compute wrap layout if needed.
        let wrap = if self.word_wrap {
            let cpr = ((content_w - CONTENT_PAD) / cell_w).floor().max(1.0) as usize;
            Some(WrapLayout::compute(&self.state.lines, cpr))
        } else {
            None
        };
        let total_visual_rows = wrap.as_ref().map_or(self.state.line_count(), |w| w.total_visual_rows);
        let content_height = total_visual_rows as f32 * LINE_HEIGHT + CONTENT_PAD_Y * 2.0;

        // Clamp scroll so we don't render past the content.
        let max_scroll = (content_height - bounds.height).max(0.0);
        let scroll_y = self.state.scroll_y.clamp(0.0, max_scroll);

        // Horizontal scroll (only when not word-wrapping).
        let scroll_x = if self.word_wrap {
            0.0
        } else {
            let max_chars = self.state.lines.iter()
                .map(|l| l.chars().count())
                .max()
                .unwrap_or(0);
            let total_content_w = max_chars as f32 * cell_w + CONTENT_PAD * 2.0;
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
                if has_blocks
                    && let Some(info) = self.state.block_line_map.get(line_idx)
                        && let Some(block) = self.state.blocks.get(info.block_idx) {
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

                // Selection highlight.
                if let Some((sel_start, sel_end)) = selection
                    && line_idx >= sel_start.line && line_idx <= sel_end.line {
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
                            let sel_x = content_x + CONTENT_PAD + vis_start as f32 * cell_w - scroll_x;
                            let sel_w = (vis_end - vis_start) as f32 * cell_w;
                            renderer::Renderer::fill_quad(
                                renderer,
                                renderer::Quad {
                                    bounds: Rectangle { x: sel_x, y, width: sel_w, height: LINE_HEIGHT },
                                    border: Border::default(),
                                    ..renderer::Quad::default()
                                },
                                Color { a: 0.3, ..theme::accent() },
                            );
                        }
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
                        let spans = self.state.highlight_spans.as_ref().and_then(|cache| cache.get(line_idx));

                        if let Some(spans) = spans {
                            let mut col = 0usize;
                            for span in spans {
                                let span_chars = span.text.chars().count();
                                let span_end = col + span_chars;
                                if span_end > char_start && col < char_end {
                                    let vis_start = col.max(char_start) - char_start;
                                    let vis_end = span_end.min(char_end) - char_start;
                                    let slice: String = span.text.chars()
                                        .skip(col.max(char_start) - col)
                                        .take(vis_end - vis_start)
                                        .collect();
                                    if !slice.is_empty() {
                                        let sw = slice.chars().count() as f32 * cell_w;
                                        let sx = content_x + CONTENT_PAD + vis_start as f32 * cell_w - scroll_x;
                                        renderer.fill_text(
                                            iced::advanced::Text {
                                                content: slice,
                                                bounds: Size::new(sw + cell_w, LINE_HEIGHT),
                                                size: Pixels(font_size()),
                                                line_height: text::LineHeight::Absolute(Pixels(LINE_HEIGHT)),
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
                                col = span_end;
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
                        bounds.y + CONTENT_PAD_Y + vrow as f32 * LINE_HEIGHT - scroll_y,
                        content_x + CONTENT_PAD + col_in_row as f32 * cell_w,
                    )
                } else {
                    (
                        bounds.y + CONTENT_PAD_Y + cursor_line as f32 * LINE_HEIGHT - scroll_y,
                        content_x + CONTENT_PAD + cursor_col as f32 * cell_w - scroll_x,
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
                    let (line_idx, sub_row) = if let Some(ref w) = wrap {
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
            // rail. Skipped in `fit_content` mode because such editors never
            // overflow internally; their parent scrollable handles scrolling.
            if !self.fit_content {
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
                            border: Border { radius: SCROLLBAR_RADIUS.into(), ..Border::default() },
                            ..renderer::Quad::default()
                        },
                        scroller_color,
                    );
                }

                if !self.word_wrap && content_w > 0.0 {
                    let max_chars = self.state.lines.iter()
                        .map(|l| l.chars().count())
                        .max()
                        .unwrap_or(0);
                    let total_content_w = max_chars as f32 * cell_w + CONTENT_PAD * 2.0;
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
                                border: Border { radius: SCROLLBAR_RADIUS.into(), ..Border::default() },
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
    if w > 0.0 {
        w
    } else {
        7.8
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
    let scroll_x = if wrap.is_some() { 0.0 } else { state.scroll_x.max(0.0) };

    let vrow = ((point.y - bounds.y - CONTENT_PAD_Y + scroll_y) / LINE_HEIGHT)
        .floor()
        .max(0.0) as usize;

    let col_in_row = if point.x + scroll_x > content_x {
        ((point.x + scroll_x - content_x) / cell_w).round() as usize
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
