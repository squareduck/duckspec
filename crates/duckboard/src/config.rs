//! Application configuration stored at `~/.config/duckboard/config.toml`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: FontConfig,
    pub content: FontConfig,
}

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

pub fn config_dir() -> PathBuf {
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
