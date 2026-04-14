//! Terminal emulator widget powered by alacritty_terminal.
//!
//! Renders a PTY-backed terminal inside an Iced Canvas. The heavy lifting
//! (VT parsing, grid state) is handled by alacritty_terminal; we only do
//! cell-by-cell rendering and keyboard/mouse plumbing.

use std::cell::Cell;
use std::io::{Read as _, Write};
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event as TermEvent, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{self, Color as AnsiColor, NamedColor, Rgb};

use iced::keyboard;
use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::Cache;
use iced::{Color, Element, Length, Point as IcedPoint, Rectangle, Size, Subscription, Theme};

use crate::theme;

// ── Constants ────────────────────────────────────────────────────────────────

const FONT_SIZE: f32 = theme::FONT_MD;
const CELL_WIDTH: f32 = 8.4;
const CELL_HEIGHT: f32 = 18.0;
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
}

impl Default for RenderCell {
    fn default() -> Self {
        Self {
            grapheme: ' ',
            fg: theme::text_primary(),
            bg: theme::bg_base(),
            bold: false,
            italic: false,
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
        (NamedColor::Black, Rgb { r: 0x1e, g: 0x1e, b: 0x2e }),
        (NamedColor::Red, Rgb { r: 0xf3, g: 0x8b, b: 0xa8 }),
        (NamedColor::Green, Rgb { r: 0xa6, g: 0xe3, b: 0xa1 }),
        (NamedColor::Yellow, Rgb { r: 0xf9, g: 0xe2, b: 0xaf }),
        (NamedColor::Blue, Rgb { r: 0x89, g: 0xb4, b: 0xfa }),
        (NamedColor::Magenta, Rgb { r: 0xf5, g: 0xc2, b: 0xe7 }),
        (NamedColor::Cyan, Rgb { r: 0x94, g: 0xe2, b: 0xd5 }),
        (NamedColor::White, Rgb { r: 0xba, g: 0xc2, b: 0xde }),
        (NamedColor::BrightBlack, Rgb { r: 0x58, g: 0x5b, b: 0x70 }),
        (NamedColor::BrightRed, Rgb { r: 0xf3, g: 0x8b, b: 0xa8 }),
        (NamedColor::BrightGreen, Rgb { r: 0xa6, g: 0xe3, b: 0xa1 }),
        (NamedColor::BrightYellow, Rgb { r: 0xf9, g: 0xe2, b: 0xaf }),
        (NamedColor::BrightBlue, Rgb { r: 0x89, g: 0xb4, b: 0xfa }),
        (NamedColor::BrightMagenta, Rgb { r: 0xf5, g: 0xc2, b: 0xe7 }),
        (NamedColor::BrightCyan, Rgb { r: 0x94, g: 0xe2, b: 0xd5 }),
        (NamedColor::BrightWhite, Rgb { r: 0xa6, g: 0xad, b: 0xc8 }),
    ];
    for (name, rgb) in &palette {
        colors[*name] = Some(*rgb);
    }

    // Foreground / background / cursor.
    colors[NamedColor::Foreground] = Some(Rgb { r: 0xcd, g: 0xd6, b: 0xf4 });
    colors[NamedColor::Background] = Some(Rgb { r: 0x1e, g: 0x1e, b: 0x2e });
    colors[NamedColor::Cursor] = Some(Rgb { r: 0xf5, g: 0xe0, b: 0xdc });

    // Dim variants.
    colors[NamedColor::DimBlack] = Some(Rgb { r: 0x14, g: 0x14, b: 0x21 });
    colors[NamedColor::DimRed] = Some(Rgb { r: 0xa8, g: 0x61, b: 0x75 });
    colors[NamedColor::DimGreen] = Some(Rgb { r: 0x74, g: 0x9e, b: 0x71 });
    colors[NamedColor::DimYellow] = Some(Rgb { r: 0xae, g: 0x9e, b: 0x7a });
    colors[NamedColor::DimBlue] = Some(Rgb { r: 0x60, g: 0x7e, b: 0xaf });
    colors[NamedColor::DimMagenta] = Some(Rgb { r: 0xab, g: 0x88, b: 0xa2 });
    colors[NamedColor::DimCyan] = Some(Rgb { r: 0x68, g: 0x9e, b: 0x95 });
    colors[NamedColor::DimWhite] = Some(Rgb { r: 0x82, g: 0x88, b: 0x9b });
    colors[NamedColor::DimForeground] = Some(Rgb { r: 0x90, g: 0x96, b: 0xab });
    colors[NamedColor::BrightForeground] = Some(Rgb { r: 0xcd, g: 0xd6, b: 0xf4 });

    // 216-color cube (indices 16..232).
    for i in 0..216 {
        let r = if i / 36 > 0 { (i / 36) * 40 + 55 } else { 0 };
        let g = if (i / 6) % 6 > 0 { ((i / 6) % 6) * 40 + 55 } else { 0 };
        let b = if i % 6 > 0 { (i % 6) * 40 + 55 } else { 0 };
        colors[16 + i] = Some(Rgb { r: r as u8, g: g as u8, b: b as u8 });
    }

    // Grayscale ramp (indices 232..256).
    for i in 0..24 {
        let value = (i * 10 + 8) as u8;
        colors[232 + i] = Some(Rgb { r: value, g: value, b: value });
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
    /// The portable-pty master handle, needed for resize signals.
    pty_master: Option<Box<dyn portable_pty::MasterPty + Send>>,
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
        self.parser.advance(&mut self.term, bytes);
        self.flush_writeback();
        self.rebuild_buffer();
        self.generation += 1;
    }

    /// Apply pending scroll without waiting for PTY output.
    pub fn apply_scroll(&mut self) {
        let scroll = self.pending_scroll.replace(0);
        if scroll != 0 {
            self.term.scroll_display(Scroll::Delta(scroll as i32));
            self.rebuild_buffer();
            self.generation += 1;
        }
        if let Some((cols, rows)) = self.pending_resize.take() {
            self.resize(cols, rows);
        }
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

    /// Encode a keyboard event and write it to the PTY.
    pub fn write_key(
        &mut self,
        key: keyboard::Key,
        mods: keyboard::Modifiers,
        text: Option<&str>,
    ) {
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
                pixel_width: (cols as f32 * CELL_WIDTH) as u16,
                pixel_height: (rows as f32 * CELL_HEIGHT) as u16,
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
        if !bytes.is_empty() && let Some(ref mut writer) = self.pty_writer {
            let _ = writer.write_all(&bytes);
            let _ = writer.flush();
        }
    }

    fn rebuild_buffer(&mut self) {
        let content = self.term.renderable_content();
        let term_colors = content.colors;
        let fallback = &self.default_colors;

        // Resolve default fg/bg.
        let default_fg = resolve_color(AnsiColor::Named(NamedColor::Foreground), term_colors, fallback);
        let default_bg = resolve_color(AnsiColor::Named(NamedColor::Background), term_colors, fallback);
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

            // Skip wide char spacers.
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER)
                || cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }

            let grapheme = cell.c;
            let bold = cell.flags.contains(Flags::BOLD);
            let italic = cell.flags.contains(Flags::ITALIC);
            let inverse = cell.flags.contains(Flags::INVERSE);

            let mut fg = resolve_color(cell.fg, term_colors, fallback);
            let mut bg = resolve_color(cell.bg, term_colors, fallback);

            if inverse {
                std::mem::swap(&mut fg, &mut bg);
            }

            let idx = row_idx * snap_cols + col_idx;
            if idx < self.buffer.cells.len() {
                self.buffer.cells[idx] = RenderCell {
                    grapheme,
                    fg,
                    bg,
                    bold,
                    italic,
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

// ── Canvas rendering ─────────────────────────────────────────────────────────

/// The Canvas Program that reads the pre-computed cell buffer and draws it.
pub struct TerminalCanvas<'a> {
    state: &'a TerminalState,
}

/// Internal canvas state — just the geometry cache.
pub struct CanvasState {
    cache: Cache,
    last_generation: Cell<u64>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            cache: Cache::default(),
            last_generation: Cell::new(0),
        }
    }
}

impl<'a> canvas::Program<()> for TerminalCanvas<'a> {
    type State = CanvasState;

