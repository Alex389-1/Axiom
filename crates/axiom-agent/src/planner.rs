use axiom_core::{
    config::AppConfig,
    errors::{AxiomError, Result},
    types::{
        Action, ConversationMessage, DaemonEvent, ModelProfile, PermissionCategory,
        PermissionScope, StepStatus, ToolCall, ToolCallingMode,
    },
};
use axiom_tools::ToolRegistry;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error};

use crate::{
    context::ContextManager,
    parser::{build_retry_prompt, parse_action, parse_constrained},
    permissions::{PermissionManager, PermissionStatus},
    providers::{GenerateRequest, ModelProvider, ProviderMessage},
};

/// The agent's pending permission request — sent to the UI for user response.
#[derive(Debug, Clone)]
pub struct PendingPermission {
    pub call: ToolCall,
    pub category: PermissionCategory,
    pub is_high_risk: bool,
}

/// Callbacks / events sent from the planner to the daemon session layer.
pub enum PlannerEvent {
    Token(String),
    ToolCallStarted { call: ToolCall, step: u32 },
    ToolCallCompleted { result: axiom_core::types::ToolResult, step: u32 },
    TurnCompleted(ConversationMessage),
    PermissionRequired(PendingPermission),
    AgentStep { step: u32, description: String, status: StepStatus },
    Error(String),
}

/// The deterministic agent loop.
/// One `AgentPlanner` is created per session.
pub struct AgentPlanner {
    config: AppConfig,
    provider: Arc<dyn ModelProvider>,
    tool_registry: Arc<ToolRegistry>,
    context_manager: ContextManager,
    permission_manager: PermissionManager,
    profile: ModelProfile,
    conversation: Vec<ConversationMessage>,
    event_tx: mpsc::Sender<PlannerEvent>,
    /// Channel for receiving permission responses from the UI.
    permission_rx: Option<mpsc::Receiver<PermissionResponse>>,
    permission_tx: mpsc::Sender<PermissionResponse>,
}

/// The user's response to a permission dialog.
#[derive(Debug, Clone)]
pub struct PermissionResponse {
    pub scope: PermissionScope,
}

impl AgentPlanner {
    pub fn new(
        config: AppConfig,
        provider: Arc<dyn ModelProvider>,
        tool_registry: Arc<ToolRegistry>,
        profile: ModelProfile,
        project_root: Option<std::path::PathBuf>,
        event_tx: mpsc::Sender<PlannerEvent>,
    ) -> Self {
        let (perm_tx, perm_rx) = mpsc::channel(1);
        let context_manager = ContextManager::new(
            project_root.clone(),
            config.agent.max_context_tokens,
        );
        let permission_manager = PermissionManager::new(project_root);

        Self {
            config,
            provider,
            tool_registry,
            context_manager,
            permission_manager,
            profile,
            conversation: Vec::new(),
            event_tx,
            permission_rx: Some(perm_rx),
            permission_tx: perm_tx,
        }
    }

    /// Returns a sender that the UI layer uses to respond to permission dialogs.
    pub fn permission_sender(&self) -> mpsc::Sender<PermissionResponse> {
        self.permission_tx.clone()
    }

    /// Record a permission grant from the UI.
    pub fn apply_permission_grant(
        &self,
        category: PermissionCategory,
        scope: PermissionScope,
    ) -> Result<()> {
        self.permission_manager.grant(category, scope)
    }

    pub fn conversation(&self) -> &[ConversationMessage] {
        &self.conversation
    }

    /// Pops the last turn (the last user message and all subsequent messages) from the conversation history.
    pub fn pop_last_turn(&mut self) -> Vec<ConversationMessage> {
        let mut popped = Vec::new();
        while let Some(msg) = self.conversation.pop() {
            popped.push(msg.clone());
            if msg.role == axiom_core::types::Role::User {
                break;
            }
        }
        popped
    }

