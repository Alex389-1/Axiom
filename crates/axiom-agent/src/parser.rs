use axiom_core::{
    errors::{AxiomError, Result},
    types::{Action, ToolCall},
};
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;
use tracing::{debug, warn};

// ─── Regex patterns (compiled once) ──────────────────────────────────────────

fn tool_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)<tool>\s*(.*?)\s*</tool>").expect("tool tag regex")
    })
}

fn think_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)<think>.*?</think>").expect("think tag regex")
    })
}

fn command_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)<command>\s*(.*?)\s*</command>").expect("command tag regex")
    })
}

fn react_action_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?m)^Action:\s*(.+)$").expect("react action regex")
    })
}

fn react_input_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?m)^(?:Action )?Input:\s*(.+)$").expect("react input regex")
    })
}

/// Hints that indicate the model is done (no more tool calls).
const FINAL_MARKERS: &[&str] = &[
    "FINAL ANSWER:",
    "Final Answer:",
    "final answer:",
];

// ─── Public API ───────────────────────────────────────────────────────────────

/// Parse a raw model response into an `Action`.
///
/// Parsing modes attempted in order:
/// 1. Structured JSON — the whole response is a JSON object with `tool` + `arguments`.
/// 2. JSON repair — bracket-match to extract embedded JSON from free text, then validate.
/// 3. Tagged extraction — `<tool>...</tool>` or `<command>...</command>` XML tags.
/// 4. ReAct fallback — `Action: / Input:` line-based extraction.
///
/// If all modes fail, returns `Action::Final` with the raw text so the user
/// still gets a response rather than a crash.
pub fn parse_action(response: &str) -> Result<Action> {
    let trimmed = response.trim();

    // Quick check: is this a final answer?
    for marker in FINAL_MARKERS {
        if let Some(pos) = trimmed.find(marker) {
            let text = trimmed[pos + marker.len()..].trim().to_string();
            return Ok(Action::Final { text });
        }
    }

    // Mode 1: Full JSON
    if let Some(action) = try_parse_structured(trimmed) {
        debug!("Parser: mode=structured");
        return Ok(action);
    }

    // Mode 2: Bracket-match repair
    if let Some(action) = try_parse_repaired(trimmed) {
        debug!("Parser: mode=repaired");
        return Ok(action);
    }

    // Mode 3: XML tags
    if let Some(action) = try_parse_tagged(trimmed) {
        debug!("Parser: mode=tagged");
        return Ok(action);
    }

    // Mode 4: ReAct
    if let Some(action) = try_parse_react(trimmed) {
        debug!("Parser: mode=react");
        return Ok(action);
    }

    // All modes failed — return as final text
    warn!("Parser: all modes failed, returning as Final");
    Ok(Action::Final {
        text: trimmed.to_string(),
    })
}

/// Parse after a constrained-decoding attempt that produced schema-valid JSON.
/// Only does JSON validation (no fallback modes needed since the model was constrained).
pub fn parse_constrained(response: &str) -> Result<Action> {
    let trimmed = response.trim();
    match try_parse_structured(trimmed) {
        Some(action) => Ok(action),
        None => Err(AxiomError::ParseFailed(format!(
            "Constrained decoding produced invalid JSON: {}",
            &trimmed[..trimmed.len().min(200)]
        ))),
    }
}

/// Build the retry prompt to send to the model when parsing fails.
pub fn build_retry_prompt(bad_response: &str, schema_hint: &str) -> String {
    format!(
        "Your previous response could not be parsed as a valid tool call. \
        Please respond with ONLY a valid JSON object in this exact format:\n\
        {}\n\n\
        Your previous response was:\n{}\n\n\
        Try again now with ONLY the JSON object, no extra text:",
        schema_hint,
        &bad_response[..bad_response.len().min(500)]
    )
}

// ─── Mode implementations ─────────────────────────────────────────────────────

fn try_parse_structured(text: &str) -> Option<Action> {
    // The whole text is valid JSON
    let v: Value = serde_json::from_str(text).ok()?;
    json_to_action(v)
}

fn try_parse_repaired(text: &str) -> Option<Action> {
    // Find all JSON objects via bracket matching and try them
    let json_strs = extract_json_objects(text);
    for json_str in json_strs {
        if let Ok(v) = serde_json::from_str(&json_str) {
            if let Some(action) = json_to_action(v) {
                return Some(action);
            }
        }
    }
    None
}

fn try_parse_tagged(text: &str) -> Option<Action> {
    // Try <tool>JSON</tool>
    if let Some(cap) = tool_tag_re().captures(text) {
        let inner = cap.get(1)?.as_str();
        // inner might be JSON or just a tool name
        if let Ok(v) = serde_json::from_str::<Value>(inner) {
            if let Some(action) = json_to_action(v) {
                return Some(action);
            }
        }
        // Treat as tool name with no args
        return Some(Action::Tool(ToolCall {
            tool: inner.trim().to_string(),
            arguments: serde_json::json!({}),
        }));
    }

    // Try <command>shell command</command>
    if let Some(cap) = command_tag_re().captures(text) {
        let cmd = cap.get(1)?.as_str().trim();
        return Some(Action::Tool(ToolCall {
            tool: "terminal.exec".into(),
            arguments: serde_json::json!({ "command": cmd }),
        }));
    }

    None
}