    fn update(
        &self,
        _state: &mut Self::State,
        event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<()>> {
        match event {
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => -*y as isize,
                    mouse::ScrollDelta::Pixels { y, .. } => {
                        -(*y / CELL_HEIGHT) as isize
                    }
                };
                if lines != 0 {
                    self.state.request_scroll(lines);
                    return Some(canvas::Action::publish(()).and_capture());
                }
                None
            }
            _ => None,
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

        // Request resize if widget bounds changed.
        let desired_cols = (bounds.width / CELL_WIDTH).floor().max(1.0) as usize;
        let desired_rows = (bounds.height / CELL_HEIGHT).floor().max(1.0) as usize;
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
                    let x = col as f32 * CELL_WIDTH;
                    let y = row as f32 * CELL_HEIGHT;

                    // Draw cell background if it differs from the terminal default.
                    if cell.bg != buffer.default_bg {
                        frame.fill_rectangle(
                            IcedPoint::new(x, y),
                            Size::new(CELL_WIDTH, CELL_HEIGHT),
                            cell.bg,
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
                                ..iced::Font::MONOSPACE
                            }
                        } else if cell.italic {
                            iced::Font {
                                style: iced::font::Style::Italic,
                                ..iced::Font::MONOSPACE
                            }
                        } else {
                            iced::Font::MONOSPACE
                        };

                        frame.fill_text(canvas::Text {
                            content: cell.grapheme.to_string(),
                            position: IcedPoint::new(x, y),
                            color: cell.fg,
                            size: iced::Pixels(FONT_SIZE),
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
                let cx = cursor.x as f32 * CELL_WIDTH;
                let cy = cursor.y as f32 * CELL_HEIGHT;
                frame.fill_rectangle(
                    IcedPoint::new(cx, cy),
                    Size::new(CELL_WIDTH, CELL_HEIGHT),
                    Color {
                        a: 0.5,
                        ..theme::text_primary()
                    },
                );
            }
        });

        vec![geometry]
    }
}

/// Build a terminal view element from the current terminal state.
pub fn view_terminal(state: &TerminalState) -> Element<'_, ()> {
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
pub fn pty_subscription(key: String) -> Subscription<(String, PtyEvent)> {
    Subscription::run_with(key, |key| {
        use iced::futures::StreamExt;
        let key = key.clone();
        pty_worker().map(move |e| (key.clone(), e))
    })
}

fn pty_worker() -> impl iced::futures::Stream<Item = PtyEvent> {
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
            let cmd = portable_pty::CommandBuilder::new_default_prog();
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
