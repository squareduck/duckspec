//! Terminal emulator widget powered by libghostty-vt.
//!
//! Renders a PTY-backed terminal inside an Iced Canvas. The heavy lifting
//! (VT parsing, grid state) is handled by libghostty-vt; we only do
//! cell-by-cell rendering and keyboard/mouse plumbing.

use std::cell::{Cell, RefCell};
use std::io::{Read as _, Write};
use std::rc::Rc;

use iced::keyboard;
use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::Cache;
use iced::{Color, Element, Length, Point, Rectangle, Size, Subscription, Theme};

use libghostty_vt as vt;

use crate::theme;

// ── Constants ────────────────────────────────────────────────────────────────

const FONT_SIZE: f32 = theme::FONT_MD;
const CELL_WIDTH: f32 = 8.4;
const CELL_HEIGHT: f32 = 18.0;
const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;
const MAX_SCROLLBACK: usize = 10_000;

// ── Render buffer ────────────────────────────────────────────────────────────

/// A single rendered cell, pre-computed from the libghostty-vt grid.
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
            fg: theme::TEXT_PRIMARY,
            bg: theme::BG_BASE,
            bold: false,
            italic: false,
        }
    }
}

/// Pre-computed grid of cells ready for Canvas rendering.
struct CellBuffer {
    cols: u16,
    rows: u16,
    cells: Vec<RenderCell>,
    cursor: Option<CursorInfo>,
    default_fg: Color,
    default_bg: Color,
}

#[derive(Clone)]
struct CursorInfo {
    x: u16,
    y: u16,
    visible: bool,
}

impl CellBuffer {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            cells: vec![RenderCell::default(); (cols as usize) * (rows as usize)],
            cursor: None,
            default_fg: theme::TEXT_PRIMARY,
            default_bg: theme::BG_BASE,
        }
    }

    fn cell(&self, row: u16, col: u16) -> &RenderCell {
        &self.cells[(row as usize) * (self.cols as usize) + (col as usize)]
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        self.cells
            .resize((cols as usize) * (rows as usize), RenderCell::default());
    }
}

// ── Terminal state ───────────────────────────────────────────────────────────

/// Owns the libghostty-vt terminal and all associated state.
///
/// Lives on the main thread (`!Send + !Sync` due to libghostty-vt types).
/// The pre-computed `buffer` is rebuilt after every `feed()` call so that
/// Canvas `draw()` can read it without mutation.
pub struct TerminalState {
    /// Boxed to keep a stable address — libghostty-vt stores a raw pointer
    /// to the Terminal's internal VTable for callback dispatch. Moving the
    /// Terminal would invalidate that pointer and cause a use-after-free.
    vt: Box<vt::Terminal<'static, 'static>>,
    render: vt::RenderState<'static>,
    key_encoder: vt::key::Encoder<'static>,
    pty_writer: Option<Box<dyn Write + Send>>,
    /// Bytes the terminal wants to write back to the PTY (device responses).
    pty_writeback: Rc<RefCell<Vec<u8>>>,
    buffer: CellBuffer,
    pub generation: u64,
    pub cols: u16,
    pub rows: u16,
    /// Set by Canvas draw() when widget bounds change. Applied in next feed().
    pending_resize: Cell<Option<(u16, u16)>>,
    /// Scroll delta queued by Canvas update(). Applied in next feed().
    pending_scroll: Cell<isize>,
    /// The portable-pty master handle, needed for resize signals.
    pty_master: Option<Box<dyn portable_pty::MasterPty + Send>>,
}

