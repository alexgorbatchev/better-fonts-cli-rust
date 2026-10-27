//! Transactional macOS application font patching and lossless ASAR editing.
pub mod app;
pub mod archive;
pub mod cli;
pub mod config;
pub mod fuses;
pub mod injection;
mod native;
pub mod process;
mod transaction;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Parse(#[from] clap::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("operation cancelled")]
    Cancelled,
    #[error("{program} failed: {message}")]
    Process { program: String, message: String },
    #[error("changes committed, but {0}")]
    Committed(String),
    #[error("{0}")]
    Dependency(#[from] cli_deps::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
pub(crate) fn parent(path: &std::path::Path) -> &std::path::Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."))
}
