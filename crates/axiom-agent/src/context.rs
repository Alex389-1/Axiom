use axiom_core::{config::AppConfig, types::ConversationMessage};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Assembles the context window for a model request.
///
/// Priority order (dropped first when over budget):
/// 1. System prompt (always included)
/// 2. Previous tool results from current session (highest relevance)
/// 3. Current open file content
/// 4. Keyword/grep retrieval results
/// 5. README summary
/// 6. Git diff (last change)
/// 7. Project manifest (Cargo.toml / package.json)
pub struct ContextManager {
    project_root: Option<PathBuf>,
    max_tokens: usize,
}

impl ContextManager {
    pub fn new(project_root: Option<PathBuf>, max_tokens: usize) -> Self {
        Self {
            project_root,
            max_tokens,
        }
    }

    /// Build the system prompt for the agent, including tool specs.
    pub fn build_system_prompt(&self, tool_specs_json: &str) -> String {
        let project_info = self
            .project_root
            .as_ref()
            .map(|p| format!("Working Directory: {}", p.display()))
            .unwrap_or_else(|| "No workspace selected.".into());

        let mut prompt = format!(
            r#"You are Axiom, a sleek and intelligent AI assistant for software development powered by local language models.

{project_info}

# Instructions
- **CRITICAL**: You are running in a local environment on the user's machine. You CAN and MUST execute terminal commands, read/write files, and perform actions when requested. Do NOT refuse by saying "I cannot execute commands" or "I am an AI".
- Provide clear, well-structured, and helpful answers.
- When asked for explanations, concepts, or "easy" overviews, provide simple natural language explanations first before presenting code examples.
- Format all code snippets with standard markdown code blocks and syntax highlighting.
- Be direct, conversational, and answer exactly what the user asks."#,
            project_info = project_info,
        );

        if !tool_specs_json.is_empty() {
            prompt.push_str(&format!(
                "\n\n# Tools Available\nYou have access to the following tools to interact with the user's system. You may call a tool by outputting a JSON object matching this schema: {{\"tool\": \"<tool_name>\", \"arguments\": {{...}}}}. You can output your reasoning first, but you must include the JSON object to execute the tool.\n\n{}",
                tool_specs_json
            ));
        }

        prompt
    }

    /// Assemble context for a new request, enforcing the token budget.
    pub async fn assemble_context(
        &self,
        user_query: &str,
        recent_history: &[ConversationMessage],
    ) -> Vec<ContextPiece> {
        let mut pieces = Vec::new();

        // 1. Recent tool call results (most relevant)
        for msg in recent_history.iter().rev().take(10) {
            if let Some(result) = &msg.tool_result {
                pieces.push(ContextPiece {
                    priority: 10,
                    label: format!("Tool result: {}", result.tool),
                    content: result.output.to_context_string(),
                });
            }
        }

        // 2. Current file / last-opened file (if in query)
        if let Some(file_ctx) = self.extract_file_from_query(user_query).await {
            pieces.push(ContextPiece {
                priority: 9,
                label: format!("File: {}", file_ctx.0),
                content: file_ctx.1,
            });
        }

        // 3. Keyword/grep retrieval
        let keywords = extract_keywords(user_query);
        if !keywords.is_empty() {
            if let Some(root) = &self.project_root {
                let results = self.ripgrep_search(root, &keywords).await;
                if !results.is_empty() {
                    pieces.push(ContextPiece {
                        priority: 7,
                        label: "Relevant code snippets".into(),
                        content: results,
                    });
                }
            }
        }

        // 4. Git diff
        if let Some(diff) = self.get_git_diff().await {
            pieces.push(ContextPiece {
                priority: 5,
                label: "Recent git diff".into(),
                content: diff,
            });
        }

        // 5. README summary
        if let Some(readme) = self.get_readme_summary().await {
            pieces.push(ContextPiece {
                priority: 4,
                label: "README".into(),
                content: readme,
            });
        }

        // 6. Project manifest
        if let Some(manifest) = self.get_project_manifest().await {
            pieces.push(ContextPiece {
                priority: 3,
                label: "Project manifest".into(),
                content: manifest,
            });
        }

        // Sort by priority descending and enforce token budget
        pieces.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.apply_token_budget(pieces)
    }

