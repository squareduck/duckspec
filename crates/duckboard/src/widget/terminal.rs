//! Terminal emulator widget powered by alacritty_terminal.
//!
//! Renders a PTY-backed terminal inside an Iced Canvas. The heavy lifting
//! (VT parsing, grid state) is handled by alacritty_terminal; we only do
//! cell-by-cell rendering and keyboard/mouse plumbing.

use std::cell::{Cell, RefCell};
use std::io::{Read as _, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use alacritty_terminal::event::{Event as TermEvent, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point as GridPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{self, Color as AnsiColor, NamedColor, Rgb};

use iced::keyboard;
use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::{Cache, Frame};
use iced::{Color, Element, Length, Point as IcedPoint, Rectangle, Size, Subscription, Theme};
use linkify::LinkFinder;

use crate::theme;

// ── Constants ────────────────────────────────────────────────────────────────

fn font_size() -> f32 {
    theme::content_size()
}
// Cell dimensions are measured from the configured content font via cosmic-text
// so glyphs sit flush regardless of which monospace the user picked.
fn cell_width() -> f32 {
    theme::content_cell_width()
}
fn cell_height() -> f32 {
    theme::content_cell_height()
}
const PAD_X: f32 = 6.0;
const PAD_Y: f32 = 4.0;
const DEFAULT_COLS: usize = 80;
const DEFAULT_ROWS: usize = 24;
const MAX_SCROLLBACK: usize = 10_000;

// ── Render buffer ────────────────────────────────────────────────────────────

/// A single rendered cell, pre-computed from the alacritty_terminal grid.
#[derive(Clone)]
struct RenderCell {
    grapheme: char,
    fg: Color,
    bg: Color,
    bold: bool,
    italic: bool,
    selected: bool,
}

impl Default for RenderCell {
    fn default() -> Self {
        Self {
            grapheme: ' ',
            fg: theme::text_primary(),
            bg: theme::bg_base(),
            bold: false,
            italic: false,
            selected: false,
        }
    }
}

/// Pre-computed grid of cells ready for Canvas rendering.
struct CellBuffer {
    cols: usize,
    rows: usize,
    cells: Vec<RenderCell>,
    cursor: Option<CursorInfo>,
    default_fg: Color,
    default_bg: Color,
}

#[derive(Clone)]
struct CursorInfo {
    x: usize,
    y: usize,
    visible: bool,
}

impl CellBuffer {
    fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![RenderCell::default(); cols * rows],
            cursor: None,
            default_fg: theme::text_primary(),
            default_bg: theme::bg_base(),
        }
    }

    fn cell(&self, row: usize, col: usize) -> &RenderCell {
        &self.cells[row * self.cols + col]
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.cells.resize(cols * rows, RenderCell::default());
    }
}

// ── Event listener ──────────────────────────────────────────────────────────

/// Collects device responses (PtyWrite events) from alacritty_terminal.
#[derive(Clone)]
struct Listener {
    writeback: Arc<Mutex<Vec<u8>>>,
}

impl EventListener for Listener {
    fn send_event(&self, event: TermEvent) {
        if let TermEvent::PtyWrite(text) = event
            && let Ok(mut wb) = self.writeback.lock()
        {
            wb.extend_from_slice(text.as_bytes());
        }
    }
}

// ── Terminal dimensions ─────────────────────────────────────────────────────