impl TerminalState {
    pub fn new() -> anyhow::Result<Self> {
        let writeback: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));

        // Box the terminal FIRST, then register callbacks. This ensures the
        // VTable has a stable heap address before libghostty-vt stores a
        // raw pointer to it.
        let mut terminal = Box::new(vt::Terminal::new(vt::TerminalOptions {
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            max_scrollback: MAX_SCROLLBACK,
        })?);

        let wb = writeback.clone();
        terminal.on_pty_write(move |_term, data| {
            wb.borrow_mut().extend_from_slice(data);
        })?;

        let render = vt::RenderState::new()?;
        let key_encoder = vt::key::Encoder::new()?;

        Ok(Self {
            vt: terminal,
            render,
            key_encoder,
            pty_writer: None,
            pty_writeback: writeback,
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
            self.vt
                .scroll_viewport(vt::terminal::ScrollViewport::Delta(scroll));
        }
        self.vt.vt_write(bytes);
        self.flush_writeback();
        self.rebuild_buffer();
        self.generation += 1;
    }

    /// Apply pending scroll without waiting for PTY output.
    /// Called from update() when scroll events arrive but no PTY data is flowing.
    pub fn apply_scroll(&mut self) {
        let scroll = self.pending_scroll.replace(0);
        if scroll != 0 {
            self.vt
                .scroll_viewport(vt::terminal::ScrollViewport::Delta(scroll));
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
    pub fn request_resize(&self, cols: u16, rows: u16) {
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

        if let Some(encoded) = encode_key(&mut self.key_encoder, &self.vt, key, mods, text) {
            let _ = writer.write_all(&encoded);
            let _ = writer.flush();
        }
    }

    /// Resize the terminal grid and notify the PTY child.
    fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows || cols == 0 || rows == 0 {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let _ = self.vt.resize(cols, rows, CELL_WIDTH as u32, CELL_HEIGHT as u32);
        self.buffer.resize(cols, rows);

        // Notify PTY child of new size (sends SIGWINCH).
        if let Some(ref master) = self.pty_master {
            let _ = master.resize(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: (cols as f32 * CELL_WIDTH) as u16,
                pixel_height: (rows as f32 * CELL_HEIGHT) as u16,
            });
        }

        self.rebuild_buffer();
        self.generation += 1;
    }

    fn flush_writeback(&mut self) {
        let bytes: Vec<u8> = self.pty_writeback.borrow_mut().drain(..).collect();
        if !bytes.is_empty()
            && let Some(ref mut writer) = self.pty_writer {
                let _ = writer.write_all(&bytes);
                let _ = writer.flush();
            }
    }

    fn rebuild_buffer(&mut self) {
        let Ok(snapshot) = self.render.update(&self.vt) else {
            return;
        };

        // Read default colors from snapshot.
        if let Ok(colors) = snapshot.colors() {
            self.buffer.default_fg = rgb_to_color(colors.foreground);
            self.buffer.default_bg = rgb_to_color(colors.background);
        }

        // Sync buffer dimensions with what the snapshot reports.
        if let (Ok(snap_cols), Ok(snap_rows)) = (snapshot.cols(), snapshot.rows())
            && (snap_cols != self.buffer.cols || snap_rows != self.buffer.rows) {
                self.buffer.resize(snap_cols, snap_rows);
            }

        // Update cursor info.
        self.buffer.cursor = snapshot
            .cursor_viewport()
            .ok()
            .flatten()
            .map(|c| CursorInfo {
                x: c.x,
                y: c.y,
                visible: snapshot.cursor_visible().unwrap_or(true),
            });

        // Create fresh iterators each time — storing them across calls can
        // cause stale internal pointers after alternate screen switches.
        let Ok(mut row_iter) = vt::render::RowIterator::new() else {
            return;
        };
        let Ok(mut cell_iter) = vt::render::CellIterator::new() else {
            return;
        };
        let Ok(mut rows) = row_iter.update(&snapshot) else {
            return;
        };

        let default_fg = self.buffer.default_fg;
        let default_bg = self.buffer.default_bg;
        let buf_cols = self.buffer.cols as usize;
        let buf_rows = self.buffer.rows as usize;
        let buf_len = self.buffer.cells.len();
        let mut row_idx: usize = 0;

        while let Some(row) = rows.next() {
            if row_idx >= buf_rows {
                break;
            }

            let Ok(mut cells) = cell_iter.update(row) else {
                row_idx += 1;
                continue;
            };

            let mut col_idx: usize = 0;
            while let Some(cell) = cells.next() {
                if col_idx >= buf_cols {
                    break;
                }

                let idx = row_idx * buf_cols + col_idx;
                if idx >= buf_len {
                    break;
                }

                // Grapheme: take the first codepoint.
                let grapheme = cell
                    .graphemes()
                    .ok()
                    .and_then(|g| g.into_iter().next())
                    .unwrap_or(' ');

                // Colors: use cell-level resolved colors, fall back to defaults.
                let fg = cell
                    .fg_color()
                    .ok()
                    .flatten()
                    .map(rgb_to_color)
                    .unwrap_or(default_fg);
                let bg = cell
                    .bg_color()
                    .ok()
                    .flatten()
                    .map(rgb_to_color)
                    .unwrap_or(default_bg);

                // Style attributes.
                let style = cell.style().ok();
                let bold = style.as_ref().is_some_and(|s| s.bold);
                let italic = style.as_ref().is_some_and(|s| s.italic);

                // Handle inverse video.
                let (fg, bg) = if style.as_ref().is_some_and(|s| s.inverse) {
                    (bg, fg)
                } else {
                    (fg, bg)
                };

                self.buffer.cells[idx] = RenderCell {
                    grapheme,
                    fg,
                    bg,
                    bold,
                    italic,
                };

                col_idx += 1;
            }

            row_idx += 1;
        }
    }
}

