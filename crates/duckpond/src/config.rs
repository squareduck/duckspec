use std::path::{Path, PathBuf};

/// Configuration from `duckspec/config.toml`.
#[derive(Debug, Default)]
pub struct Config {
    /// Paths to scan for `@spec` backlinks, relative to the project root.
    /// Empty means "scan from project root".
    pub test_paths: Vec<PathBuf>,
}

/// Error loading config.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config.toml: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config.toml: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("config.toml: test_paths must be an array of strings")]
    BadTestPaths,
}

impl Config {
    /// Load configuration from `duckspec/config.toml`, or return defaults
    /// if the file does not exist.
    pub fn load(duckspec_root: &Path) -> Result<Self, ConfigError> {
        let config_path = duckspec_root.join("config.toml");
        if !config_path.is_file() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let table: toml::Table = content.parse()?;

        let test_paths = match table.get("test_paths") {
            Some(toml::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(PathBuf::from))
                .collect(),
            Some(_) => return Err(ConfigError::BadTestPaths),
            None => Vec::new(),
        };

        Ok(Self { test_paths })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert!(config.test_paths.is_empty());
    }

    #[test]
    fn parses_test_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            r#"test_paths = ["tests/", "src/tests/"]"#,
        )
        .unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(
            config.test_paths,
            vec![PathBuf::from("tests/"), PathBuf::from("src/tests/")]
        );
    }

    #[test]
    fn empty_config_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "").unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert!(config.test_paths.is_empty());
    }
}