struct TermDimensions {
    cols: usize,
    lines: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.lines
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

// ── Default ANSI color palette ──────────────────────────────────────────────

fn default_ansi_colors() -> alacritty_terminal::term::color::Colors {
    let mut colors = alacritty_terminal::term::color::Colors::default();

    // Standard 16 ANSI colors.
    let palette: [(NamedColor, Rgb); 16] = [
        (
            NamedColor::Black,
            Rgb {
                r: 0x1e,
                g: 0x1e,
                b: 0x2e,
            },
        ),
        (
            NamedColor::Red,
            Rgb {
                r: 0xf3,
                g: 0x8b,
                b: 0xa8,
            },
        ),
        (
            NamedColor::Green,
            Rgb {
                r: 0xa6,
                g: 0xe3,
                b: 0xa1,
            },
        ),
        (
            NamedColor::Yellow,
            Rgb {
                r: 0xf9,
                g: 0xe2,
                b: 0xaf,
            },
        ),
        (
            NamedColor::Blue,
            Rgb {
                r: 0x89,
                g: 0xb4,
                b: 0xfa,
            },
        ),
        (
            NamedColor::Magenta,
            Rgb {
                r: 0xf5,
                g: 0xc2,
                b: 0xe7,
            },
        ),
        (
            NamedColor::Cyan,
            Rgb {
                r: 0x94,
                g: 0xe2,
                b: 0xd5,
            },
        ),
        (
            NamedColor::White,
            Rgb {
                r: 0xba,
                g: 0xc2,
                b: 0xde,
            },
        ),
        (
            NamedColor::BrightBlack,
            Rgb {
                r: 0x58,
                g: 0x5b,
                b: 0x70,
            },
        ),
        (
            NamedColor::BrightRed,
            Rgb {
                r: 0xf3,
                g: 0x8b,
                b: 0xa8,
            },
        ),
        (
            NamedColor::BrightGreen,
            Rgb {
                r: 0xa6,
                g: 0xe3,
                b: 0xa1,
            },
        ),
        (
            NamedColor::BrightYellow,
            Rgb {
                r: 0xf9,
                g: 0xe2,
                b: 0xaf,
            },
        ),
        (
            NamedColor::BrightBlue,
            Rgb {
                r: 0x89,
                g: 0xb4,
                b: 0xfa,
            },
        ),
        (
            NamedColor::BrightMagenta,
            Rgb {
                r: 0xf5,
                g: 0xc2,
                b: 0xe7,
            },
        ),
        (
            NamedColor::BrightCyan,
            Rgb {
                r: 0x94,
                g: 0xe2,
                b: 0xd5,
            },
        ),
        (
            NamedColor::BrightWhite,
            Rgb {
                r: 0xa6,
                g: 0xad,
                b: 0xc8,
            },
        ),
    ];
    for (name, rgb) in &palette {
        colors[*name] = Some(*rgb);
    }

    // Foreground / background / cursor.
    colors[NamedColor::Foreground] = Some(Rgb {
        r: 0xcd,
        g: 0xd6,
        b: 0xf4,
    });
    colors[NamedColor::Background] = Some(Rgb {
        r: 0x1e,
        g: 0x1e,
        b: 0x2e,
    });
    colors[NamedColor::Cursor] = Some(Rgb {
        r: 0xf5,
        g: 0xe0,
        b: 0xdc,
    });

    // Dim variants.
    colors[NamedColor::DimBlack] = Some(Rgb {
        r: 0x14,
        g: 0x14,
        b: 0x21,
    });
    colors[NamedColor::DimRed] = Some(Rgb {
        r: 0xa8,
        g: 0x61,
        b: 0x75,
    });
    colors[NamedColor::DimGreen] = Some(Rgb {
        r: 0x74,
        g: 0x9e,
        b: 0x71,
    });
    colors[NamedColor::DimYellow] = Some(Rgb {
        r: 0xae,
        g: 0x9e,
        b: 0x7a,
    });
    colors[NamedColor::DimBlue] = Some(Rgb {
        r: 0x60,
        g: 0x7e,
        b: 0xaf,
    });
    colors[NamedColor::DimMagenta] = Some(Rgb {
        r: 0xab,
        g: 0x88,
        b: 0xa2,
    });
    colors[NamedColor::DimCyan] = Some(Rgb {
        r: 0x68,
        g: 0x9e,
        b: 0x95,
    });
    colors[NamedColor::DimWhite] = Some(Rgb {
        r: 0x82,
        g: 0x88,
        b: 0x9b,
    });
    colors[NamedColor::DimForeground] = Some(Rgb {
        r: 0x90,
        g: 0x96,
        b: 0xab,
    });
    colors[NamedColor::BrightForeground] = Some(Rgb {
        r: 0xcd,
        g: 0xd6,
        b: 0xf4,
    });

    // 216-color cube (indices 16..232).
    for i in 0..216 {
        let r = if i / 36 > 0 { (i / 36) * 40 + 55 } else { 0 };
        let g = if (i / 6) % 6 > 0 {
            ((i / 6) % 6) * 40 + 55
        } else {
            0
        };
        let b = if i % 6 > 0 { (i % 6) * 40 + 55 } else { 0 };
        colors[16 + i] = Some(Rgb {
            r: r as u8,
            g: g as u8,
            b: b as u8,
        });
    }

    // Grayscale ramp (indices 232..256).
    for i in 0..24 {
        let value = (i * 10 + 8) as u8;
        colors[232 + i] = Some(Rgb {
            r: value,
            g: value,
            b: value,
        });
    }

    colors
}

// ── Terminal state ───────────────────────────────────────────────────────────

/// Owns the alacritty_terminal and all associated state.
pub struct TerminalState {
    term: Term<Listener>,
    parser: ansi::Processor,
    listener: Listener,
    /// Default ANSI color palette used when the terminal hasn't set a color.
    default_colors: alacritty_terminal::term::color::Colors,
    pty_writer: Option<Box<dyn Write + Send>>,
    buffer: CellBuffer,
    pub generation: u64,
    pub cols: usize,
    pub rows: usize,
    /// Set by Canvas draw() when widget bounds change. Applied in next feed().
    pending_resize: Cell<Option<(usize, usize)>>,
    /// Scroll delta queued by Canvas update(). Applied in next feed().
    pending_scroll: Cell<isize>,
    /// Selection event queued by Canvas update(). Applied in next feed().
    pending_selection: Cell<Option<PendingSelection>>,
    /// The portable-pty master handle, needed for resize signals.
    pty_master: Option<Box<dyn portable_pty::MasterPty + Send>>,
}

/// A selection mutation queued from the canvas thread.
#[derive(Clone, Copy)]
enum PendingSelection {
    Start(GridPoint, Side),
    Update(GridPoint, Side),
}

impl TerminalState {
    pub fn new() -> anyhow::Result<Self> {
        let listener = Listener {
            writeback: Arc::new(Mutex::new(Vec::new())),
        };

        let config = Config {
            scrolling_history: MAX_SCROLLBACK,
            ..Config::default()
        };

        let dims = TermDimensions {
            cols: DEFAULT_COLS,
            lines: DEFAULT_ROWS,
        };

        let term = Term::new(config, &dims, listener.clone());
        let parser = ansi::Processor::new();
        let default_colors = default_ansi_colors();

        Ok(Self {
            term,
            parser,
            listener,
            default_colors,
            pty_writer: None,
            buffer: CellBuffer::new(DEFAULT_COLS, DEFAULT_ROWS),
            generation: 0,
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            pending_resize: Cell::new(None),
            pending_scroll: Cell::new(0),
            pending_selection: Cell::new(None),
            pty_master: None,
        })
    }

