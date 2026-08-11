use thiserror::Error;

#[derive(Debug, Error)]
pub enum AxiomError {
    // ── Provider errors ────────────────────────────────────────────────────
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Streaming error: {0}")]
    Streaming(String),

    // ── Tool errors ────────────────────────────────────────────────────────
    #[error("Tool not found: {name}")]
    ToolNotFound { name: String },

    #[error("Tool schema validation failed for '{tool}': {reason}")]
    ToolSchemaInvalid { tool: String, reason: String },

    #[error("Tool execution failed for '{tool}': {reason}")]
    ToolExecutionFailed { tool: String, reason: String },

    #[error("Permission denied for tool '{tool}' ({category:?})")]
    PermissionDenied {
        tool: String,
        category: crate::types::PermissionCategory,
    },

    // ── Parse errors ───────────────────────────────────────────────────────
    #[error("Action parse failed after all fallback modes: {0}")]
    ParseFailed(String),

    #[error("JSON schema validation failed: {0}")]
    SchemaValidation(String),

    // ── PTY / terminal errors ──────────────────────────────────────────────
    #[error("PTY error: {0}")]
    Pty(String),

    #[error("Shell not found: {0}")]
    ShellNotFound(String),

    // ── IPC / daemon errors ────────────────────────────────────────────────
    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Session not found: {id}")]
    SessionNotFound { id: String },

    // ── Configuration errors ───────────────────────────────────────────────
    #[error("Config error: {0}")]
    Config(String),

    // ── Database errors ────────────────────────────────────────────────────
    #[error("Database error: {0}")]
    Database(String),

    // ── General ───────────────────────────────────────────────────────────
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AxiomError>;
