//! Live Codex model catalog for ACP `initialize` advertise.
//!
//! Discovers models via official `codex app-server` `model/list`. On any
//! failure or empty list, the agent advertises **no** models so the host
//! picker stays empty when Codex is unavailable (Graceful unavailability).

use serde_json::{Value, json};

use crate::codex::{AppServer, AppServerError, CodexSpawnFactory};

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
    Process(String),
    Empty,
}

impl std::fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoverError::Process(m) => write!(f, "codex model/list: {m}"),
            DiscoverError::Empty => write!(f, "codex model/list returned no models"),
        }
    }
}

impl From<AppServerError> for DiscoverError {
    fn from(e: AppServerError) -> Self {
        DiscoverError::Process(e.to_string())
    }
}

/// Choose the advertise set: success with a non-empty list keeps that list;
/// any failure or empty list yields **no** models (backend unusable).
pub(crate) fn resolve_advertised_models(
    live: Result<Vec<AdvertisedModel>, DiscoverError>,
) -> Vec<AdvertisedModel> {
    match live {
        Ok(models) if !models.is_empty() => models,
        _ => Vec::new(),
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

/// Discover models from a short-lived `codex app-server` `model/list` call.
pub(crate) async fn discover_live_models(
    factory: &CodexSpawnFactory,
) -> Result<Vec<AdvertisedModel>, DiscoverError> {
    let mut server = AppServer::connect(factory).await?;
    let result = server.model_list().await;
    // Always tear down the discovery process — do not leave heat for sessions.
    server.kill().await;
    let result = result?;
    parse_model_list(&result)
}

/// Parse App Server `model/list` result into advertise entries.
///
/// Skips entries marked `hidden`. Prefers `displayName` when present.
pub(crate) fn parse_model_list(result: &Value) -> Result<Vec<AdvertisedModel>, DiscoverError> {
    let data = result
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| DiscoverError::Process("model/list missing data array".into()))?;

    let mut models = Vec::new();
    for entry in data {
        if entry.get("hidden").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let id = entry
            .get("id")
            .or_else(|| entry.get("model"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        let Some(id) = id else {
            continue;
        };
        let name = entry
            .get("displayName")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or(id);
        models.push(AdvertisedModel {
            id: id.to_string(),
            name: name.to_string(),
            context_window: None,
        });
    }

    if models.is_empty() {
        return Err(DiscoverError::Empty);
    }
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec harness/openai-codex Model discovery and oneshot preference: Discovered models are tagged with the openai-codex harness
    /// (agent-side half: live list is advertised on initialize; host tags harness)
    #[test]
    fn successful_live_discovery_advertises_those_models_on_initialize() {
        let live = vec![
            AdvertisedModel {
                id: "gpt-5.6-sol".into(),
                name: "GPT-5.6-Sol".into(),
                context_window: None,
            },
            AdvertisedModel {
                id: "gpt-5.4-mini".into(),
                name: "GPT-5.4-Mini".into(),
                context_window: None,
            },
        ];

        let advertised = resolve_advertised_models(Ok(live.clone()));
        let init = initialize_result(&advertised);

        let available = init
            .pointer("/_meta/modelState/availableModels")
            .and_then(Value::as_array)
            .expect("availableModels array");
        assert_eq!(available.len(), 2);
        assert_eq!(available[0]["modelId"], "gpt-5.6-sol");
        assert_eq!(available[0]["name"], "GPT-5.6-Sol");
        assert_eq!(available[1]["modelId"], "gpt-5.4-mini");
        assert_eq!(available[1]["name"], "GPT-5.4-Mini");
    }

    #[test]
    fn failed_live_discovery_advertises_empty() {
        let advertised =
            resolve_advertised_models(Err(DiscoverError::Process("spawn failed".into())));
        let init = initialize_result(&advertised);

        let available = init
            .pointer("/_meta/modelState/availableModels")
            .and_then(Value::as_array)
            .expect("availableModels array");
        assert!(
            available.is_empty(),
            "process failure must not advertise curated models, got {available:?}"
        );
    }

    #[test]
    fn empty_live_list_advertises_empty() {
        let advertised = resolve_advertised_models(Ok(Vec::new()));
        assert!(advertised.is_empty());
        let empty_err = resolve_advertised_models(Err(DiscoverError::Empty));
        assert!(empty_err.is_empty());
    }

    #[test]
    fn parse_model_list_skips_hidden_and_uses_display_name() {
        let result = json!({
            "data": [
                {
                    "id": "gpt-visible",
                    "displayName": "Visible Model",
                    "hidden": false
                },
                {
                    "id": "gpt-hidden",
                    "displayName": "Hidden",
                    "hidden": true
                },
                {
                    "id": "gpt-no-name",
                    "hidden": false
                }
            ],
            "nextCursor": null
        });
        let models = parse_model_list(&result).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gpt-visible");
        assert_eq!(models[0].name, "Visible Model");
        assert_eq!(models[1].id, "gpt-no-name");
        assert_eq!(models[1].name, "gpt-no-name");
    }

    #[test]
    fn parse_model_list_empty_data_is_error() {
        let result = json!({ "data": [], "nextCursor": null });
        assert!(matches!(
            parse_model_list(&result),
            Err(DiscoverError::Empty)
        ));
    }
}