    fn apply_token_budget(&self, pieces: Vec<ContextPiece>) -> Vec<ContextPiece> {
        let mut budget = self.max_tokens;
        let mut result = Vec::new();

        for piece in pieces {
            // Rough token estimate: ~4 chars per token
            let tokens = piece.content.len() / 4;
            if tokens <= budget {
                budget -= tokens;
                result.push(piece);
            } else if budget > 100 {
                // Include a truncated version
                let max_chars = budget * 4;
                let truncated = piece.content[..max_chars.min(piece.content.len())].to_string();
                result.push(ContextPiece {
                    priority: piece.priority,
                    label: format!("{} [truncated]", piece.label),
                    content: truncated,
                });
                break;
            } else {
                break;
            }
        }

        result
    }

    async fn extract_file_from_query(&self, query: &str) -> Option<(String, String)> {
        let root = self.project_root.as_ref()?;

        // Look for file paths mentioned in the query
        let path_re = regex::Regex::new(r"[a-zA-Z0-9_./\-]+\.[a-zA-Z]{1,6}").ok()?;
        for cap in path_re.find_iter(query) {
            let candidate = cap.as_str();
            let full_path = root.join(candidate);
            if full_path.exists() && full_path.is_file() {
                if let Ok(content) = tokio::fs::read_to_string(&full_path).await {
                    let content = if content.len() > 8000 {
                        format!("[truncated]\n{}", &content[..8000])
                    } else {
                        content
                    };
                    return Some((candidate.to_string(), content));
                }
            }
        }
        None
    }

    async fn ripgrep_search(&self, root: &Path, keywords: &[String]) -> String {
        let query = keywords.join("|");
        let output = tokio::process::Command::new("rg")
            .args([
                "--line-number",
                "--with-filename",
                "--color=never",
                "-m", "3",
                "-e", &query,
                root.to_str().unwrap_or("."),
            ])
            .output()
            .await;

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let lines: Vec<&str> = stdout.lines().take(30).collect();
                lines.join("\n")
            }
            Err(_) => String::new(),
        }
    }

    async fn get_git_diff(&self) -> Option<String> {
        let root = self.project_root.as_ref()?;
        let output = tokio::process::Command::new("git")
            .args(["diff", "--stat", "HEAD"])
            .current_dir(root)
            .output()
            .await
            .ok()?;

        let diff = String::from_utf8_lossy(&output.stdout).to_string();
        if diff.trim().is_empty() {
            None
        } else {
            let lines: Vec<&str> = diff.lines().take(20).collect();
            Some(lines.join("\n"))
        }
    }

    async fn get_readme_summary(&self) -> Option<String> {
        let root = self.project_root.as_ref()?;
        for name in &["README.md", "README.txt", "README", "readme.md"] {
            let path = root.join(name);
            if path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    // First ~500 tokens (~2000 chars)
                    let summary = &content[..content.len().min(2000)];
                    return Some(summary.to_string());
                }
            }
        }
        None
    }

    async fn get_project_manifest(&self) -> Option<String> {
        let root = self.project_root.as_ref()?;
        for name in &["Cargo.toml", "package.json", "pyproject.toml", "go.mod"] {
            let path = root.join(name);
            if path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let content = &content[..content.len().min(3000)];
                    return Some(format!("[{}]\n{}", name, content));
                }
            }
        }
        None
    }
}

/// A piece of context with a priority score for budget enforcement.
#[derive(Debug, Clone)]
pub struct ContextPiece {
    /// Higher = kept first when trimming.
    pub priority: u8,
    pub label: String,
    pub content: String,
}

/// Extract meaningful keywords from a user query for grep-based retrieval.
fn extract_keywords(query: &str) -> Vec<String> {
    // Strip common English stop words and keep likely identifiers/terms
    let stop_words = [
        "the", "a", "an", "is", "it", "in", "on", "at", "to", "do", "can", "you",
        "me", "my", "we", "i", "of", "for", "and", "or", "but", "not", "be",
        "have", "has", "with", "that", "this", "from", "fix", "please",
    ];

    query
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| !stop_words.contains(&w.as_str()))
        .take(5) // Limit to top 5 keywords
        .collect()
}