    /// Process a user message. Runs the agent loop until final answer or MAX_STEPS.
    pub async fn process_message(&mut self, user_content: &str) -> Result<String> {
        let result = self.run_agent_loop(user_content).await;
        if let Err(ref e) = result {
            let _ = self.event_tx.send(PlannerEvent::Error(e.to_string())).await;
        }
        result
    }

    async fn run_agent_loop(&mut self, user_content: &str) -> Result<String> {
        // Add user message to conversation
        let user_msg = ConversationMessage::user(user_content);
        self.conversation.push(user_msg);

        let tool_specs = serde_json::to_string_pretty(&self.tool_registry.specs()).unwrap_or_default();
        let system_prompt = self.context_manager.build_system_prompt(&tool_specs);

        let mut messages = vec![ProviderMessage {
            role: "system".into(),
            content: system_prompt,
        }];

        // Add conversation history
        for msg in &self.conversation {
            let role = match msg.role {
                axiom_core::types::Role::User => "user",
                axiom_core::types::Role::Assistant => "assistant",
                axiom_core::types::Role::Tool => "user",
                axiom_core::types::Role::System => "system",
            };
            messages.push(ProviderMessage {
                role: role.into(),
                content: msg.content.clone(),
            });
        }

        let mut final_response = String::new();
        let mut step = 1;
        let max_steps = 10;

        while step <= max_steps {
            let mut current_messages = messages.clone();
            
            // Append a reminder at the end of the context window
            current_messages.push(ProviderMessage {
                role: "system".into(),
                content: "IMPORTANT: For greetings, general questions, or chat, respond directly in clear natural markdown. Only output a JSON tool call if the user requested an action on the local machine (such as executing commands or accessing files).".into(),
            });

            // Generate response directly (streaming for UI feedback)
            let req = GenerateRequest {
                model: self.profile.model.clone(),
                messages: current_messages,
                json_schema: None,
                temperature: Some(0.7),
                max_tokens: Some(4096),
            };

            let raw_response = self.generate_with_events(req).await?;

            // Try parsing action
            let action = match self.parse_with_retry(&raw_response, &messages).await {
                Ok(a) => a,
                Err(e) => {
                    warn!("Failed to parse action: {}", e);
                    Action::Final { text: raw_response.clone() }
                }
            };

            match action {
                Action::Final { text } => {
                    let assistant_msg = ConversationMessage::assistant(&text);
                    self.conversation.push(assistant_msg.clone());
                    let _ = self.event_tx.send(PlannerEvent::TurnCompleted(assistant_msg)).await;
                    final_response = text;
                    break;
                }
                Action::Tool(call) => {
                    // Send ToolCallStarted event
                    let _ = self.event_tx.send(PlannerEvent::ToolCallStarted {
                        call: call.clone(),
                        step,
                    }).await;

                    // Append assistant's intention to context
                    messages.push(ProviderMessage {
                        role: "assistant".into(),
                        content: raw_response.clone(),
                    });
                    
                    let assistant_msg = ConversationMessage::assistant(&raw_response);
                    self.conversation.push(assistant_msg);

                    // Execute Tool
                    let result = self.tool_registry.execute(&call).await?;
                    
                    // Send ToolCallCompleted event
                    let _ = self.event_tx.send(PlannerEvent::ToolCallCompleted {
                        result: result.clone(),
                        step,
                    }).await;

                    // Append tool result to context
                    let result_str = result.output.to_context_string();
                    messages.push(ProviderMessage {
                        role: "user".into(),
                        content: format!("Tool result for {}:\n{}", call.tool, result_str),
                    });
                    
                    let tool_msg = ConversationMessage::tool_result(call.clone(), result.clone());
                    self.conversation.push(tool_msg);

                    step += 1;
                }
            }
        }

        if step > max_steps {
            let warn_msg = "Agent reached maximum steps limit.".to_string();
            let assistant_msg = ConversationMessage::assistant(&warn_msg);
            self.conversation.push(assistant_msg.clone());
            let _ = self.event_tx.send(PlannerEvent::TurnCompleted(assistant_msg)).await;
            return Ok(warn_msg);
        }

        Ok(final_response)
    }

