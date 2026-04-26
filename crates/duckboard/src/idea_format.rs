//! Format an idea body using duckpond's doc parse path. Ideas live outside
//! `duckspec/`, so we don't have a path to classify; `format_doc` is the
//! path-free entry point in duckpond that runs the same parser used by
//! codex/proposal/design/project documents.

use std::path::Path;

use duckpond::config::Config;
use duckpond::format::format_doc;

/// Run the duckpond doc formatter against an idea body. `duckspec_root` is
/// used to load `config.toml` for line-width and other format knobs; if it's
/// `None` or the file is missing, `FormatConfig::default()` applies.
///
/// On success returns the canonical body. On parse failure returns the
/// stringified errors so callers can surface them in the dashboard panel
/// without taking a dependency on `duckpond::error`.
pub fn format_body(body: &str, duckspec_root: Option<&Path>) -> Result<String, Vec<String>> {
    let config = duckspec_root
        .and_then(|root| Config::load(root).ok())
        .unwrap_or_default();
    format_doc(body, &config.format).map_err(|errors| errors.iter().map(|e| e.to_string()).collect())
}
