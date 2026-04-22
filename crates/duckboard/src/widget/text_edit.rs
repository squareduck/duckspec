//! Custom monospace text editor widget with integrated line number gutter.
//!
//! Split into submodules:
//! - `state`: Editor state, positions, blocks, text buffer, cursor, undo/redo, navigation
//! - `render`: Iced widget implementation, word-wrap, drawing

mod render;
mod state;

// Re-export public API so existing `use crate::widget::text_edit::*` continues to work.
pub use render::{TextEdit, view};
pub use state::{Block, BlockKind, EditorAction, EditorState, LineBgKind, Pos};
