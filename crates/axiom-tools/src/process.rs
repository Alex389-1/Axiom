use async_trait::async_trait;
use axiom_core::{
    errors::Result,
    types::{PermissionCategory, ToolOutput},
};
use serde_json::{json, Value};

use crate::registry::Tool;

pub struct ProcessTool;

impl ProcessTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProcessTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "List or kill running processes. Operations: list (show user's processes), kill (send signal to a PID)."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["operation"],
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["list", "kill"],
                    "description": "Process operation"
                },
                "pid": {
                    "type": "integer",
                    "description": "Process ID (required for kill operation)"
                },
                "signal": {
                    "type": "string",
                    "enum": ["SIGTERM", "SIGKILL", "SIGINT"],
                    "description": "Signal to send (default: SIGTERM)",
                    "default": "SIGTERM"
                }
            },
            "additionalProperties": false
        })
    }

    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Process
    }

    fn is_high_risk(&self, arguments: &Value) -> bool {
        // Killing processes is always flagged as requiring confirmation
        arguments.get("operation").and_then(|v| v.as_str()) == Some("kill")
    }

    async fn execute(&self, arguments: Value) -> Result<ToolOutput> {
        let operation = arguments["operation"].as_str().unwrap_or("list");

        match operation {
            "list" => {
                let output = tokio::process::Command::new("ps")
                    .args(["aux", "--no-header"])
                    .output()
                    .await;

                match output {
                    Ok(o) => {
                        let stdout = String::from_utf8_lossy(&o.stdout);
                        // Return only first 50 processes to avoid context overflow
                        let lines: Vec<&str> = stdout.lines().take(50).collect();
                        Ok(ToolOutput::Success {
                            stdout: lines.join("\n"),
                            stderr: String::new(),
                            exit_code: 0,
                        })
                    }
                    Err(e) => Ok(ToolOutput::Error {
                        message: format!("ps failed: {}", e),
                    }),
                }
            }
            "kill" => {
                let pid = match arguments.get("pid").and_then(|v| v.as_i64()) {
                    Some(p) => p,
                    None => {
                        return Ok(ToolOutput::Error {
                            message: "pid is required for kill operation".into(),
                        })
                    }
                };

                let signal = arguments
                    .get("signal")
                    .and_then(|v| v.as_str())
                    .unwrap_or("SIGTERM");

                let sig_flag = match signal {
                    "SIGKILL" => "-9",
                    "SIGINT" => "-2",
                    _ => "-15", // SIGTERM
                };

                let output = tokio::process::Command::new("kill")
                    .args([sig_flag, &pid.to_string()])
                    .output()
                    .await;

                match output {
                    Ok(o) => Ok(ToolOutput::Success {
                        stdout: format!("Sent {} to PID {}", signal, pid),
                        stderr: String::from_utf8_lossy(&o.stderr).to_string(),
                        exit_code: o.status.code().unwrap_or(-1),
                    }),
                    Err(e) => Ok(ToolOutput::Error {
                        message: format!("kill failed: {}", e),
                    }),
                }
            }
            _ => Ok(ToolOutput::Error {
                message: format!("Unknown process operation: {}", operation),
            }),
        }
    }
}
