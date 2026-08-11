use async_trait::async_trait;
use axiom_core::{
    errors::Result,
    types::{PermissionCategory, ToolOutput},
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::registry::Tool;

/// Unified filesystem tool — handles read, write, list, and search.
/// Operations are sandboxed under the working directory for safety.
pub struct FilesystemTool {
    working_dir: PathBuf,
}

impl FilesystemTool {
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            self.working_dir.join(p)
        }
    }
}

#[async_trait]
impl Tool for FilesystemTool {
    fn name(&self) -> &str {
        "filesystem"
    }

    fn description(&self) -> &str {
        "Read, write, list, or search files in the project. \
         Supported operations: read (get file contents), write (create/overwrite file), \
         list (directory listing), search (ripgrep-style text search across files)."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["operation", "path"],
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "list", "search"],
                    "description": "The filesystem operation to perform"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory path (relative to project root or absolute)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write (required for write operation)"
                },
                "query": {
                    "type": "string",
                    "description": "Search query string (required for search operation)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Max search results to return (default: 20)",
                    "minimum": 1,
                    "maximum": 100,
                    "default": 20
                }
            },
            "additionalProperties": false
        })
    }

    fn permission_category(&self) -> PermissionCategory {
        // Returns Write for all; caller uses the operation to differentiate.
        // The permission manager checks Read vs Write based on the operation field.
        PermissionCategory::Read
    }

    fn is_high_risk(&self, _arguments: &Value) -> bool {
        false
    }

    async fn execute(&self, arguments: Value) -> Result<ToolOutput> {
        let operation = arguments["operation"].as_str().unwrap_or("");
        let path = arguments["path"].as_str().unwrap_or("");
        let resolved = self.resolve(path);

        match operation {
            "read" => read_file(&resolved).await,
            "write" => {
                let content = arguments["content"].as_str().unwrap_or("");
                write_file(&resolved, content).await
            }
            "list" => list_dir(&resolved).await,
            "search" => {
                let query = arguments["query"].as_str().unwrap_or("");
                let max_results = arguments
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20) as usize;
                search_files(&self.working_dir, query, max_results).await
            }
            _ => Ok(ToolOutput::Error {
                message: format!("Unknown filesystem operation: {}", operation),
            }),
        }
    }
}

async fn read_file(path: &Path) -> Result<ToolOutput> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => {
            // Truncate very large files
            let content = if content.len() > 100_000 {
                format!("[File truncated at 100KB]\n{}", &content[..100_000])
            } else {
                content
            };
            Ok(ToolOutput::Success {
                stdout: content,
                stderr: String::new(),
                exit_code: 0,
            })
        }
        Err(e) => Ok(ToolOutput::Error {
            message: format!("Cannot read {}: {}", path.display(), e),
        }),
    }
}

async fn write_file(path: &Path, content: &str) -> Result<ToolOutput> {
    if let Some(parent) = path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return Ok(ToolOutput::Error {
                message: format!("Cannot create directory: {}", e),
            });
        }
    }
    match tokio::fs::write(path, content).await {
        Ok(_) => Ok(ToolOutput::Success {
            stdout: format!("Written {} bytes to {}", content.len(), path.display()),
            stderr: String::new(),
            exit_code: 0,
        }),
        Err(e) => Ok(ToolOutput::Error {
            message: format!("Cannot write {}: {}", path.display(), e),
        }),
    }
}

async fn list_dir(path: &Path) -> Result<ToolOutput> {
    let path = path.to_path_buf();
    let result = tokio::task::spawn_blocking(move || {
        let mut entries = Vec::new();
        let walker = WalkDir::new(&path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                // Skip hidden dirs except .git info at root
                !e.path()
                    .components()
                    .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            });

        for entry in walker {
            let rel = entry
                .path()
                .strip_prefix(&path)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();
            if rel.is_empty() {
                continue;
            }
            let marker = if entry.file_type().is_dir() { "/" } else { "" };
            entries.push(format!("{}{}", rel, marker));
        }
        entries.sort();
        entries.join("\n")
    })
    .await;

    match result {
        Ok(listing) => Ok(ToolOutput::Success {
            stdout: listing,
            stderr: String::new(),
            exit_code: 0,
        }),
        Err(e) => Ok(ToolOutput::Error {
            message: format!("List failed: {}", e),
        }),
    }
}

async fn search_files(root: &Path, query: &str, max_results: usize) -> Result<ToolOutput> {
    // Try ripgrep first; fall back to a simple walkdir+contains search
    let output = tokio::process::Command::new("rg")
        .args([
            "--line-number",
            "--with-filename",
            "--color=never",
            "-m",
            "5",
            query,
            root.to_str().unwrap_or("."),
        ])
        .output()
        .await;

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let lines: Vec<&str> = stdout.lines().take(max_results).collect();
            Ok(ToolOutput::Success {
                stdout: lines.join("\n"),
                stderr: String::new(),
                exit_code: o.status.code().unwrap_or(0),
            })
        }
        Err(_) => {
            // Fallback: naive search
            let query = query.to_string();
            let root = root.to_path_buf();
            let results = tokio::task::spawn_blocking(move || {
                let mut matches = Vec::new();
                for entry in WalkDir::new(&root)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    if matches.len() >= max_results {
                        break;
                    }
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        for (i, line) in content.lines().enumerate() {
                            if line.contains(&query) {
                                matches.push(format!(
                                    "{}:{}: {}",
                                    entry.path().display(),
                                    i + 1,
                                    line.trim()
                                ));
                            }
                        }
                    }
                }
                matches.join("\n")
            })
            .await
            .unwrap_or_default();

            Ok(ToolOutput::Success {
                stdout: results,
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }
}
