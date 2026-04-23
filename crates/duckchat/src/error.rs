use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to spawn provider process: {0}")]
    Spawn(String),

    #[error("provider process failed: {0}")]
    Process(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("cancelled")]
    Cancelled,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}