    /// Feed raw bytes from the PTY into the terminal emulator.
    pub fn feed(&mut self, bytes: &[u8]) {
        // Apply any pending resize before processing new bytes.
        if let Some((cols, rows)) = self.pending_resize.take() {
            self.resize(cols, rows);
        }
        // Apply any pending scroll.
        let scroll = self.pending_scroll.replace(0);
        if scroll != 0 {
            self.term.scroll_display(Scroll::Delta(scroll as i32));
        }
        // Apply any pending selection mutation.
        self.apply_pending_selection();
        self.parser.advance(&mut self.term, bytes);
        self.flush_writeback();
        self.rebuild_buffer();
        self.generation += 1;
    }

    /// Apply pending scroll / selection without waiting for PTY output.
    pub fn apply_scroll(&mut self) {
        let mut dirty = false;
        let scroll = self.pending_scroll.replace(0);
        if scroll != 0 {
            self.term.scroll_display(Scroll::Delta(scroll as i32));
            dirty = true;
        }
        if self.apply_pending_selection() {
            dirty = true;
        }
        if dirty {
            self.rebuild_buffer();
            self.generation += 1;
        }
        if let Some((cols, rows)) = self.pending_resize.take() {
            self.resize(cols, rows);
        }
    }

    fn apply_pending_selection(&mut self) -> bool {
        let Some(ev) = self.pending_selection.take() else {
            return false;
        };
        match ev {
            PendingSelection::Start(pt, side) => {
                self.term.selection = Some(Selection::new(SelectionType::Simple, pt, side));
            }
            PendingSelection::Update(pt, side) => {
                if let Some(ref mut sel) = self.term.selection {
                    sel.update(pt, side);
                }
            }
        }
        true
    }

    /// Set the PTY writer (called once when the PTY subscription is ready).
    pub fn set_writer(&mut self, writer: Box<dyn Write + Send>) {
        self.pty_writer = Some(writer);
    }

    /// Set the PTY master handle for resize signals.
    pub fn set_master(&mut self, master: Box<dyn portable_pty::MasterPty + Send>) {
        self.pty_master = Some(master);
    }

    /// Request a resize — called from Canvas draw() via interior mutability.
    pub fn request_resize(&self, cols: usize, rows: usize) {
        if cols != self.cols || rows != self.rows {
            self.pending_resize.set(Some((cols, rows)));
        }
    }

    /// Queue a scroll delta — called from Canvas update() on mouse wheel.
    pub fn request_scroll(&self, delta: isize) {
        let current = self.pending_scroll.get();
        self.pending_scroll.set(current + delta);
    }

    /// Begin a new selection at the given canvas-local pixel coordinates.
    pub fn queue_selection_start(&self, px: f32, py: f32) {
        let (pt, side) = self.point_from_canvas(px, py);
        self.pending_selection
            .set(Some(PendingSelection::Start(pt, side)));
    }

    /// Extend the active selection to the given canvas-local pixel coordinates.
    pub fn queue_selection_update(&self, px: f32, py: f32) {
        let (pt, side) = self.point_from_canvas(px, py);
        self.pending_selection
            .set(Some(PendingSelection::Update(pt, side)));
    }

