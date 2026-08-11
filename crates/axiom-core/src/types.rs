use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Tool protocol ────────────────────────────────────────────────────────────

/// A structured tool call emitted by the model (after parsing).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Dot-namespaced tool name, e.g. "terminal.exec", "filesystem.read"
    pub tool: String,
    /// Arguments matching the tool's JSON Schema.
    pub arguments: serde_json::Value,
}

/// Result returned after a tool executes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: String,
    pub arguments: serde_json::Value,
    pub output: ToolOutput,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ToolOutput {
    Success { stdout: String, stderr: String, exit_code: i32 },
    Error { message: String },
}

impl ToolOutput {
    pub fn is_success(&self) -> bool {
        matches!(self, ToolOutput::Success { exit_code, .. } if *exit_code == 0)
    }

    pub fn to_context_string(&self) -> String {
        match self {
            ToolOutput::Success { stdout, stderr, exit_code } => {
                let mut s = String::new();
                if !stdout.is_empty() {
                    s.push_str(stdout);
                }
                if !stderr.is_empty() {
                    if !s.is_empty() { s.push('\n'); }
                    s.push_str("[stderr] ");
                    s.push_str(stderr);
                }
                if *exit_code != 0 {
                    s.push_str(&format!("\n[exit code: {}]", exit_code));
                }
                s
            }
            ToolOutput::Error { message } => format!("[error] {}", message),
        }
    }
}

// ─── Agent action ─────────────────────────────────────────────────────────────

/// What the agent wants to do next, after the model emits a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Model is done; return this text to the user.
    Final { text: String },
    /// Model wants to call a tool.
    Tool(ToolCall),
}

// ─── Conversation ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    Tool,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: Role,
    pub content: String,
    /// Attached tool call (for assistant messages that include a tool invocation).
    pub tool_call: Option<ToolCall>,
    /// Attached tool result (for tool messages).
    pub tool_result: Option<ToolResult>,
    pub timestamp: DateTime<Utc>,
}

impl ConversationMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::User,
            content: content.into(),
            tool_call: None,
            tool_result: None,
            timestamp: Utc::now(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content: content.into(),
            tool_call: None,
            tool_result: None,
            timestamp: Utc::now(),
        }
    }

    pub fn tool_result(call: ToolCall, result: ToolResult) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::Tool,
            content: result.output.to_context_string(),
            tool_call: Some(call),
            tool_result: Some(result),
            timestamp: Utc::now(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: Role::System,
            content: content.into(),
            tool_call: None,
            tool_result: None,
            timestamp: Utc::now(),
        }
    }
}

// ─── Model info ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<DateTime<Utc>>,
}

// ─── Model capability profile ─────────────────────────────────────────────────

/// Auto-probed capability profile for a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub model: String,
    /// Which tool-calling mode works best for this model.
    pub tool_mode: ToolCallingMode,
    pub supports_thinking: bool,
    pub supports_constrained_decoding: bool,
    pub detected_at: String, // "probe" | "manual"
    pub last_verified: DateTime<Utc>,
    /// Optional model file hash (for drift detection across quantizations).
    pub model_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallingMode {
    /// Model supports native function calling.
    Native,
    /// Model reliably outputs structured JSON tool calls.
    Structured,
    /// Model needs XML-tagged extraction.
    Tagged,
    /// ReAct Thought/Action/Input fallback.
    React,
}

// ─── Permissions ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PermissionCategory {
    Read,
    Write,
    Execute,
    Network,
    Delete,
    Git,
    Process,
}

impl PermissionCategory {
    /// Returns true for categories that can never be session-cached
    /// (always prompt individually regardless of prior grants).
    pub fn always_prompt(&self) -> bool {
        false // individual high-risk commands are filtered at the command level
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    Once,
    Session,
    Project,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub category: PermissionCategory,
    pub scope: PermissionScope,
    pub granted_at: DateTime<Utc>,
    /// For project-scoped grants: which project root.
    pub project: Option<String>,
}

// ─── Jobs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub label: String,
    pub status: JobStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

// ─── IPC message types ────────────────────────────────────────────────────────

/// Requests sent from the Tauri frontend → daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonRequest {
    ListModels,
    SendMessage {
        session_id: String,
        content: String,
    },
    DeleteLastTurn {
        session_id: String,
    },
    GetConversation {
        session_id: String,
    },
    CreateSession {
        project_path: Option<String>,
        model: Option<String>,
    },
    GetTerminalOutput {
        session_id: String,
        lines: Option<usize>,
    },
    WriteTerminal {
        session_id: String,
        input: String,
    },
    ResizeTerminal {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    SetPermission {
        session_id: String,
        category: PermissionCategory,
        scope: PermissionScope,
    },
    ListJobs {
        session_id: String,
    },
    GetModelProfiles,
    ProbeModel {
        model: String,
    },
    ListProjects,
    GetConfig,
    UpdateConfig {
        config: serde_json::Value,
    },
    GetEvents {
        session_id: String,
    },
    StopAgent {
        session_id: String,
    },
    Shutdown,
}

/// Responses sent from daemon → Tauri frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    Ok,
    Error { message: String },
    Models { models: Vec<ModelInfo> },
    Session { session_id: String },
    Conversation { messages: Vec<ConversationMessage> },
    TerminalOutput { lines: Vec<String> },
    Jobs { jobs: Vec<Job> },
    ModelProfiles { profiles: HashMap<String, ModelProfile> },
    Config { config: serde_json::Value },
    Events { events: Vec<DaemonEvent> },
}

/// Streaming events pushed from daemon → frontend (via Tauri events).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonEvent {
    /// Streaming token from model generation.
    Token { session_id: String, token: String },
    /// A tool call is about to be executed.
    ToolCallStarted { session_id: String, call: ToolCall, step: u32 },
    /// A tool call completed.
    ToolCallCompleted { session_id: String, result: ToolResult, step: u32 },
    /// Agent finished the current turn.
    TurnCompleted { session_id: String, message: ConversationMessage },
    /// Permission needed before proceeding.
    PermissionRequired {
        session_id: String,
        call: ToolCall,
        category: PermissionCategory,
        is_high_risk: bool,
    },
    /// Terminal output line.
    TerminalOutput { session_id: String, data: String },
    /// Agent step update (for timeline display).
    AgentStep {
        session_id: String,
        step: u32,
        description: String,
        status: StepStatus,
    },
    /// Error during agent loop.
    AgentError { session_id: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Running,
    Completed,
    Warning,
    Failed,
}