fn rgb_to_color(rgb: vt::style::RgbColor) -> Color {
    Color::from_rgb8(rgb.r, rgb.g, rgb.b)
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
                    return Some(canvas::Action::publish(())
                        .and_capture());
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
        let desired_cols = (bounds.width / CELL_WIDTH).floor().max(1.0) as u16;
        let desired_rows = (bounds.height / CELL_HEIGHT).floor().max(1.0) as u16;
        self.state.request_resize(desired_cols, desired_rows);

        // Invalidate cache when terminal content changes.
        if self.state.generation != state.last_generation.get() {
            state.cache.clear();
            state.last_generation.set(self.state.generation);
        }

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            // Fill background.
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), buffer.default_bg);

            let cols = buffer.cols as usize;
            let rows = buffer.rows as usize;

            for row in 0..rows {
                for col in 0..cols {
                    let cell = buffer.cell(row as u16, col as u16);
                    let x = col as f32 * CELL_WIDTH;
                    let y = row as f32 * CELL_HEIGHT;

                    // Draw cell background if it differs from the terminal default.
                    if cell.bg != buffer.default_bg {
                        frame.fill_rectangle(
                            Point::new(x, y),
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
                            position: Point::new(x, y),
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
                && cursor.visible {
                    let cx = cursor.x as f32 * CELL_WIDTH;
                    let cy = cursor.y as f32 * CELL_HEIGHT;
                    frame.fill_rectangle(
                        Point::new(cx, cy),
                        Size::new(CELL_WIDTH, CELL_HEIGHT),
                        Color {
                            a: 0.5,
                            ..theme::TEXT_PRIMARY
                        },
                    );
                }
        });

        vec![geometry]
    }
}

/// Build a terminal view element from the current terminal state.
///
/// The returned element produces no messages — keyboard input is handled
/// via a top-level subscription.
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
    inner: std::sync::Arc<std::sync::Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
}

impl PtyMaster {
    pub fn into_master(self) -> Box<dyn portable_pty::MasterPty + Send> {
        match std::sync::Arc::try_unwrap(self.inner) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(_arc) => {
                // Can't unwrap — this shouldn't happen in practice since
                // we only ever send this once.
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
    inner: std::sync::Arc<std::sync::Mutex<Box<dyn Write + Send>>>,
}

impl PtyWriter {
    pub fn into_writer(self) -> Box<dyn Write + Send> {
        match std::sync::Arc::try_unwrap(self.inner) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => Box::new(ArcWriter(arc)),
        }
    }
}

struct ArcWriter(std::sync::Arc<std::sync::Mutex<Box<dyn Write + Send>>>);
impl Write for ArcWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

/// Create a subscription that spawns a PTY child and streams its output.
/// The `key` parameter makes each subscription unique so multiple terminals can coexist.
/// Returns `(key, event)` tuples so the caller can route events without capturing in `.map()`.
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
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
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
                inner: std::sync::Arc::new(std::sync::Mutex::new(writer)),
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
                inner: std::sync::Arc::new(std::sync::Mutex::new(pair.master)),
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
    encoder: &mut vt::key::Encoder<'_>,
    terminal: &vt::Terminal,
    key: keyboard::Key,
    mods: keyboard::Modifiers,
    text: Option<&str>,
) -> Option<Vec<u8>> {
    // Sync encoder options with current terminal state.
    encoder.set_options_from_terminal(terminal);

    let mut event = vt::key::Event::new().ok()?;
    event.set_action(vt::key::Action::Press);

    // Map Iced modifiers to libghostty-vt mods.
    let mut ghostty_mods = vt::key::Mods::empty();
    if mods.shift() {
        ghostty_mods |= vt::key::Mods::SHIFT;
    }
    if mods.control() {
        ghostty_mods |= vt::key::Mods::CTRL;
    }
    if mods.alt() {
        ghostty_mods |= vt::key::Mods::ALT;
    }
    if mods.logo() {
        ghostty_mods |= vt::key::Mods::SUPER;
    }
    event.set_mods(ghostty_mods);

    // Map the key.
    match &key {
        keyboard::Key::Named(named) => {
            let ghostty_key = map_named_key(*named)?;
            event.set_key(ghostty_key);
        }
        keyboard::Key::Character(ch) => {
            // Map character to physical key if possible, otherwise use Unidentified.
            let ghostty_key = map_char_to_key(ch).unwrap_or(vt::key::Key::Unidentified);
            event.set_key(ghostty_key);
            if let Some(c) = ch.chars().next() {
                event.set_unshifted_codepoint(c);
            }
        }
        _ => return None,
    }

    // Set utf8 text if available.
    if let Some(t) = text {
        event.set_utf8(Some(t));
    } else if let keyboard::Key::Character(ch) = &key {
        event.set_utf8(Some(ch.as_str()));
    }

    let mut buf = Vec::new();
    encoder.encode_to_vec(&event, &mut buf).ok()?;
    if buf.is_empty() { None } else { Some(buf) }
}