    /// Return the currently selected text, if any.
    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|s| !s.is_empty())
    }

    /// Write text to the PTY, honoring bracketed paste mode.
    pub fn paste_text(&mut self, text: &str) {
        let Some(ref mut writer) = self.pty_writer else {
            return;
        };
        let bracketed = self.term.mode().contains(TermMode::BRACKETED_PASTE);
        if bracketed {
            let _ = writer.write_all(b"\x1b[200~");
            let _ = writer.write_all(text.as_bytes());
            let _ = writer.write_all(b"\x1b[201~");
        } else {
            let _ = writer.write_all(text.as_bytes());
        }
        let _ = writer.flush();
    }

    /// Convert a canvas-local pixel position to a grid point (clamped to grid).
    fn point_from_canvas(&self, px: f32, py: f32) -> (GridPoint, Side) {
        let cols = self.cols.max(1);
        let rows = self.rows.max(1);
        let col_float = ((px - PAD_X) / cell_width()).max(0.0);
        let col = (col_float as usize).min(cols - 1);
        let side = if col_float - col as f32 <= 0.5 {
            Side::Left
        } else {
            Side::Right
        };
        let row_float = ((py - PAD_Y) / cell_height()).max(0.0);
        let row = (row_float as usize).min(rows - 1);
        let display_offset = self.term.grid().display_offset() as i32;
        let line = (row as i32) - display_offset;
        (GridPoint::new(Line(line), Column(col)), side)
    }

    /// Encode a keyboard event and write it to the PTY.
    pub fn write_key(&mut self, key: keyboard::Key, mods: keyboard::Modifiers, text: Option<&str>) {
        let Some(ref mut writer) = self.pty_writer else {
            return;
        };

        if let Some(encoded) = encode_key(&self.term, key, mods, text) {
            let _ = writer.write_all(encoded.as_bytes());
            let _ = writer.flush();
        }
    }

    /// Resize the terminal grid and notify the PTY child.
    fn resize(&mut self, cols: usize, rows: usize) {
        if (cols == self.cols && rows == self.rows) || cols == 0 || rows == 0 {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let dims = TermDimensions { cols, lines: rows };
        self.term.resize(dims);
        self.buffer.resize(cols, rows);

        // Notify PTY child of new size (sends SIGWINCH).
        if let Some(ref master) = self.pty_master {
            let _ = master.resize(portable_pty::PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: (cols as f32 * cell_width()) as u16,
                pixel_height: (rows as f32 * cell_height()) as u16,
            });
        }

        self.rebuild_buffer();
        self.generation += 1;
    }

    fn flush_writeback(&mut self) {
        let bytes: Vec<u8> = {
            let Ok(mut wb) = self.listener.writeback.lock() else {
                return;
            };
            wb.drain(..).collect()
        };
        if !bytes.is_empty()
            && let Some(ref mut writer) = self.pty_writer
        {
            let _ = writer.write_all(&bytes);
            let _ = writer.flush();
        }
    }

    fn rebuild_buffer(&mut self) {
        let selection_range = self
            .term
            .selection
            .as_ref()
            .and_then(|s| s.to_range(&self.term));
        let content = self.term.renderable_content();
        let term_colors = content.colors;
        let fallback = &self.default_colors;

        // Resolve default fg/bg.
        let default_fg = resolve_color(
            AnsiColor::Named(NamedColor::Foreground),
            term_colors,
            fallback,
        );
        let default_bg = resolve_color(
            AnsiColor::Named(NamedColor::Background),
            term_colors,
            fallback,
        );
        self.buffer.default_fg = default_fg;
        self.buffer.default_bg = default_bg;

        // Sync buffer dimensions.
        let grid = self.term.grid();
        let snap_cols = grid.columns();
        let snap_rows = grid.screen_lines();
        if snap_cols != self.buffer.cols || snap_rows != self.buffer.rows {
            self.buffer.resize(snap_cols, snap_rows);
        }

        // Update cursor info.
        let cursor = &content.cursor;
        let cursor_visible = cursor.shape != alacritty_terminal::vte::ansi::CursorShape::Hidden;
        self.buffer.cursor = if cursor_visible {
            let display_offset = content.display_offset as i32;
            let cursor_line = cursor.point.line.0 + display_offset;
            if cursor_line >= 0 && (cursor_line as usize) < snap_rows {
                Some(CursorInfo {
                    x: cursor.point.column.0,
                    y: cursor_line as usize,
                    visible: true,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Clear buffer before populating.
        for cell in &mut self.buffer.cells {
            *cell = RenderCell::default();
        }

        // Iterate visible cells.
        for indexed in content.display_iter {
            let line = indexed.point.line.0 + content.display_offset as i32;
            if line < 0 || line as usize >= snap_rows {
                continue;
            }
            let row_idx = line as usize;
            let col_idx = indexed.point.column.0;
            if col_idx >= snap_cols {
                continue;
            }

            let cell = &indexed.cell;

            // Wide-char spacer cells carry the same fg/bg as their leading
            // wide char (alacritty writes them with `write_at_cursor(' ')` and
            // the active template), so we fall through for bg painting and let
            // the ' ' grapheme check below skip the glyph.
            let grapheme = cell.c;
            let bold = cell.flags.contains(Flags::BOLD);
            let italic = cell.flags.contains(Flags::ITALIC);
            let inverse = cell.flags.contains(Flags::INVERSE);

            let mut fg = resolve_color(cell.fg, term_colors, fallback);
            let mut bg = resolve_color(cell.bg, term_colors, fallback);

            if inverse {
                std::mem::swap(&mut fg, &mut bg);
            }

            let selected = selection_range
                .as_ref()
                .is_some_and(|r| r.contains(indexed.point));

            let idx = row_idx * snap_cols + col_idx;
            if idx < self.buffer.cells.len() {
                self.buffer.cells[idx] = RenderCell {
                    grapheme,
                    fg,
                    bg,
                    bold,
                    italic,
                    selected,
                };
            }
        }
    }
}

/// Resolve an alacritty Color enum to an iced Color.
/// Checks the terminal's live colors first, then falls back to our default palette.
fn resolve_color(
    color: AnsiColor,
    colors: &alacritty_terminal::term::color::Colors,
    fallback: &alacritty_terminal::term::color::Colors,
) -> Color {
    match color {
        AnsiColor::Named(name) => {
            let rgb = colors[name].or(fallback[name]);
            if let Some(rgb) = rgb {
                Color::from_rgb8(rgb.r, rgb.g, rgb.b)
            } else {
                match name {
                    NamedColor::Foreground | NamedColor::BrightForeground => theme::text_primary(),
                    NamedColor::Background => theme::bg_base(),
                    _ => theme::text_primary(),
                }
            }
        }
        AnsiColor::Spec(rgb) => Color::from_rgb8(rgb.r, rgb.g, rgb.b),
        AnsiColor::Indexed(idx) => {
            let rgb = colors[idx as usize].or(fallback[idx as usize]);
            if let Some(rgb) = rgb {
                Color::from_rgb8(rgb.r, rgb.g, rgb.b)
            } else {
                theme::text_primary()
            }
        }
    }
}

// ── Modifier tracking ────────────────────────────────────────────────────────

/// Process-wide live modifier state, set by the global key event handler in
/// `main.rs`. Mouse events in Iced canvases don't carry modifiers, so we mirror
/// the latest known state here for canvas widgets to query.
static CURRENT_MODIFIERS: AtomicU32 = AtomicU32::new(0);

const MOD_SHIFT: u32 = 1 << 0;
const MOD_CTRL: u32 = 1 << 1;
const MOD_ALT: u32 = 1 << 2;
const MOD_LOGO: u32 = 1 << 3;

/// Update the live modifier state. Call from a global keyboard event handler.
pub fn set_current_modifiers(mods: keyboard::Modifiers) {
    let mut bits = 0u32;
    if mods.shift() {
        bits |= MOD_SHIFT;
    }
    if mods.control() {
        bits |= MOD_CTRL;
    }
    if mods.alt() {
        bits |= MOD_ALT;
    }
    if mods.logo() {
        bits |= MOD_LOGO;
    }
    CURRENT_MODIFIERS.store(bits, Ordering::Relaxed);
}

/// Read the live modifier state. Returns the current state of the keyboard
/// modifiers as last seen by the global key event handler. Available to other
/// widgets (text_edit) that need modifier-aware mouse behaviors.
pub fn current_modifiers() -> keyboard::Modifiers {
    let bits = CURRENT_MODIFIERS.load(Ordering::Relaxed);
    let mut m = keyboard::Modifiers::empty();
    if bits & MOD_SHIFT != 0 {
        m |= keyboard::Modifiers::SHIFT;
    }
    if bits & MOD_CTRL != 0 {
        m |= keyboard::Modifiers::CTRL;
    }
    if bits & MOD_ALT != 0 {
        m |= keyboard::Modifiers::ALT;
    }
    if bits & MOD_LOGO != 0 {
        m |= keyboard::Modifiers::LOGO;
    }
    m
}

// ── Canvas rendering ─────────────────────────────────────────────────────────

/// Events the terminal canvas publishes to its parent.
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// Triggers a re-render (mouse scroll/click/drag, hover changes).
    Redraw,
    /// User cmd-clicked a hyperlink in the terminal output.
    OpenUrl(String),
}

/// A URL detected in the visible buffer that the cursor is currently over.
#[derive(Clone, Debug, PartialEq, Eq)]
struct LinkHover {
    row: usize,
    start_col: usize,
    end_col: usize,
    url: String,
}

/// The Canvas Program that reads the pre-computed cell buffer and draws it.
pub struct TerminalCanvas<'a> {
    state: &'a TerminalState,
}

/// Internal canvas state — geometry cache + drag tracking.
pub struct CanvasState {
    cache: Cache,
    last_generation: Cell<u64>,
    dragging: Cell<bool>,
    /// Residual pixel delta from trackpad wheel events that did not amount to
    /// a full line. Carried across events so small per-frame deltas eventually
    /// trigger a scroll instead of being silently dropped.
    scroll_accum: Cell<f32>,
    /// URL currently under the mouse while a modifier is held. Drives the
    /// underline overlay and the pointer cursor.
    hover: RefCell<Option<LinkHover>>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            cache: Cache::default(),
            last_generation: Cell::new(0),
            dragging: Cell::new(false),
            scroll_accum: Cell::new(0.0),
            hover: RefCell::new(None),
        }
    }
}

