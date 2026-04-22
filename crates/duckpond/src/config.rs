use std::path::{Path, PathBuf};

/// Configuration from `duckspec/config.toml`.
#[derive(Debug, Default)]
pub struct Config {
    /// Paths to scan for `@spec` backlinks, relative to the project root.
    /// Empty means "scan from project root".
    pub test_paths: Vec<PathBuf>,
    /// Formatting options applied when rendering artifact schemas.
    pub format: FormatConfig,
}

/// Formatting knobs applied when rendering artifacts.
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Target line width for wrapping prose in artifacts.
    pub line_width: usize,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self { line_width: 90 }
    }
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
    #[error("config.toml: [format] must be a table")]
    BadFormat,
    #[error("config.toml: format.line_width must be a positive integer")]
    BadLineWidth,
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

        let format = match table.get("format") {
            Some(toml::Value::Table(t)) => {
                let line_width = match t.get("line_width") {
                    Some(toml::Value::Integer(n)) if *n > 0 => *n as usize,
                    Some(_) => return Err(ConfigError::BadLineWidth),
                    None => FormatConfig::default().line_width,
                };
                FormatConfig { line_width }
            }
            Some(_) => return Err(ConfigError::BadFormat),
            None => FormatConfig::default(),
        };

        Ok(Self { test_paths, format })
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

    #[test]
    fn default_line_width_is_90() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.format.line_width, 90);
    }

    #[test]
    fn parses_format_line_width() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            "[format]\nline_width = 100\n",
        )
        .unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.format.line_width, 100);
    }

    #[test]
    fn format_table_without_line_width_uses_default() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "[format]\n").unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.format.line_width, 90);
    }

    #[test]
    fn zero_line_width_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "[format]\nline_width = 0\n").unwrap();
        let err = Config::load(dir.path()).unwrap_err();
        assert!(matches!(err, ConfigError::BadLineWidth));
    }

    #[test]
    fn non_integer_line_width_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            "[format]\nline_width = \"wide\"\n",
        )
        .unwrap();
        let err = Config::load(dir.path()).unwrap_err();
        assert!(matches!(err, ConfigError::BadLineWidth));
    }

    #[test]
    fn format_not_a_table_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "format = 80\n").unwrap();
        let err = Config::load(dir.path()).unwrap_err();
        assert!(matches!(err, ConfigError::BadFormat));
    }
}
