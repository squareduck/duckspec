//! Claude Code catalog advertised on ACP `initialize`.
//!
//! Fetches the public catalog with no auth, admits main-section rows the
//! spawned `claude` binary is new enough to run, and reports fetch status.
//! A failed fetch still produces a successful initialize with no models.

use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};

const CATALOG_URL: &str = "https://downloads.claude.ai/model-catalog/v1/catalog.json";
const CATALOG_TIMEOUT: Duration = Duration::from_secs(15);
const VERSION_TIMEOUT: Duration = Duration::from_secs(20);

/// Effort scale carried on an admitted catalog row.
struct AdvertisedEffort {
    default_level: String,
    levels: Vec<String>,
}

/// A model the agent advertises to the ACP host.
struct AdvertisedModel {
    id: String,
    name: String,
    context_window: Option<usize>,
    effort: Option<AdvertisedEffort>,
}

/// Fetch outcome reported under `modelState.catalog`.
enum CatalogReport {
    Ok { expires_at: Option<String> },
    Failed,
}

/// Advertise set plus the catalog status initialize should report.
struct Advertise {
    models: Vec<AdvertisedModel>,
    catalog: CatalogReport,
}

/// A row minimum: absent, a parsed `N.N.N`, or present but not that shape.
enum Minimum {
    None,
    Triple((u64, u64, u64)),
    Unreadable,
}

/// Fetch the public catalog, read this agent's `claude --version`, and build
/// the initialize result. Fetch failure still returns a result.
pub(crate) async fn advertise_initialize() -> Value {
    let document = match fetch_catalog_document().await {
        Ok(document) => document,
        Err(err) => {
            tracing::warn!("claude catalog fetch failed, advertising no models: {err}");
            return fetch_failure_result();
        }
    };
    let version = claude_version_text().await;
    if version.is_none() {
        tracing::warn!("claude --version unreadable; dropping catalog rows that require a minimum");
    }
    initialize_result(&admit_catalog(&document, version.as_deref()))
}

fn fetch_failure_result() -> Value {
    initialize_result(&Advertise {
        models: Vec::new(),
        catalog: CatalogReport::Failed,
    })
}