impl<'a> canvas::Program<TerminalEvent> for TerminalCanvas<'a> {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<TerminalEvent>> {
        match event {
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return None;
                }
                // macOS natural scrolling: swipe up (positive y) reveals older
                // scrollback, which in alacritty is a positive Scroll::Delta.
                let ch = cell_height();
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => {
                        state.scroll_accum.set(0.0);
                        *y as isize
                    }
                    mouse::ScrollDelta::Pixels { y, .. } => {
                        // Accumulate sub-line pixel deltas so trackpad scroll
                        // doesn't feel stuck.
                        let accum = state.scroll_accum.get() + *y;
                        let whole = (accum / ch).trunc();
                        state.scroll_accum.set(accum - whole * ch);
                        whole as isize
                    }
                };
                if lines != 0 {
                    self.state.request_scroll(lines);
                    Some(canvas::Action::publish(TerminalEvent::Redraw).and_capture())
                } else {
                    // Still capture so the wheel event doesn't bubble to a
                    // parent scrollable while we're busy accumulating.
                    Some(canvas::Action::capture())
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_in(bounds)?;
                // Cmd-click on a hovered link opens it instead of starting a
                // selection.
                if current_modifiers().command()
                    && let Some(hover) = state.hover.borrow().clone()
                    && self.point_over_hover(pos, &hover)
                {
                    return Some(
                        canvas::Action::publish(TerminalEvent::OpenUrl(hover.url)).and_capture(),
                    );
                }
                self.state.queue_selection_start(pos.x, pos.y);
                state.dragging.set(true);
                Some(canvas::Action::publish(TerminalEvent::Redraw).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.dragging.get()
                    && let Some(pos) = cursor.position_from(bounds.position())
                {
                    // Auto-scroll when the user drags past the top or bottom
                    // of the canvas so selection can extend beyond the visible
                    // viewport.
                    let ch = cell_height();
                    if pos.y < 0.0 {
                        let overshoot = ((-pos.y) / ch).ceil().max(1.0) as isize;
                        self.state.request_scroll(overshoot);
                    } else if pos.y > bounds.height {
                        let overshoot = ((pos.y - bounds.height) / ch).ceil().max(1.0) as isize;
                        self.state.request_scroll(-overshoot);
                    }
                    self.state.queue_selection_update(pos.x, pos.y);
                    return Some(canvas::Action::publish(TerminalEvent::Redraw).and_capture());
                }
                // Hover detection while cmd is held.
                let cursor_pos = cursor.position_in(bounds);
                let want_hover = current_modifiers().command() && cursor_pos.is_some();
                let new_hover = if want_hover {
                    self.detect_link_at(cursor_pos.unwrap())
                } else {
                    None
                };
                let mut hover_slot = state.hover.borrow_mut();
                if *hover_slot != new_hover {
                    *hover_slot = new_hover;
                    return Some(canvas::Action::publish(TerminalEvent::Redraw));
                }
                None
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging.get() {
                    state.dragging.set(false);
                    return Some(canvas::Action::publish(TerminalEvent::Redraw).and_capture());
                }
                None
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if let Some(hover) = state.hover.borrow().as_ref()
            && let Some(pos) = cursor.position_in(bounds)
            && self.point_over_hover(pos, hover)
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<iced::Renderer>> {
        let buffer = &self.state.buffer;

        // Request resize if widget bounds changed (account for padding).
        let cw = cell_width();
        let ch = cell_height();
        let inner_w = (bounds.width - PAD_X * 2.0).max(cw);
        let inner_h = (bounds.height - PAD_Y * 2.0).max(ch);
        let desired_cols = (inner_w / cw).floor().max(1.0) as usize;
        let desired_rows = (inner_h / ch).floor().max(1.0) as usize;
        self.state.request_resize(desired_cols, desired_rows);

        // Invalidate cache when terminal content changes.
        if self.state.generation != state.last_generation.get() {
            state.cache.clear();
            state.last_generation.set(self.state.generation);
        }

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            // Fill background.
            frame.fill_rectangle(IcedPoint::ORIGIN, bounds.size(), buffer.default_bg);

            let cols = buffer.cols;
            let rows = buffer.rows;

            for row in 0..rows {
                for col in 0..cols {
                    let cell = buffer.cell(row, col);
                    let x = PAD_X + col as f32 * cw;
                    let y = PAD_Y + row as f32 * ch;

                    // Draw cell background if it differs from the terminal default.
                    if cell.bg != buffer.default_bg {
                        frame.fill_rectangle(IcedPoint::new(x, y), Size::new(cw, ch), cell.bg);
                    }

                    // Overlay selection tint.
                    if cell.selected {
                        frame.fill_rectangle(
                            IcedPoint::new(x, y),
                            Size::new(cw, ch),
                            Color {
                                a: 0.35,
                                ..theme::accent()
                            },
                        );
                    }

                    // Draw character.
                    if cell.grapheme != ' ' && cell.grapheme != '\0' {
                        let font = if cell.bold {
                            iced::Font {
                                weight: iced::font::Weight::Bold,
                                style: if cell.italic {
                                    iced::font::Style::Italic
                                } else {
                                    iced::font::Style::Normal
                                },
                                ..theme::content_font()
                            }
                        } else if cell.italic {
                            iced::Font {
                                style: iced::font::Style::Italic,
                                ..theme::content_font()
                            }
                        } else {
                            theme::content_font()
                        };

                        frame.fill_text(canvas::Text {
                            content: cell.grapheme.to_string(),
                            position: IcedPoint::new(x, y),
                            color: cell.fg,
                            size: iced::Pixels(font_size()),
                            font,
                            ..canvas::Text::default()
                        });
                    }
                }
            }

            // Draw cursor.
            if let Some(ref cursor) = buffer.cursor
                && cursor.visible
            {
                let cx = PAD_X + cursor.x as f32 * cw;
                let cy = PAD_Y + cursor.y as f32 * ch;
                frame.fill_rectangle(
                    IcedPoint::new(cx, cy),
                    Size::new(cw, ch),
                    Color {
                        a: 0.5,
                        ..theme::text_primary()
                    },
                );
            }
        });

        // Hover underline (drawn outside the cache so it tracks mouse moves).
        let mut overlays = vec![geometry];
        if let Some(hover) = state.hover.borrow().as_ref() {
            let mut overlay = Frame::new(renderer, bounds.size());
            let x = PAD_X + hover.start_col as f32 * cw;
            let y = PAD_Y + hover.row as f32 * ch + ch - 1.0;
            let width = (hover.end_col - hover.start_col) as f32 * cw;
            overlay.fill_rectangle(
                IcedPoint::new(x, y),
                Size::new(width, 1.0),
                theme::accent(),
            );
            overlays.push(overlay.into_geometry());
        }
        overlays
    }
}