    /// Generate with streaming token events.
    async fn generate_with_events(&self, req: GenerateRequest) -> Result<String> {
        // Stream tokens for live UI feedback
        let mut stream = match self.provider.stream(req.clone()).await {
            Ok(s) => s,
            Err(e) => {
                warn!("Streaming failed ({}); falling back to standard generate", e);
                let resp = self.provider.generate(req).await?;
                let _ = self.event_tx.send(PlannerEvent::Token(resp.text.clone())).await;
                return Ok(resp.text);
            }
        };

        let mut full_text = String::new();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(token) => {
                    if !token.text.is_empty() {
                        full_text.push_str(&token.text);
                        let _ = self.event_tx.send(PlannerEvent::Token(token.text)).await;
                    }
                    if token.done {
                        break;
                    }
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    break;
                }
            }
        }

        if full_text.trim().is_empty() {
            warn!("Stream yielded empty output; falling back to standard generate");
            let resp = self.provider.generate(req).await?;
            let _ = self.event_tx.send(PlannerEvent::Token(resp.text.clone())).await;
            return Ok(resp.text);
        }

        Ok(full_text)
    }

    /// Parse action, with one repair retry if the first parse fails.
    async fn parse_with_retry(
        &self,
        response: &str,
        messages: &[ProviderMessage],
    ) -> Result<Action> {
        // First attempt
        let action = parse_action(response)?;

        // If we got a Final action but the response looks like it should be a tool call, retry.
        if let Action::Final { ref text } = action {
            if text.trim().starts_with('{') {
                // Might be malformed JSON — retry with explicit correction prompt
                return self.retry_parse(response, messages).await;
            }
        }

        Ok(action)
    }

    async fn retry_parse(
        &self,
        bad_response: &str,
        base_messages: &[ProviderMessage],
    ) -> Result<Action> {
        let schema_hint = r#"{"tool": "<tool_name>", "arguments": {<args>}}"#;
        let retry_prompt = build_retry_prompt(bad_response, schema_hint);

        let mut retry_messages = base_messages.to_vec();
        retry_messages.push(ProviderMessage {
            role: "assistant".into(),
            content: bad_response.to_string(),
        });
        retry_messages.push(ProviderMessage {
            role: "user".into(),
            content: retry_prompt,
        });

        let req = GenerateRequest {
            model: self.profile.model.clone(),
            messages: retry_messages,
            json_schema: Some(tool_call_schema()),
            temperature: Some(0.0),
            max_tokens: Some(256),
        };

        let resp = self.provider.generate(req).await?;
        parse_action(&resp.text)
    }

    fn tool_category(&self, call: &ToolCall) -> PermissionCategory {
        match call.tool.as_str() {
            "filesystem" => {
                let op = call
                    .arguments
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("read");
                match op {
                    "write" => PermissionCategory::Write,
                    _ => PermissionCategory::Read,
                }
            }
            "terminal.exec" => PermissionCategory::Execute,
            "git" => PermissionCategory::Git,
            "process" => PermissionCategory::Process,
            _ => PermissionCategory::Execute,
        }
    }
}

fn summarize_args(args: &serde_json::Value) -> String {
    match args {
        serde_json::Value::Object(map) => {
            // Show the first string value as a brief summary
            map.values()
                .filter_map(|v| v.as_str())
                .next()
                .map(|s| {
                    let s = s.trim();
                    if s.len() > 60 {
                        format!("{}...", &s[..60])
                    } else {
                        s.to_string()
                    }
                })
                .unwrap_or_default()
        }
        _ => String::new(),
    }
}

/// JSON schema for the tool call response format used in constrained decoding.
fn tool_call_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["tool", "arguments"],
        "properties": {
            "tool": {
                "type": "string",
                "description": "The tool name to call"
            },
            "arguments": {
                "type": "object",
                "description": "Arguments for the tool"
            }
        },
        "additionalProperties": false
    })
}
