//! File-system watcher with debouncing and .gitignore filtering.
//!
//! Watches the project root recursively, debounces events, filters through
//! the ignore crate (respects .gitignore), and emits classified file events
//! as an Iced subscription.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use ignore::gitignore::GitignoreBuilder;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};

// ── Event types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum FileEvent {
    /// File content was modified (or created — debouncer merges both).
    Modified(PathBuf),
    /// File or directory was removed.
    Removed(PathBuf),
}

// ── Subscription ────────────────────────────────────────────────────────────

/// Hashable wrapper so `Subscription::run_with` can deduplicate.
#[derive(Clone)]
struct WatchId(PathBuf);

impl Hash for WatchId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "file-watcher".hash(state);
        self.0.hash(state);
    }
}

/// Iced subscription that watches `project_root` for file changes.
///
/// Returns batches of [`FileEvent`]s after debouncing. The subscription keeps
/// running until the application exits.
pub fn watch_subscription(project_root: PathBuf) -> iced::Subscription<Vec<FileEvent>> {
    iced::Subscription::run_with(WatchId(project_root), |id| watch_stream(id.0.clone()))
}

fn watch_stream(
    project_root: PathBuf,
) -> impl iced::futures::Stream<Item = Vec<FileEvent>> {
    iced::stream::channel(32, |mut sender: iced::futures::channel::mpsc::Sender<Vec<FileEvent>>| async move {
        use iced::futures::SinkExt;

        // notify's debouncer sends to a std::sync::mpsc channel (blocking).
        // We bridge it to the async world via a tokio mpsc channel and a
        // dedicated background thread that does the blocking recv.
        let (notify_tx, notify_rx) = mpsc::channel();
        let (async_tx, mut async_rx) = tokio::sync::mpsc::channel::<Vec<FileEvent>>(32);

        let mut debouncer = match new_debouncer(Duration::from_millis(250), notify_tx) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("failed to create file watcher: {e}");
                std::future::pending::<()>().await;
                return;
            }
        };

        if let Err(e) = debouncer.watcher().watch(
            &project_root,
            notify_debouncer_mini::notify::RecursiveMode::Recursive,
        ) {
            tracing::error!("failed to watch {}: {e}", project_root.display());
            std::future::pending::<()>().await;
            return;
        }

        tracing::info!("file watcher active on {}", project_root.display());

        let gitignore = build_gitignore(&project_root);

        // Background thread: blocking recv from notify → async send.
        std::thread::spawn(move || {
            loop {
                match notify_rx.recv() {
                    Ok(Ok(events)) => {
                        let file_events: Vec<FileEvent> = events
                            .into_iter()
                            .filter(|ev| !is_ignored(&ev.path, &gitignore))
                            .filter_map(|ev| classify(&ev.path, ev.kind))
                            .collect();

                        if !file_events.is_empty() {
                            if async_tx.blocking_send(file_events).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Err(err)) => {
                        tracing::warn!("file watcher error: {err}");
                    }
                    Err(_) => {
                        tracing::debug!("file watcher notify channel closed");
                        break;
                    }
                }
            }
        });

        // Async loop: forward from tokio channel to iced stream.
        while let Some(file_events) = async_rx.recv().await {
            tracing::debug!(count = file_events.len(), "forwarding file events to UI");
            if sender.send(file_events).await.is_err() {
                tracing::debug!("file watcher iced receiver dropped");
                break;
            }
        }

        // Keep debouncer alive so the watcher doesn't get dropped.
        drop(debouncer);
    })
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn build_gitignore(project_root: &Path) -> ignore::gitignore::Gitignore {
    let mut builder = GitignoreBuilder::new(project_root);
    let gitignore_path = project_root.join(".gitignore");
    if gitignore_path.exists() {
        let _ = builder.add(&gitignore_path);
    }
    // Always ignore VCS and build internals even without a .gitignore.
    let _ = builder.add_line(None, ".git/");
    let _ = builder.add_line(None, ".jj/");
    let _ = builder.add_line(None, "target/");
    builder.build().unwrap_or_else(|e| {
        tracing::warn!("failed to build gitignore matcher: {e}");
        GitignoreBuilder::new(project_root).build().unwrap()
    })
}

fn is_ignored(path: &Path, gitignore: &ignore::gitignore::Gitignore) -> bool {
    let is_dir = path.is_dir();
    gitignore
        .matched_path_or_any_parents(path, is_dir)
        .is_ignore()
}

fn classify(path: &Path, kind: DebouncedEventKind) -> Option<FileEvent> {
    match kind {
        DebouncedEventKind::Any => {
            if path.exists() {
                Some(FileEvent::Modified(path.to_path_buf()))
            } else {
                Some(FileEvent::Removed(path.to_path_buf()))
            }
        }
        DebouncedEventKind::AnyContinuous | _ => None,
    }
}