impl<'a> TerminalCanvas<'a> {
    /// True when `pos` (canvas-local pixels) lies within the underlined cells
    /// of `hover`. Used both to gate cmd-click and to decide cursor shape.
    fn point_over_hover(&self, pos: IcedPoint, hover: &LinkHover) -> bool {
        let cw = cell_width();
        let ch = cell_height();
        let col_f = (pos.x - PAD_X) / cw;
        let row_f = (pos.y - PAD_Y) / ch;
        if col_f < 0.0 || row_f < 0.0 {
            return false;
        }
        let col = col_f as usize;
        let row = row_f as usize;
        row == hover.row && col >= hover.start_col && col < hover.end_col
    }

    /// Detect a URL under canvas-local pixel position `pos`. Scans only the
    /// hovered visible row (no wrap-aware joining yet — wrapped URLs won't be
    /// detected).
    fn detect_link_at(&self, pos: IcedPoint) -> Option<LinkHover> {
        let cw = cell_width();
        let ch = cell_height();
        let col_f = (pos.x - PAD_X) / cw;
        let row_f = (pos.y - PAD_Y) / ch;
        if col_f < 0.0 || row_f < 0.0 {
            return None;
        }
        let col = col_f as usize;
        let row = row_f as usize;
        let buffer = &self.state.buffer;
        if row >= buffer.rows || col >= buffer.cols {
            return None;
        }

        let cols = buffer.cols;
        let line: String = (0..cols).map(|c| buffer.cell(row, c).grapheme).collect();

        let finder = LinkFinder::new();
        for link in finder.links(&line) {
            let start_col = line[..link.start()].chars().count();
            let end_col = line[..link.end()].chars().count();
            if col >= start_col && col < end_col {
                return Some(LinkHover {
                    row,
                    start_col,
                    end_col,
                    url: link.as_str().to_string(),
                });
            }
        }
        None
    }
}

/// Build a terminal view element from the current terminal state.
pub fn view_terminal(state: &TerminalState) -> Element<'_, TerminalEvent> {
    canvas::Canvas::new(TerminalCanvas { state })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── PTY subscription ─────────────────────────────────────────────────────────

/// Events produced by the PTY background reader.
#[derive(Clone)]
pub enum PtyEvent {
    /// PTY is ready — carries the writer and master handles.
    Ready(PtyWriter, PtyMaster),
    /// Raw bytes from the PTY child process.
    Output(Vec<u8>),
    /// The child process exited.
    Exited,
}