async fn fetch_catalog_document() -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(CATALOG_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(CATALOG_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("catalog HTTP {status}"));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// `--version` text from the same `claude` argv this agent spawns.
/// `None` when the command fails, times out, or prints nothing.
async fn claude_version_text() -> Option<String> {
    let prefix = crate::claude::claude_argv_prefix();
    let mut args = prefix.into_iter();
    let program = args.next()?;
    let mut cmd = tokio::process::Command::new(program);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = match tokio::time::timeout(VERSION_TIMEOUT, cmd.output()).await {
        Ok(Ok(output)) => output,
        _ => return None,
    };
    if !output.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    let text = text.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// Admit main-section rows against `version_output` (`None` when `--version`
/// could not be read) and keep the document's `expires_at` when it has one.
fn admit_catalog(document: &Value, version_output: Option<&str>) -> Advertise {
    let binary = version_output.and_then(parse_leading_version);
    let expires_at = document
        .get("expires_at")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Advertise {
        models: admitted_models(document, binary),
        catalog: CatalogReport::Ok { expires_at },
    }
}

fn admitted_models(document: &Value, binary: Option<(u64, u64, u64)>) -> Vec<AdvertisedModel> {
    let Some(configs) = document
        .pointer("/surfaces/cc/model_selector_config")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let mut models = Vec::new();
    for cfg in configs {
        let Some(rows) = cfg.get("models").and_then(Value::as_array) else {
            continue;
        };
        for row in rows {
            if let Some(model) = admit_row(row, binary) {
                models.push(model);
            }
        }
    }
    models
}

fn admit_row(row: &Value, binary: Option<(u64, u64, u64)>) -> Option<AdvertisedModel> {
    if row.get("section").and_then(Value::as_str) != Some("main") {
        return None;
    }
    if !version_allows(binary, row_minimum(row)) {
        return None;
    }
    let id = row
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let name = row
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(id)
        .to_string();
    Some(AdvertisedModel {
        id: id.to_string(),
        name,
        context_window: row_window(row),
        effort: row_effort(row),
    })
}

fn version_allows(binary: Option<(u64, u64, u64)>, minimum: Minimum) -> bool {
    match minimum {
        Minimum::None => true,
        Minimum::Unreadable => false,
        Minimum::Triple(min) => binary.is_some_and(|bin| bin >= min),
    }
}

fn row_minimum(row: &Value) -> Minimum {
    match row.get("min_claude_code_version") {
        None | Some(Value::Null) => Minimum::None,
        Some(value) => {
            let Some(raw) = value.as_str() else {
                return Minimum::Unreadable;
            };
            let raw = raw.trim();
            if raw.is_empty() {
                return Minimum::None;
            }
            match parse_exact_triple(raw) {
                Some(triple) => Minimum::Triple(triple),
                None => Minimum::Unreadable,
            }
        }
    }
}

fn row_window(row: &Value) -> Option<usize> {
    let n = row
        .pointer("/runtime/max_input_tokens")
        .and_then(Value::as_u64)?;
    if n == 0 { None } else { Some(n as usize) }
}

/// Effort only when the row's thinking type is `effort`, its level list is
/// non-empty, and it names a default. The client drops an incomplete object.
fn row_effort(row: &Value) -> Option<AdvertisedEffort> {
    if row.pointer("/thinking/type").and_then(Value::as_str) != Some("effort") {
        return None;
    }
    let levels: Vec<String> = row
        .pointer("/runtime/effort_levels")
        .and_then(Value::as_array)?
        .iter()
        .map(|level| {
            level
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .collect::<Option<_>>()?;
    if levels.is_empty() {
        return None;
    }
    let default_level = row
        .pointer("/runtime/default_effort")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    Some(AdvertisedEffort {
        default_level,
        levels,
    })
}

/// First `N.N.N` in `text`. Later components after that triple are ignored.
fn parse_leading_version(text: &str) -> Option<(u64, u64, u64)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let rest = &text[i..];
            let token_len = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .map(char::len_utf8)
                .sum::<usize>();
            if let Some(version) = parse_triple_prefix(&rest[..token_len]) {
                return Some(version);
            }
            i += token_len.max(1);
            continue;
        }
        i += 1;
    }
    None
}

fn parse_triple_prefix(token: &str) -> Option<(u64, u64, u64)> {
    let mut parts = token.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

fn parse_exact_triple(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn initialize_result(advertise: &Advertise) -> Value {
    let available: Vec<Value> = advertise.models.iter().map(model_entry).collect();
    let catalog = match &advertise.catalog {
        CatalogReport::Ok {
            expires_at: Some(expires_at),
        } => json!({ "status": "ok", "expiresAt": expires_at }),
        CatalogReport::Ok { expires_at: None } => json!({ "status": "ok" }),
        CatalogReport::Failed => json!({ "status": "failed" }),
    };
    json!({
        "protocolVersion": 1,
        "agentCapabilities": { "loadSession": true },
        "_meta": {
            "modelState": {
                "availableModels": available,
                "catalog": catalog,
            }
        }
    })
}

fn model_entry(model: &AdvertisedModel) -> Value {
    let mut entry = json!({
        "modelId": model.id,
        "name": model.name,
    });
    let mut meta = serde_json::Map::new();
    if let Some(window) = model.context_window {
        meta.insert("totalContextTokens".into(), json!(window));
    }
    if let Some(effort) = &model.effort {
        meta.insert(
            "effort".into(),
            json!({
                "default": effort.default_level,
                "levels": effort.levels,
            }),
        );
    }
    if !meta.is_empty() {
        entry["_meta"] = Value::Object(meta);
    }
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    fn available_models(init: &Value) -> &Vec<Value> {
        init.pointer("/_meta/modelState/availableModels")
            .and_then(Value::as_array)
            .expect("availableModels array")
    }

    fn model_ids(init: &Value) -> Vec<&str> {
        available_models(init)
            .iter()
            .filter_map(|model| model.get("modelId").and_then(Value::as_str))
            .collect()
    }

    /// @spec harness/claude Agent model advertise: Admitted catalog rows carry name, window, and effort
    #[test]
    fn admitted_catalog_rows_carry_name_window_and_effort() {
        // GIVEN a catalog and a claude binary whose leading triple satisfies
        // one main row and is older than another main row's minimum.
        let document = json!({
            "expires_at": "2026-10-16T08:03:29Z",
            "surfaces": {
                "cc": {
                    "model_selector_config": [{
                        "models": [
                            {
                                "id": "claude-sonnet-5-5",
                                "name": "Sonnet 5.5",
                                "section": "main",
                                "min_claude_code_version": "2.1.0",
                                "thinking": { "type": "effort" },
                                "runtime": {
                                    "max_input_tokens": 1_000_000,
                                    "effort_levels": ["low", "medium", "high", "xhigh", "max"],
                                    "default_effort": "medium"
                                }
                            },
                            {
                                "id": "claude-haiku-5-5",
                                "name": "Haiku 5.5",
                                "section": "main",
                                "min_claude_code_version": "1.0.0",
                                "thinking": { "type": "none" },
                                "runtime": { "max_input_tokens": 200_000 }
                            },
                            {
                                "id": "claude-opus-9",
                                "name": "Opus 9",
                                "section": "main",
                                "min_claude_code_version": "9.0.0",
                                "runtime": { "max_input_tokens": 1_000_000 }
                            },
                            {
                                "id": "claude-overflow",
                                "name": "Overflow",
                                "section": "overflow",
                                "runtime": { "max_input_tokens": 1_000_000 }
                            }
                        ]
                    }]
                }
            }
        });

        // WHEN the agent completes initialize
        let init = initialize_result(&admit_catalog(&document, Some("2.1.100 (Claude Code)")));

        // THEN the satisfied row carries its name, window, and effort scale
        // AND the row with no effort scale is advertised without effort
        // AND the too-new row and the non-main row are not advertised
        let models = available_models(&init);
        assert_eq!(model_ids(&init), ["claude-sonnet-5-5", "claude-haiku-5-5"]);
        assert_eq!(models[0]["name"], "Sonnet 5.5");
        assert_eq!(models[0]["_meta"]["totalContextTokens"], 1_000_000);
        assert_eq!(
            models[0]["_meta"]["effort"]["levels"],
            json!(["low", "medium", "high", "xhigh", "max"])
        );
        assert_eq!(models[0]["_meta"]["effort"]["default"], "medium");
        assert_eq!(models[1]["name"], "Haiku 5.5");
        assert!(models[1].pointer("/_meta/effort").is_none());
    }

    /// @spec harness/claude Agent model advertise: An unreadable binary version drops rows that require a minimum
    #[test]
    fn unreadable_binary_version_drops_rows_that_require_a_minimum() {
        let document = json!({
            "surfaces": {
                "cc": {
                    "model_selector_config": [{
                        "models": [
                            {
                                "id": "needs-min",
                                "name": "Needs Min",
                                "section": "main",
                                "min_claude_code_version": "2.0.0"
                            },
                            {
                                "id": "open",
                                "name": "Open",
                                "section": "main"
                            }
                        ]
                    }]
                }
            }
        });

        // Command failure and text that is not a leading N.N.N are the same outcome.
        for version in [None, Some("not a version"), Some("2.1")] {
            // WHEN the agent completes initialize
            let init = initialize_result(&admit_catalog(&document, version));

            // THEN only the row without a minimum is advertised
            assert_eq!(model_ids(&init), ["open"], "version {version:?}");
        }
    }

    /// @spec harness/claude Agent model advertise: A failed catalog fetch advertises no models
    #[test]
    fn failed_catalog_fetch_advertises_no_models() {
        // GIVEN a catalog fetch that fails
        // WHEN the agent completes initialize
        let init = fetch_failure_result();

        // THEN initialize succeeds, the catalog is failed, and no models are advertised
        assert!(init.get("error").is_none());
        assert_eq!(init["protocolVersion"], 1);
        assert!(available_models(&init).is_empty());
        assert_eq!(
            init.pointer("/_meta/modelState/catalog/status"),
            Some(&json!("failed"))
        );
    }

    /// @spec harness/claude Agent model advertise: Catalog expiry is reported only when the document has one
    #[test]
    fn catalog_expiry_is_reported_only_when_the_document_has_one() {
        let row = json!({
            "id": "claude-sonnet-5-5",
            "name": "Sonnet 5.5",
            "section": "main"
        });
        let with_expiry = json!({
            "expires_at": "2026-10-16T08:03:29Z",
            "surfaces": { "cc": { "model_selector_config": [{ "models": [row] }] } }
        });
        let row = json!({
            "id": "claude-sonnet-5-5",
            "name": "Sonnet 5.5",
            "section": "main"
        });
        let without_expiry = json!({
            "surfaces": { "cc": { "model_selector_config": [{ "models": [row] }] } }
        });

        // WHEN the agent completes initialize for each catalog
        let with_init = initialize_result(&admit_catalog(&with_expiry, None));
        let without_init = initialize_result(&admit_catalog(&without_expiry, None));

        // THEN both report the catalog ok, and only the document with an expiry carries it
        assert_eq!(
            with_init.pointer("/_meta/modelState/catalog/status"),
            Some(&json!("ok"))
        );
        assert_eq!(
            without_init.pointer("/_meta/modelState/catalog/status"),
            Some(&json!("ok"))
        );
        assert_eq!(
            with_init.pointer("/_meta/modelState/catalog/expiresAt"),
            Some(&json!("2026-10-16T08:03:29Z"))
        );
        assert!(
            without_init
                .pointer("/_meta/modelState/catalog/expiresAt")
                .is_none()
        );
    }
}
