//! Live Claude model catalog for ACP `initialize` advertise.
//!
//! Discovers models via Anthropic `GET /v1/models` using credentials available
//! to the official `claude` install (API key env, then macOS keychain OAuth,
//! then `~/.claude/.credentials.json`). On any failure, callers use
//! [`curated_fallback`].

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::{Value, json};

/// A model the agent advertises to the ACP host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdvertisedModel {
    pub id: String,
    pub name: String,
    pub context_window: Option<usize>,
}

/// Why live discovery did not yield a usable catalog.
#[derive(Debug)]
pub(crate) enum DiscoverError {
    NoAuth,
    Http(String),
    Empty,
    Other(String),
}

impl std::fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoverError::NoAuth => write!(f, "no Claude credentials available"),
            DiscoverError::Http(m) => write!(f, "models API: {m}"),
            DiscoverError::Empty => write!(f, "models API returned no models"),
            DiscoverError::Other(m) => write!(f, "{m}"),
        }
    }
}

/// Curated alias set used when live discovery fails. Non-empty by construction.
pub(crate) fn curated_fallback() -> Vec<AdvertisedModel> {
    vec![
        AdvertisedModel {
            id: "fable".into(),
            name: "Fable 5".into(),
            context_window: None,
        },
        AdvertisedModel {
            id: "opus".into(),
            name: "Opus 4.8".into(),
            context_window: None,
        },
        AdvertisedModel {
            id: "sonnet".into(),
            name: "Sonnet 4.6".into(),
            context_window: None,
        },
        AdvertisedModel {
            id: "haiku".into(),
            name: "Haiku 4.5".into(),
            context_window: None,
        },
    ]
}

/// Choose the advertise set from a live discovery result: success with a
/// non-empty list keeps that list; any failure or empty list yields the curated
/// alias fallback.
pub(crate) fn resolve_advertised_models(
    live: Result<Vec<AdvertisedModel>, DiscoverError>,
) -> Vec<AdvertisedModel> {
    match live {
        Ok(models) if !models.is_empty() => models,
        _ => curated_fallback(),
    }
}

/// Build the ACP `initialize` result value advertising `models`.
pub(crate) fn initialize_result(models: &[AdvertisedModel]) -> Value {
    let available: Vec<Value> = models
        .iter()
        .map(|m| {
            let mut entry = json!({
                "modelId": m.id,
                "name": m.name,
            });
            if let Some(window) = m.context_window {
                entry["_meta"] = json!({ "totalContextTokens": window });
            }
            entry
        })
        .collect();
    json!({
        "protocolVersion": 1,
        "agentCapabilities": {
            "loadSession": true,
        },
        "_meta": {
            "modelState": {
                "availableModels": available,
            }
        }
    })
}

/// Discover models from Anthropic using credentials available to Claude Code.
pub(crate) async fn discover_live_models() -> Result<Vec<AdvertisedModel>, DiscoverError> {
    let auth = resolve_auth().ok_or(DiscoverError::NoAuth)?;
    fetch_models(&auth).await
}

enum Auth {
    ApiKey(String),
    Bearer(String),
}

fn resolve_auth() -> Option<Auth> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Some(Auth::ApiKey(key));
        }
    }
    if let Some(token) = oauth_access_token() {
        return Some(Auth::Bearer(token));
    }
    None
}

/// Prefer a non-expired OAuth access token from the macOS keychain entry Claude
/// Code maintains, then `~/.claude/.credentials.json`.
fn oauth_access_token() -> Option<String> {
    for raw in [keychain_credentials_json(), file_credentials_json()]
        .into_iter()
        .flatten()
    {
        if let Some(token) = parse_oauth_access_token(&raw) {
            return Some(token);
        }
    }
    None
}

fn keychain_credentials_json() -> Option<String> {
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn file_credentials_json() -> Option<String> {
    let path = claude_credentials_path()?;
    std::fs::read_to_string(path).ok()
}

fn claude_credentials_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".claude").join(".credentials.json"))
}

fn parse_oauth_access_token(raw: &str) -> Option<String> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let oauth = v.get("claudeAiOauth")?;
    let token = oauth.get("accessToken")?.as_str()?.trim();
    if token.is_empty() {
        return None;
    }
    // Prefer non-expired tokens when expiresAt is present (ms since epoch).
    if let Some(exp) = oauth.get("expiresAt").and_then(Value::as_u64) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_millis() as u64;
        if exp <= now_ms {
            return None;
        }
    }
    Some(token.to_string())
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ApiModel>,
}