/// A clonable wrapper around the PTY master for resize signals.
#[derive(Clone)]
pub struct PtyMaster {
    inner: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
}

impl PtyMaster {
    pub fn into_master(self) -> Box<dyn portable_pty::MasterPty + Send> {
        match Arc::try_unwrap(self.inner) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(_arc) => {
                panic!("PtyMaster still has multiple references");
            }
        }
    }
}

impl std::fmt::Debug for PtyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(..) => write!(f, "PtyEvent::Ready"),
            Self::Output(bytes) => write!(f, "PtyEvent::Output({} bytes)", bytes.len()),
            Self::Exited => write!(f, "PtyEvent::Exited"),
        }
    }
}

/// A clonable wrapper around the PTY writer so it can live in Iced messages.
#[derive(Clone)]
pub struct PtyWriter {
    inner: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl PtyWriter {
    pub fn into_writer(self) -> Box<dyn Write + Send> {
        match Arc::try_unwrap(self.inner) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => Box::new(ArcWriter(arc)),
        }
    }
}

struct ArcWriter(Arc<Mutex<Box<dyn Write + Send>>>);
impl Write for ArcWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

/// Create a subscription that spawns a PTY child and streams its output.
///
/// `cwd`, when `Some`, is used as the working directory of the spawned shell.
/// It is part of the subscription identity, so changing the project root
/// tears down the old shell and spawns a fresh one in the new directory.
pub fn pty_subscription(
    key: String,
    cwd: Option<std::path::PathBuf>,
) -> Subscription<(String, PtyEvent)> {
    Subscription::run_with((key, cwd), |(key, cwd)| {
        use iced::futures::StreamExt;
        let key = key.clone();
        pty_worker(cwd.clone()).map(move |e| (key.clone(), e))
    })
}

fn pty_worker(cwd: Option<std::path::PathBuf>) -> impl iced::futures::Stream<Item = PtyEvent> {
    iced::stream::channel(
        256,
        |mut sender: iced::futures::channel::mpsc::Sender<PtyEvent>| async move {
            use iced::futures::SinkExt;

            // Open PTY.
            let pty_system = portable_pty::native_pty_system();
            let pair = match pty_system.openpty(portable_pty::PtySize {
                rows: DEFAULT_ROWS as u16,
                cols: DEFAULT_COLS as u16,
                pixel_width: 0,
                pixel_height: 0,
            }) {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::error!("failed to open PTY: {e}");
                    return;
                }
            };

            // Spawn default shell.
            let mut cmd = portable_pty::CommandBuilder::new_default_prog();
            // Tell the child shell what terminal we emulate. Without this,
            // GUI launches (where the parent process has no TERM from
            // launchd) leave readline in a no-cursor-control mode and the
            // backspace echo degenerates to a bare space — cursor appears
            // to advance forward. xterm-256color is universally available
            // in terminfo; COLORTERM lets apps opt into 24-bit color.
            cmd.env("TERM", "xterm-256color");
            cmd.env("COLORTERM", "truecolor");
            cmd.env("PROMPT_EOL_MARK", "");
            if let Some(ref path) = cwd {
                cmd.cwd(path);
            }
            let _child = match pair.slave.spawn_command(cmd) {
                Ok(child) => child,
                Err(e) => {
                    tracing::error!("failed to spawn shell: {e}");
                    return;
                }
            };
            drop(pair.slave);

            // Send writer back to main thread.
            let writer = match pair.master.take_writer() {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("failed to take PTY writer: {e}");
                    return;
                }
            };
            let pty_writer = PtyWriter {
                inner: Arc::new(Mutex::new(writer)),
            };

            // Clone reader before sending master away.
            let mut reader = match pair.master.try_clone_reader() {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("failed to clone PTY reader: {e}");
                    return;
                }
            };

            // Send writer and master back to main thread.
            let pty_master = PtyMaster {
                inner: Arc::new(Mutex::new(pair.master)),
            };
            let _ = sender.send(PtyEvent::Ready(pty_writer, pty_master)).await;

            // Read from PTY in a background thread, forward via channel.
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<Vec<u8>>>();

            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            let _ = tx.send(None);
                            break;
                        }
                        Ok(n) => {
                            if tx.send(Some(buf[..n].to_vec())).is_err() {
                                break;
                            }
                        }
                        Err(_) => {
                            let _ = tx.send(None);
                            break;
                        }
                    }
                }
            });

            // Forward PTY output as Iced messages.
            while let Some(data) = rx.recv().await {
                match data {
                    Some(bytes) => {
                        if sender.send(PtyEvent::Output(bytes)).await.is_err() {
                            break;
                        }
                    }
                    None => {
                        let _ = sender.send(PtyEvent::Exited).await;
                        break;
                    }
                }
            }
        },
    )
}

// ── Key encoding ─────────────────────────────────────────────────────────────

