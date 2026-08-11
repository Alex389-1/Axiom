use axiom_core::{
    errors::{AxiomError, Result},
    types::{PermissionCategory, ToolCall, ToolOutput, ToolResult},
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Every tool must implement this trait.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> Value;
    fn permission_category(&self) -> PermissionCategory;
    /// Returns true if this specific invocation should always individually prompt
    /// (overrides session-level grants — used for high-risk commands).
    fn is_high_risk(&self, _arguments: &Value) -> bool {
        false
    }
    async fn execute(&self, arguments: Value) -> Result<ToolOutput>;
}

/// Specification exposed to the model in the system prompt.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: Value,
    pub permission_category: PermissionCategory,
}

/// Central registry: maps tool name → `Arc<dyn Tool>`.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools
            .values()
            .map(|t| ToolSpec {
                name: t.name().to_string(),
                description: t.description().to_string(),
                schema: t.schema(),
                permission_category: t.permission_category(),
            })
            .collect()
    }

    /// Validate a tool call against the registered tool's JSON Schema.
    pub fn validate(&self, call: &ToolCall) -> Result<()> {
        let tool = self.tools.get(&call.tool).ok_or_else(|| AxiomError::ToolNotFound {
            name: call.tool.clone(),
        })?;

        let schema = tool.schema();
        let compiled = jsonschema::validator_for(&schema).map_err(|e| {
            AxiomError::ToolSchemaInvalid {
                tool: call.tool.clone(),
                reason: e.to_string(),
            }
        })?;

        compiled.validate(&call.arguments).map_err(|e| {
            AxiomError::ToolSchemaInvalid {
                tool: call.tool.clone(),
                reason: e.to_string(),
            }
        })?;
        Ok(())
    }

    /// Execute a validated tool call and wrap the result.
    pub async fn execute(&self, call: &ToolCall) -> Result<ToolResult> {
        let tool = self.tools.get(&call.tool).ok_or_else(|| AxiomError::ToolNotFound {
            name: call.tool.clone(),
        })?;

        let start = Instant::now();
        let output = tool.execute(call.arguments.clone()).await.unwrap_or_else(|e| {
            ToolOutput::Error { message: e.to_string() }
        });
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ToolResult {
            tool: call.tool.clone(),
            arguments: call.arguments.clone(),
            output,
            duration_ms,
            timestamp: chrono::Utc::now(),
        })
    }

    pub fn is_high_risk(&self, call: &ToolCall) -> bool {
        self.tools
            .get(&call.tool)
            .map(|t| t.is_high_risk(&call.arguments))
            .unwrap_or(false)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// We need async_trait for the Tool trait
// Add it as a dependency via a re-export trick (inline the macro usage)
// The crate uses #[async_trait::async_trait] which requires the async-trait crate.
// We declare it in the Cargo.toml via the registry module.

/// Build a default registry with all built-in tools.
pub fn default_registry(working_dir: std::path::PathBuf) -> ToolRegistry {
    use crate::{filesystem::FilesystemTool, git::GitTool, process::ProcessTool, terminal::TerminalExecTool};
    let mut registry = ToolRegistry::new();
    registry.register(TerminalExecTool::new(working_dir.clone()));
    registry.register(FilesystemTool::new(working_dir.clone()));
    registry.register(GitTool::new(working_dir.clone()));
    registry.register(ProcessTool::new());
    registry
}

/// High-risk command patterns — these always prompt individually.
pub fn is_high_risk_command(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    let patterns = [
        "sudo",
        "rm -rf",
        "rm -r /",
        "mkfs",
        "dd if=",
        "dd of=",
        "curl | sh",
        "curl|sh",
        "wget | sh",
        "wget|sh",
        "> /dev/",
        "chmod 777 /",
        ":(){ :|:& };:",
    ];
    patterns.iter().any(|p| trimmed.contains(p))
}
