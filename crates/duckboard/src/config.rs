//! Application configuration stored at `~/.config/duckboard/config.toml`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use duckchat::ModelRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: FontConfig,
    pub content: FontConfig,
    pub projects: ProjectsConfig,
    /// Default model (harness-tagged `ModelRef`) per project, keyed by
    /// `project_hash`. New chat sessions in a project inherit this default
    /// when they haven't pinned a model of their own. Absent = fall through to
    /// the built-in default. Legacy bare-string values load as the
    /// `claude-code` harness via `ModelRef`'s deserialize shim.
    pub model_defaults: HashMap<String, ModelRef>,
    /// Chat affordances: under-input agent hints and auto-message chips.
    pub chat: ChatConfig,
}

/// Global chat UI flags (all projects / instances).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatConfig {
    /// Under-input agent (oneshot) suggestions after a turn. Default off.
    pub agent_input_hints: bool,
    /// Obvious lifecycle / affirm / decline chip chrome + ⌘ bindings. Default on.
    pub auto_messages: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectsConfig {
    /// Most-recently-opened first. Capped at RECENT_CAP.
    pub recent: Vec<PathBuf>,
}

/// Maximum number of entries kept in `projects.recent`.
const RECENT_CAP: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    pub font_family: String,
    pub font_size: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: FontConfig {
                font_family: String::new(),
                font_size: 13.0,
            },
            content: FontConfig {
                font_family: String::from("monospace"),
                font_size: 13.0,
            },
            projects: ProjectsConfig::default(),
            model_defaults: HashMap::new(),
            chat: ChatConfig::default(),
        }
    }
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            agent_input_hints: false,
            auto_messages: true,
        }
    }
}

impl Config {
    /// The default model for `project_root`, if one is set.
    pub fn project_model_default(&self, project_root: &Path) -> Option<ModelRef> {
        self.model_defaults
            .get(&project_hash(project_root))
            .cloned()
    }

    /// Set (or, with `None`, clear) the default model for `project_root`.
    pub fn set_project_model_default(&mut self, project_root: &Path, model: Option<ModelRef>) {
        let key = project_hash(project_root);
        match model {
            Some(m) => {
                self.model_defaults.insert(key, m);
            }
            None => {
                self.model_defaults.remove(&key);
            }
        }
    }
}

impl ProjectsConfig {
    /// Promote `path` to the head of the recent list, deduping by canonical
    /// form when available and capping the list length. No-op if `path` is
    /// empty.
    pub fn touch(&mut self, path: &Path) {
        if path.as_os_str().is_empty() {
            return;
        }
        let canonical = path.canonicalize().ok();
        let target = canonical.as_deref().unwrap_or(path);
        self.recent.retain(|p| {
            let pc = p.canonicalize().ok();
            pc.as_deref().unwrap_or(p.as_path()) != target
        });
        self.recent.insert(0, target.to_path_buf());
        if self.recent.len() > RECENT_CAP {
            self.recent.truncate(RECENT_CAP);
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_size: 13.0,
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Per-thread redirect for `config_dir`, set by tests so that everything
    /// derived from it (`data_dir`, `ideas_root`, …) lands in a temp directory
    /// instead of the real `~/.config/duckboard`. Thread-local keeps parallel
    /// tests isolated without a serialization dependency.
    static CONFIG_DIR_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// Test-only: redirect `config_dir()` to `dir` for the current thread.
#[cfg(test)]
pub fn set_config_dir_override(dir: PathBuf) {
    CONFIG_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dir));
}

pub fn config_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(dir) = CONFIG_DIR_OVERRIDE.with(|c| c.borrow().clone()) {
        return dir;
    }
    dirs::home_dir()
        .expect("home directory must exist")
        .join(".config")
        .join("duckboard")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn data_dir(project_root: Option<&Path>) -> PathBuf {
    let base = config_dir().join("data");
    match project_root {
        Some(root) => base.join("projects").join(project_hash(root)),
        None => base,
    }
}

fn project_hash(project_root: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project_root.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn load() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(data) => match toml::from_str(&data) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!(path = %path.display(), "failed to parse config, using defaults: {e}");
                Config::default()
            }
        },
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            tracing::warn!(path = %path.display(), "failed to read config, using defaults: {e}");
            Config::default()
        }
        Err(_) => Config::default(),
    }
}

pub fn save(config: &Config) -> anyhow::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let data = toml::to_string_pretty(config)?;
    std::fs::write(config_path(), data)?;
    Ok(())
}

pub fn ui_font(config: &Config) -> iced::Font {
    if config.ui.font_family.is_empty() {
        iced::Font::DEFAULT
    } else {
        iced::Font::with_name(string_to_static(&config.ui.font_family))
    }
}

pub fn content_font(config: &Config) -> iced::Font {
    if config.content.font_family == "monospace" || config.content.font_family.is_empty() {
        iced::Font::MONOSPACE
    } else {
        iced::Font::with_name(string_to_static(&config.content.font_family))
    }
}

fn string_to_static(s: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::OnceLock;
    static INTERNED: OnceLock<std::sync::Mutex<HashSet<&'static str>>> = OnceLock::new();
    let set = INTERNED.get_or_init(|| std::sync::Mutex::new(HashSet::new()));
    let mut guard = set.lock().unwrap();
    if let Some(&existing) = guard.get(s) {
        existing
    } else {
        let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
        guard.insert(leaked);
        leaked
    }
}

pub fn list_system_fonts() -> Vec<String> {
    let source = font_kit::source::SystemSource::new();
    let mut families: Vec<String> = source.all_families().unwrap_or_default();
    families.sort_unstable();
    families.dedup();
    families
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec chat/default-prompts Agent input hints gate: Default agent input hints setting is disabled
    #[test]
    fn default_agent_input_hints_setting_is_disabled() {
        // GIVEN application config defaults
        // WHEN the agent input hints setting is read
        // THEN it is disabled
        assert!(!Config::default().chat.agent_input_hints);
        assert!(!ChatConfig::default().agent_input_hints);
    }

    // @spec chat/obvious-bubble Chrome visibility: Default auto messages setting is enabled
    #[test]
    fn default_auto_messages_setting_is_enabled() {
        // GIVEN application config defaults
        // WHEN the auto messages setting is read
        // THEN it is enabled
        assert!(Config::default().chat.auto_messages);
        assert!(ChatConfig::default().auto_messages);
    }

    #[test]
    fn missing_chat_table_deserializes_to_defaults() {
        let cfg: Config = toml::from_str("").expect("empty toml");
        assert!(!cfg.chat.agent_input_hints);
        assert!(cfg.chat.auto_messages);
    }
}
