use async_trait::async_trait;
use axiom_core::{
    errors::Result,
    types::{PermissionCategory, ToolOutput},
};
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::registry::Tool;

pub struct GitTool {
    working_dir: PathBuf,
}

impl GitTool {
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Inspect the Git repository state. Supported operations: status (working tree changes), \
         diff (changes since last commit or between refs), log (recent commit history)."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["operation"],
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["status", "diff", "log"],
                    "description": "Git operation to perform"
                },
                "args": {
                    "type": "string",
                    "description": "Additional git arguments (e.g. 'HEAD~3' for diff, '-n 10' for log)"
                }
            },
            "additionalProperties": false
        })
    }

    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Git
    }

    async fn execute(&self, arguments: Value) -> Result<ToolOutput> {
        let operation = arguments["operation"].as_str().unwrap_or("status");
        let args = arguments
            .get("args")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut cmd_args = match operation {
            "status" => vec!["status", "--short", "--branch"],
            "diff" => vec!["diff"],
            "log" => vec!["log", "--oneline", "-20"],
            _ => {
                return Ok(ToolOutput::Error {
                    message: format!("Unknown git operation: {}", operation),
                })
            }
        };

        // Append user-supplied extra args safely (split by whitespace)
        let extra: Vec<&str> = if args.is_empty() {
            vec![]
        } else {
            args.split_whitespace().collect()
        };
        cmd_args.extend(extra.iter().copied());

        let output = tokio::process::Command::new("git")
            .args(&cmd_args)
            .current_dir(&self.working_dir)
            .output()
            .await;

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                Ok(ToolOutput::Success {
                    stdout: truncate(&stdout, 150),
                    stderr,
                    exit_code: o.status.code().unwrap_or(-1),
                })
            }
            Err(e) => Ok(ToolOutput::Error {
                message: format!("git command failed: {}", e),
            }),
        }
    }
}

fn truncate(s: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= max_lines {
        s.to_string()
    } else {
        let skipped = lines.len() - max_lines;
        format!(
            "[... {} lines truncated ...]\n{}",
            skipped,
            lines[skipped..].join("\n")
        )
    }
}