fn try_parse_react(text: &str) -> Option<Action> {
    let action_match = react_action_re().captures(text)?;
    let action_name = action_match.get(1)?.as_str().trim();

    // Check for "Final" ReAct action
    if action_name.eq_ignore_ascii_case("final answer") {
        let remaining = &text[action_match.get(0)?.end()..];
        let answer = remaining.trim();
        return Some(Action::Final {
            text: answer.to_string(),
        });
    }

    // Extract Input
    let input = react_input_re()
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim())
        .unwrap_or("");

    // Map action name to a tool call
    let (tool, arguments) = map_react_action(action_name, input);
    Some(Action::Tool(ToolCall { tool, arguments }))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn json_to_action(v: Value) -> Option<Action> {
    let obj = v.as_object()?;

    // Check for "tool" + "arguments"
    if let Some(tool) = obj.get("tool").and_then(|t| t.as_str()) {
        let arguments = obj
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        return Some(Action::Tool(ToolCall {
            tool: tool.to_string(),
            arguments,
        }));
    }

    // Check for "action" + "input" (ReAct JSON variant)
    if let Some(action) = obj.get("action").and_then(|a| a.as_str()) {
        let input = obj
            .get("input")
            .or_else(|| obj.get("arguments"))
            .cloned()
            .unwrap_or(serde_json::json!({}));

        if action.eq_ignore_ascii_case("final answer") {
            let text = input.as_str().unwrap_or(&input.to_string()).to_string();
            return Some(Action::Final { text });
        }

        let (tool, arguments) = map_react_action(action, input.as_str().unwrap_or(""));
        return Some(Action::Tool(ToolCall { tool, arguments }));
    }

    // Single "command" field shorthand
    if let Some(cmd) = obj.get("command").and_then(|c| c.as_str()) {
        return Some(Action::Tool(ToolCall {
            tool: "terminal.exec".into(),
            arguments: serde_json::json!({ "command": cmd }),
        }));
    }

    None
}

/// Map a ReAct-style action name to (tool, arguments).
fn map_react_action(action: &str, input: &str) -> (String, Value) {
    let action_lower = action.to_lowercase();
    match action_lower.as_str() {
        "terminal" | "terminal.exec" | "exec" | "run" | "bash" | "shell" => (
            "terminal.exec".into(),
            serde_json::json!({ "command": input }),
        ),
        "read" | "filesystem.read" | "file_read" | "read_file" => (
            "filesystem".into(),
            serde_json::json!({ "operation": "read", "path": input }),
        ),
        "write" | "filesystem.write" | "file_write" | "write_file" => (
            "filesystem".into(),
            serde_json::json!({ "operation": "write", "path": input }),
        ),
        "search" | "filesystem.search" | "grep" => (
            "filesystem".into(),
            serde_json::json!({ "operation": "search", "path": ".", "query": input }),
        ),
        "list" | "filesystem.list" | "ls" => (
            "filesystem".into(),
            serde_json::json!({ "operation": "list", "path": input }),
        ),
        "git" | "git.status" => (
            "git".into(),
            serde_json::json!({ "operation": "status" }),
        ),
        "git.diff" => ("git".into(), serde_json::json!({ "operation": "diff" })),
        _ => (
            "terminal.exec".into(),
            serde_json::json!({ "command": format!("{} {}", action, input) }),
        ),
    }
}

/// Extract all complete JSON objects from text using bracket matching.
fn extract_json_objects(text: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' {
            let start = i;
            let mut depth = 0i32;
            let mut in_string = false;
            let mut escape_next = false;
            let mut found_end = false;

            for (j, &b) in bytes[start..].iter().enumerate() {
                if escape_next {
                    escape_next = false;
                    continue;
                }
                match b {
                    b'\\' if in_string => escape_next = true,
                    b'"' => in_string = !in_string,
                    b'{' if !in_string => depth += 1,
                    b'}' if !in_string => {
                        depth -= 1;
                        if depth == 0 {
                            objects.push(text[start..=start + j].to_string());
                            i = start + j; // advance outer loop to end of object
                            found_end = true;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if !found_end {
                // unmatched brace, just skip the starting '{'
            }
        }
        i += 1;
    }
    
    objects
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_json() {
        let resp = r#"{"tool":"terminal.exec","arguments":{"command":"cargo test"}}"#;
        let action = parse_action(resp).unwrap();
        assert!(matches!(action, Action::Tool(ref c) if c.tool == "terminal.exec"));
    }

    #[test]
    fn test_repaired_json_with_prefix() {
        let resp = r#"Sure! I'll run the tests. {"tool":"terminal.exec","arguments":{"command":"cargo test"}} That should work."#;
        let action = parse_action(resp).unwrap();
        assert!(matches!(action, Action::Tool(ref c) if c.tool == "terminal.exec"));
    }

    #[test]
    fn test_tagged_command() {
        let resp = "Let me check: <command>ls -la</command>";
        let action = parse_action(resp).unwrap();
        assert!(matches!(action, Action::Tool(ref c) if c.tool == "terminal.exec"));
    }

    #[test]
    fn test_react_style() {
        let resp = "Thought: I need to run the tests.\nAction: terminal.exec\nInput: cargo test";
        let action = parse_action(resp).unwrap();
        assert!(matches!(action, Action::Tool(ref c) if c.tool == "terminal.exec"));
    }

    #[test]
    fn test_final_answer() {
        let resp = "FINAL ANSWER: The tests are passing. I fixed the bug by updating the auth middleware.";
        let action = parse_action(resp).unwrap();
        assert!(matches!(action, Action::Final { .. }));
    }

    #[test]
    fn test_extract_json_objects() {
        let text = "prefix {\"a\":1} suffix {\"b\": 2}";
        let objs = extract_json_objects(text);
        assert_eq!(objs.len(), 2);
        assert_eq!(objs[0], "{\"a\":1}");
        assert_eq!(objs[1], "{\"b\": 2}");
    }
}
