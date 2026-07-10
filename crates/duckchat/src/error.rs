use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to spawn provider process: {0}")]
    Spawn(String),

    #[error("provider process failed: {0}")]
    Process(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    /// A previously-stored agent session id can no longer be resumed (e.g. grok
    /// `session/load` returned `FS_NOT_FOUND` after a cwd-key mismatch or
    /// pruning). Callers should drop the id and start a fresh agent session,
    /// re-feeding chat history as a preamble.
    #[error("agent session not found (cannot resume)")]
    SessionNotFound,

    #[error("cancelled")]
    Cancelled,

    /// Oneshot work exceeded the per-call wall-clock budget.
    #[error("oneshot timed out: {0}")]
    Timeout(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// True when this error means a stored resume id is dead and the turn
    /// should be retried as a fresh agent session.
    pub fn is_session_not_found(&self) -> bool {
        matches!(self, Error::SessionNotFound)
            || matches!(self, Error::Protocol(s) if protocol_indicates_session_not_found(s))
    }
}

/// Detect grok/ACP "session file missing" failures from a protocol error body
/// or Display string. Used both when mapping raw JSON-RPC errors and as a
/// defensive parse of already-stringified protocol errors.
pub fn protocol_indicates_session_not_found(msg: &str) -> bool {
    let has_load = msg.contains("session/load") || msg.contains("load failed");
    let missing = msg.contains("FS_NOT_FOUND")
        || msg.contains("Path not found")
        || msg.contains("No such file or directory")
        || msg.contains("os error 2");
    has_load && missing
}

/// True when a JSON-RPC `error` object from `session/load` means the session
/// path is gone (or never matched the cwd key).
pub fn rpc_error_is_session_not_found(err: &serde_json::Value) -> bool {
    if err
        .pointer("/data/code")
        .and_then(|v| v.as_str())
        == Some("FS_NOT_FOUND")
    {
        return true;
    }
    let message = err.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let detail = err
        .pointer("/data/detail")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    protocol_indicates_session_not_found(&format!("session/load failed: {message} {detail}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_fs_not_found_rpc() {
        let err = json!({
            "code": -32603,
            "data": {
                "code": "FS_NOT_FOUND",
                "detail": "No such file or directory (os error 2)"
            },
            "message": "Path not found."
        });
        assert!(rpc_error_is_session_not_found(&err));
    }

    #[test]
    fn detects_stringified_protocol_error() {
        let msg = r#"protocol error: grok session/load failed: {"code":-32603,"data":{"code":"FS_NOT_FOUND","detail":"No such file or directory (os error 2)"},"message":"Path not found."}"#;
        assert!(protocol_indicates_session_not_found(msg));
        assert!(Error::Protocol(msg.to_string()).is_session_not_found());
    }

    #[test]
    fn ignores_unrelated_protocol_errors() {
        assert!(!protocol_indicates_session_not_found(
            "grok session/prompt failed: boom"
        ));
        assert!(!rpc_error_is_session_not_found(&json!({
            "code": -32603,
            "message": "internal error"
        })));
    }
}