#[derive(Debug, Deserialize)]
struct ApiModel {
    id: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    max_input_tokens: Option<u64>,
}

async fn fetch_models(auth: &Auth) -> Result<Vec<AdvertisedModel>, DiscoverError> {
    let mut req = reqwest::Client::new()
        .get("https://api.anthropic.com/v1/models")
        .query(&[("limit", "1000")])
        .header("anthropic-version", "2023-06-01");
    req = match auth {
        Auth::ApiKey(key) => req.header("x-api-key", key),
        Auth::Bearer(token) => req.header("Authorization", format!("Bearer {token}")),
    };
    let resp = req
        .send()
        .await
        .map_err(|e| DiscoverError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(DiscoverError::Http(format!("{status}: {body}")));
    }
    let parsed: ModelsResponse = resp
        .json()
        .await
        .map_err(|e| DiscoverError::Other(e.to_string()))?;
    let models: Vec<AdvertisedModel> = parsed
        .data
        .into_iter()
        .filter(|m| !m.id.is_empty())
        .map(|m| {
            let name = m
                .display_name
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| m.id.clone());
            let context_window = m
                .max_input_tokens
                .filter(|&n| n > 0)
                .map(|n| n as usize);
            AdvertisedModel {
                id: m.id,
                name,
                context_window,
            }
        })
        .collect();
    if models.is_empty() {
        return Err(DiscoverError::Empty);
    }
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec harness/claude Agent model advertise: Successful live discovery advertises those models on initialize
    #[test]
    fn successful_live_discovery_advertises_those_models_on_initialize() {
        // GIVEN live Claude model discovery succeeding with a non-empty catalog
        let live = vec![
            AdvertisedModel {
                id: "claude-opus-4-8".into(),
                name: "Claude Opus 4.8".into(),
                context_window: Some(1_000_000),
            },
            AdvertisedModel {
                id: "claude-haiku-4-5-20251001".into(),
                name: "Claude Haiku 4.5".into(),
                context_window: Some(200_000),
            },
        ];

        // WHEN the agent completes initialize (advertise resolution)
        let advertised = resolve_advertised_models(Ok(live.clone()));
        let init = initialize_result(&advertised);

        // THEN the initialize result advertises that live catalog
        let available = init
            .pointer("/_meta/modelState/availableModels")
            .and_then(Value::as_array)
            .expect("availableModels array");
        assert_eq!(available.len(), 2);
        assert_eq!(available[0]["modelId"], "claude-opus-4-8");
        assert_eq!(available[0]["name"], "Claude Opus 4.8");
        assert_eq!(available[0]["_meta"]["totalContextTokens"], 1_000_000);
        assert_eq!(available[1]["modelId"], "claude-haiku-4-5-20251001");
        assert_eq!(available[1]["_meta"]["totalContextTokens"], 200_000);
    }

    /// @spec harness/claude Agent model advertise: Failed live discovery advertises the curated alias fallback
    #[test]
    fn failed_live_discovery_advertises_the_curated_alias_fallback() {
        // GIVEN live Claude model discovery failing
        // WHEN the agent completes initialize
        let advertised = resolve_advertised_models(Err(DiscoverError::NoAuth));
        let init = initialize_result(&advertised);

        // THEN the initialize result advertises the curated alias fallback set
        // AND the advertise set is non-empty
        let available = init
            .pointer("/_meta/modelState/availableModels")
            .and_then(Value::as_array)
            .expect("availableModels array");
        assert!(!available.is_empty(), "fallback advertise set must be non-empty");
        let ids: Vec<&str> = available
            .iter()
            .filter_map(|m| m.get("modelId").and_then(Value::as_str))
            .collect();
        for alias in ["fable", "opus", "sonnet", "haiku"] {
            assert!(
                ids.contains(&alias),
                "curated fallback missing {alias}: {ids:?}"
            );
        }
    }

    #[test]
    fn empty_live_list_uses_curated_fallback() {
        let advertised = resolve_advertised_models(Ok(Vec::new()));
        assert_eq!(advertised, curated_fallback());
    }
}
