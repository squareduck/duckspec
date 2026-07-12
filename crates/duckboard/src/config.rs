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
    /// Global main-chat default model (harness-tagged `ModelRef`). `None` until
    /// seeded after catalog refresh (or when no catalog model is choosable).
    /// Legacy configs omit the field and deserialize as `None`.
    pub default_model: Option<ModelRef>,
    /// Project override of the global default, keyed by `project_hash`. Absent
    /// means use the global default. Legacy bare-string values load as the
    /// `claude-code` harness via `ModelRef`'s deserialize shim.
    pub model_defaults: HashMap<String, ModelRef>,
    /// Chat affordances: optional oneshot reply chips after a turn.
    pub chat: ChatConfig,
}

/// Global chat UI flags (all projects / instances).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatConfig {
    /// After a turn, run a cheap oneshot for freeform reply chips when eligible.
    /// Default off (model cost).
    pub agent_input_hints: bool,
    /// Preferred oneshot model id per harness (`claude-code`, `grok`, …).
    /// Global (not per-project). Absent key → string-match default from catalog.
    pub oneshot_models: HashMap<String, String>,
}

impl ChatConfig {
    /// Configured oneshot model id for `harness`, if any.
    pub fn oneshot_model(&self, harness: &str) -> Option<&str> {
        self.oneshot_models.get(harness).map(String::as_str)
    }

    /// Set or clear the global oneshot model preference for `harness`.
    pub fn set_oneshot_model(&mut self, harness: &str, model: Option<String>) {
        match model {
            Some(m) => {
                self.oneshot_models.insert(harness.to_string(), m);
            }
            None => {
                self.oneshot_models.remove(harness);
            }
        }
    }
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
            default_model: None,
            model_defaults: HashMap::new(),
            chat: ChatConfig::default(),
        }
    }
}

impl Config {
    /// The global main-chat default model, if set (or seeded).
    pub fn global_model_default(&self) -> Option<&ModelRef> {
        self.default_model.as_ref()
    }

    /// Set (or, with `None`, clear) the global main-chat default model.
    pub fn set_global_model_default(&mut self, model: Option<ModelRef>) {
        self.default_model = model;
    }

    /// The project override for `project_root`, if one is set.
    pub fn project_model_default(&self, project_root: &Path) -> Option<ModelRef> {
        self.model_defaults
            .get(&project_hash(project_root))
            .cloned()
    }

    /// Set (or, with `None`, clear) the project override for `project_root`.
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

    #[test]
    fn missing_chat_table_deserializes_to_defaults() {
        let cfg: Config = toml::from_str("").expect("empty toml");
        assert!(!cfg.chat.agent_input_hints);
        assert!(cfg.chat.oneshot_models.is_empty());
    }

    #[test]
    fn unknown_auto_messages_key_is_ignored() {
        // GIVEN legacy config that still lists auto_messages
        let cfg: Config = toml::from_str(
            r#"
[chat]
agent_input_hints = true
auto_messages = true
"#,
        )
        .expect("legacy chat table");
        // THEN load succeeds and agent_input_hints is honored
        assert!(cfg.chat.agent_input_hints);
    }

    /// @spec chat/oneshot-models Global per-harness oneshot preference: A configured oneshot model for a harness is stored globally
    #[test]
    fn configured_oneshot_model_for_a_harness_is_stored_globally() {
        // GIVEN a preferred oneshot model id for a harness
        let mut cfg = Config::default();

        // WHEN the oneshot model setting is saved
        cfg.chat
            .set_oneshot_model("claude-code", Some("haiku".into()));

        // THEN that preference is stored as a global application setting for that harness
        assert_eq!(cfg.chat.oneshot_model("claude-code"), Some("haiku"));
        let toml = toml::to_string(&cfg).unwrap();
        let loaded: Config = toml::from_str(&toml).unwrap();
        assert_eq!(loaded.chat.oneshot_model("claude-code"), Some("haiku"));
    }

    /// @spec chat/oneshot-models Global per-harness oneshot preference: Preferences are keyed by harness not by project
    #[test]
    fn preferences_are_keyed_by_harness_not_by_project() {
        // GIVEN a preferred oneshot model for a harness
        // AND more than one project
        let mut cfg = Config::default();
        cfg.chat
            .set_oneshot_model("grok", Some("grok-composer-2.5-fast".into()));

        // WHEN the oneshot model setting is read in either project
        // THEN the same global preference for that harness is returned
        // (oneshot_models live on chat config, not model_defaults / project hash)
        assert!(cfg.model_defaults.is_empty());
        assert_eq!(
            cfg.chat.oneshot_model("grok"),
            Some("grok-composer-2.5-fast")
        );
        assert_eq!(cfg.chat.oneshot_model("claude-code"), None);
    }

    /// @spec harness/selection Global default model setting: A configured global default is stored as an application setting
    #[test]
    fn configured_global_default_is_stored_as_an_application_setting() {
        // GIVEN a harness-tagged model choice for the global main-chat default
        let mut cfg = Config::default();
        let choice = ModelRef::new("claude-code", "sonnet");

        // WHEN the global default setting is saved
        cfg.set_global_model_default(Some(choice.clone()));

        // THEN that choice is stored as a global application setting
        assert_eq!(cfg.global_model_default(), Some(&choice));
        let toml = toml::to_string(&cfg).unwrap();
        let loaded: Config = toml::from_str(&toml).unwrap();
        assert_eq!(loaded.global_model_default(), Some(&choice));
        assert!(loaded.model_defaults.is_empty());
    }
}
