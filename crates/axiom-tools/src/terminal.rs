use async_trait::async_trait;
use axiom_core::{
    errors::Result,
    types::{PermissionCategory, ToolOutput},
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use crate::registry::{is_high_risk_command, Tool};

/// `terminal.exec` — run a shell command in the project working directory,
/// capturing stdout/stderr and the exit code.
pub struct TerminalExecTool {
    working_dir: PathBuf,
}

impl TerminalExecTool {
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }
}

#[async_trait]
impl Tool for TerminalExecTool {
    fn name(&self) -> &str {
        "terminal.exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the project directory. Returns stdout, stderr, and exit code. \
         Use for running tests, builds, searches, and other shell operations. \
         Commands run in the project working directory."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30, max: 300)",
                    "minimum": 1,
                    "maximum": 300,
                    "default": 30
                }
            },
            "additionalProperties": false
        })
    }

    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Execute
    }

    fn is_high_risk(&self, arguments: &Value) -> bool {
        if let Some(cmd) = arguments.get("command").and_then(|v| v.as_str()) {
            is_high_risk_command(cmd)
        } else {
            false
        }
    }

    async fn execute(&self, arguments: Value) -> Result<ToolOutput> {
        let command = arguments["command"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let timeout_secs = arguments
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30)
            .min(300);

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());

        let result = timeout(
            Duration::from_secs(timeout_secs),
            run_command(&shell, &command, &self.working_dir),
        )
        .await;

        match result {
            Ok(output) => Ok(output),
            Err(_) => Ok(ToolOutput::Error {
                message: format!("Command timed out after {}s: {}", timeout_secs, command),
            }),
        }
    }
}

async fn run_command(shell: &str, command: &str, cwd: &PathBuf) -> ToolOutput {
    let mut child = match Command::new(shell)
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return ToolOutput::Error {
                message: format!("Failed to spawn command: {}", e),
            }
        }
    };

    let stdout_handle = child.stdout.take().expect("stdout piped");
    let stderr_handle = child.stderr.take().expect("stderr piped");

    // Collect stdout
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout_handle);
        let mut out = String::new();
        let _ = reader.read_to_string(&mut out).await;
        out
    });

    // Collect stderr
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr_handle);
        let mut err = String::new();
        let _ = reader.read_to_string(&mut err).await;
        err
    });

    let status = match child.wait().await {
        Ok(s) => s,
        Err(e) => {
            return ToolOutput::Error {
                message: format!("Command failed: {}", e),
            }
        }
    };

    let stdout = stdout_task.await.unwrap_or_default();
    let stderr = stderr_task.await.unwrap_or_default();

    // Truncate very long output to avoid context overflow (keep last 200 lines)
    let stdout = truncate_output(&stdout, 200);
    let stderr = truncate_output(&stderr, 50);

    ToolOutput::Success {
        stdout,
        stderr,
        exit_code: status.code().unwrap_or(-1),
    }
}

fn truncate_output(s: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= max_lines {
        s.to_string()
    } else {
        let skipped = lines.len() - max_lines;
        let kept: Vec<&str> = lines[skipped..].to_vec();
        format!("[... {} lines truncated ...]\n{}", skipped, kept.join("\n"))
    }
}

// We need read_to_string on AsyncBufReadExt
use tokio::io::AsyncReadExt;