/// Map Iced named keys to libghostty-vt key codes.
fn map_named_key(named: keyboard::key::Named) -> Option<vt::key::Key> {
    use keyboard::key::Named::*;
    Some(match named {
        Enter => vt::key::Key::Enter,
        Tab => vt::key::Key::Tab,
        Backspace => vt::key::Key::Backspace,
        Escape => vt::key::Key::Escape,
        Space => vt::key::Key::Space,
        ArrowUp => vt::key::Key::ArrowUp,
        ArrowDown => vt::key::Key::ArrowDown,
        ArrowLeft => vt::key::Key::ArrowLeft,
        ArrowRight => vt::key::Key::ArrowRight,
        Home => vt::key::Key::Home,
        End => vt::key::Key::End,
        PageUp => vt::key::Key::PageUp,
        PageDown => vt::key::Key::PageDown,
        Insert => vt::key::Key::Insert,
        Delete => vt::key::Key::Delete,
        F1 => vt::key::Key::F1,
        F2 => vt::key::Key::F2,
        F3 => vt::key::Key::F3,
        F4 => vt::key::Key::F4,
        F5 => vt::key::Key::F5,
        F6 => vt::key::Key::F6,
        F7 => vt::key::Key::F7,
        F8 => vt::key::Key::F8,
        F9 => vt::key::Key::F9,
        F10 => vt::key::Key::F10,
        F11 => vt::key::Key::F11,
        F12 => vt::key::Key::F12,
        _ => return None,
    })
}

/// Map a character string to the corresponding physical key.
fn map_char_to_key(ch: &str) -> Option<vt::key::Key> {
    let c = ch.chars().next()?;
    Some(match c.to_ascii_lowercase() {
        'a' => vt::key::Key::A,
        'b' => vt::key::Key::B,
        'c' => vt::key::Key::C,
        'd' => vt::key::Key::D,
        'e' => vt::key::Key::E,
        'f' => vt::key::Key::F,
        'g' => vt::key::Key::G,
        'h' => vt::key::Key::H,
        'i' => vt::key::Key::I,
        'j' => vt::key::Key::J,
        'k' => vt::key::Key::K,
        'l' => vt::key::Key::L,
        'm' => vt::key::Key::M,
        'n' => vt::key::Key::N,
        'o' => vt::key::Key::O,
        'p' => vt::key::Key::P,
        'q' => vt::key::Key::Q,
        'r' => vt::key::Key::R,
        's' => vt::key::Key::S,
        't' => vt::key::Key::T,
        'u' => vt::key::Key::U,
        'v' => vt::key::Key::V,
        'w' => vt::key::Key::W,
        'x' => vt::key::Key::X,
        'y' => vt::key::Key::Y,
        'z' => vt::key::Key::Z,
        '0' => vt::key::Key::Digit0,
        '1' => vt::key::Key::Digit1,
        '2' => vt::key::Key::Digit2,
        '3' => vt::key::Key::Digit3,
        '4' => vt::key::Key::Digit4,
        '5' => vt::key::Key::Digit5,
        '6' => vt::key::Key::Digit6,
        '7' => vt::key::Key::Digit7,
        '8' => vt::key::Key::Digit8,
        '9' => vt::key::Key::Digit9,
        '-' => vt::key::Key::Minus,
        '=' => vt::key::Key::Equal,
        '[' => vt::key::Key::BracketLeft,
        ']' => vt::key::Key::BracketRight,
        '\\' => vt::key::Key::Backslash,
        ';' => vt::key::Key::Semicolon,
        '\'' => vt::key::Key::Quote,
        '`' => vt::key::Key::Backquote,
        ',' => vt::key::Key::Comma,
        '.' => vt::key::Key::Period,
        '/' => vt::key::Key::Slash,
        _ => return None,
    })
}