/// Translate an Iced keyboard event into bytes for the PTY.
fn encode_key(
    term: &Term<Listener>,
    key: keyboard::Key,
    mods: keyboard::Modifiers,
    text: Option<&str>,
) -> Option<String> {
    let mode = *term.mode();
    let app_cursor = mode.contains(TermMode::APP_CURSOR);

    // Named keys first.
    if let keyboard::Key::Named(named) = &key {
        let seq = match named {
            keyboard::key::Named::Enter => {
                if mods.alt() {
                    "\x1b\r".into()
                } else {
                    "\r".into()
                }
            }
            keyboard::key::Named::Tab => {
                if mods.shift() {
                    "\x1b[Z".into()
                } else {
                    "\t".into()
                }
            }
            keyboard::key::Named::Backspace => {
                if mods.alt() {
                    "\x1b\x7f".into()
                } else if mods.control() {
                    "\x08".into()
                } else {
                    "\x7f".into()
                }
            }
            keyboard::key::Named::Escape => "\x1b".into(),
            keyboard::key::Named::Space => {
                if mods.control() {
                    "\x00".into()
                } else {
                    " ".into()
                }
            }
            keyboard::key::Named::ArrowUp => arrow_key('A', app_cursor, mods),
            keyboard::key::Named::ArrowDown => arrow_key('B', app_cursor, mods),
            keyboard::key::Named::ArrowRight => arrow_key('C', app_cursor, mods),
            keyboard::key::Named::ArrowLeft => arrow_key('D', app_cursor, mods),
            keyboard::key::Named::Home => {
                if app_cursor {
                    "\x1bOH".into()
                } else {
                    "\x1b[H".into()
                }
            }
            keyboard::key::Named::End => {
                if app_cursor {
                    "\x1bOF".into()
                } else {
                    "\x1b[F".into()
                }
            }
            keyboard::key::Named::PageUp => "\x1b[5~".into(),
            keyboard::key::Named::PageDown => "\x1b[6~".into(),
            keyboard::key::Named::Insert => "\x1b[2~".into(),
            keyboard::key::Named::Delete => "\x1b[3~".into(),
            keyboard::key::Named::F1 => "\x1bOP".into(),
            keyboard::key::Named::F2 => "\x1bOQ".into(),
            keyboard::key::Named::F3 => "\x1bOR".into(),
            keyboard::key::Named::F4 => "\x1bOS".into(),
            keyboard::key::Named::F5 => "\x1b[15~".into(),
            keyboard::key::Named::F6 => "\x1b[17~".into(),
            keyboard::key::Named::F7 => "\x1b[18~".into(),
            keyboard::key::Named::F8 => "\x1b[19~".into(),
            keyboard::key::Named::F9 => "\x1b[20~".into(),
            keyboard::key::Named::F10 => "\x1b[21~".into(),
            keyboard::key::Named::F11 => "\x1b[23~".into(),
            keyboard::key::Named::F12 => "\x1b[24~".into(),
            _ => return None,
        };
        return Some(seq);
    }

    // Character keys.
    if let keyboard::Key::Character(ch) = &key {
        if mods.control() {
            // Ctrl+letter → control character.
            if let Some(c) = ch.chars().next() {
                let ctrl = match c.to_ascii_lowercase() {
                    c @ 'a'..='z' => Some((c as u8 - b'a' + 1) as char),
                    '[' | '3' => Some('\x1b'),
                    '\\' | '4' => Some('\x1c'),
                    ']' | '5' => Some('\x1d'),
                    '6' => Some('\x1e'),
                    '/' | '7' => Some('\x1f'),
                    '8' => Some('\x7f'),
                    ' ' | '2' | '@' => Some('\x00'),
                    _ => None,
                };
                if let Some(ctrl_char) = ctrl {
                    let mut s = String::new();
                    if mods.alt() {
                        s.push('\x1b');
                    }
                    s.push(ctrl_char);
                    return Some(s);
                }
            }
        }

        // Alt+key → ESC prefix.
        if mods.alt() {
            if let Some(t) = text {
                return Some(format!("\x1b{t}"));
            } else {
                return Some(format!("\x1b{ch}"));
            }
        }

        // Plain text.
        if let Some(t) = text {
            return Some(t.to_string());
        }
        return Some(ch.to_string());
    }

    None
}

/// Encode an arrow key with optional modifier support.
fn arrow_key(direction: char, app_cursor: bool, mods: keyboard::Modifiers) -> String {
    let modifier = csi_modifier(mods);
    if modifier > 1 {
        format!("\x1b[1;{modifier}{direction}")
    } else if app_cursor {
        format!("\x1bO{direction}")
    } else {
        format!("\x1b[{direction}")
    }
}

/// Compute the CSI modifier parameter from keyboard modifiers.
fn csi_modifier(mods: keyboard::Modifiers) -> u8 {
    let mut m: u8 = 1;
    if mods.shift() {
        m += 1;
    }
    if mods.alt() {
        m += 2;
    }
    if mods.control() {
        m += 4;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_row(term: &Term<Listener>, row: usize, width: usize) -> String {
        let grid = term.grid();
        (0..width)
            .map(|c| grid[Line(row as i32)][Column(c)].c)
            .collect()
    }

    fn cursor_col(term: &Term<Listener>) -> usize {
        term.renderable_content().cursor.point.column.0
    }

    /// Guards the assumption behind the TERM fix: given a proper BS-SP-BS
    /// echo (what the shell emits when it thinks it's talking to a real
    /// terminal), the grid erases left-to-right and the cursor walks back.
    /// If this ever regresses, the on-screen symptom is that Backspace
    /// appears to advance the cursor forward.
    #[test]
    fn readline_style_backspace_echo_erases_char() {
        let mut s = TerminalState::new().unwrap();
        s.feed(b"abc");
        assert_eq!(cursor_col(&s.term), 3);
        assert_eq!(grid_row(&s.term, 0, 5), "abc  ");

        s.feed(b"\x08 \x08");
        assert_eq!(cursor_col(&s.term), 2);
        assert_eq!(grid_row(&s.term, 0, 5), "ab   ");

        s.feed(b"\x08 \x08");
        assert_eq!(cursor_col(&s.term), 1);
        assert_eq!(grid_row(&s.term, 0, 5), "a    ");

        s.feed(b"\x08 \x08");
        assert_eq!(cursor_col(&s.term), 0);
        assert_eq!(grid_row(&s.term, 0, 5), "     ");
    }
}
