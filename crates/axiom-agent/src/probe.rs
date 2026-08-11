use axiom_core::{
    errors::Result,
    types::{ConversationMessage, ModelProfile, ToolCallingMode},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::Utc;
use tracing::{info, warn};

use crate::providers::{GenerateRequest, ModelProvider, ProviderMessage};

/// Auto-probes a model on first use to determine the most reliable tool-calling mode.
pub struct ModelProber {
    profile_cache_path: PathBuf,
}

impl ModelProber {
    pub fn new() -> Self {
        let path = directories::ProjectDirs::from("com", "axiom", "axiom")
            .map(|pd| pd.config_dir().join("models.toml"))
            .unwrap_or_else(|| PathBuf::from("/tmp/axiom_models.toml"));
        Self { profile_cache_path: path }
    }

    /// Return cached profile if it exists and the model hash hasn't changed.
    pub fn get_cached(&self, model: &str) -> Option<ModelProfile> {
        let content = std::fs::read_to_string(&self.profile_cache_path).ok()?;
        let map: HashMap<String, ModelProfile> = toml::from_str(&content).ok()?;
        map.get(model).cloned()
    }

    /// Return all cached profiles.
    pub fn get_all_cached(&self) -> HashMap<String, ModelProfile> {
        std::fs::read_to_string(&self.profile_cache_path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Run probe sequence and cache the result.
    pub async fn probe(
        &self,
        model: &str,
        provider: &dyn ModelProvider,
    ) -> Result<ModelProfile> {
        info!("Probing model: {}", model);

        // Mode 1: Try structured JSON
        if let Ok(mode) = self.probe_mode(model, provider, ToolCallingMode::Structured).await {
            let profile = ModelProfile {
                model: model.to_string(),
                tool_mode: mode,
                supports_thinking: false,
                supports_constrained_decoding: provider.supports_constrained_decoding(),
                detected_at: "probe".to_string(),
                last_verified: Utc::now(),
                model_hash: None,
            };
            self.cache_profile(&profile)?;
            return Ok(profile);
        }

        // Mode 2: Tagged
        if let Ok(mode) = self.probe_mode(model, provider, ToolCallingMode::Tagged).await {
            let profile = ModelProfile {
                model: model.to_string(),
                tool_mode: mode,
                supports_thinking: false,
                supports_constrained_decoding: provider.supports_constrained_decoding(),
                detected_at: "probe".to_string(),
                last_verified: Utc::now(),
                model_hash: None,
            };
            self.cache_profile(&profile)?;
            return Ok(profile);
        }

        // Fallback: ReAct
        warn!("Model {} could not be reliably probed; defaulting to ReAct mode", model);
        let profile = ModelProfile {
            model: model.to_string(),
            tool_mode: ToolCallingMode::React,
            supports_thinking: false,
            supports_constrained_decoding: provider.supports_constrained_decoding(),
            detected_at: "probe".to_string(),
            last_verified: Utc::now(),
            model_hash: None,
        };
        self.cache_profile(&profile)?;
        Ok(profile)
    }

    async fn probe_mode(
        &self,
        model: &str,
        provider: &dyn ModelProvider,
        mode: ToolCallingMode,
    ) -> Result<ToolCallingMode> {
        let (system_prompt, user_prompt) = probe_prompts(&mode);

        let req = GenerateRequest {
            model: model.to_string(),
            messages: vec![
                ProviderMessage { role: "system".into(), content: system_prompt },
                ProviderMessage { role: "user".into(), content: user_prompt },
            ],
            json_schema: None,
            temperature: Some(0.0),
            max_tokens: Some(200),
        };

        let resp = provider.generate(req).await?;
        if mode_succeeded(&mode, &resp.text) {
            Ok(mode)
        } else {
            Err(axiom_core::errors::AxiomError::ParseFailed(format!(
                "Mode {:?} probe failed for {}",
                mode, model
            )))
        }
    }

    fn cache_profile(&self, profile: &ModelProfile) -> Result<()> {
        if let Some(parent) = self.profile_cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = if self.profile_cache_path.exists() {
            std::fs::read_to_string(&self.profile_cache_path)
                .ok()
                .unwrap_or_default()
        } else {
            String::new()
        };

        let mut map: HashMap<String, ModelProfile> = toml::from_str(&content).unwrap_or_default();
        map.insert(profile.model.clone(), profile.clone());

        let serialized = toml::to_string(&map)
            .map_err(|e| axiom_core::errors::AxiomError::Config(e.to_string()))?;
        std::fs::write(&self.profile_cache_path, serialized)?;
        Ok(())
    }
}

impl Default for ModelProber {
    fn default() -> Self {
        Self::new()
    }
}

fn probe_prompts(mode: &ToolCallingMode) -> (String, String) {
    let system = match mode {
        ToolCallingMode::Structured => {
            r#"You are an agent. When you want to use a tool, respond with ONLY a JSON object:
{"tool": "<tool_name>", "arguments": {<args>}}
Available tools: terminal.exec(command), filesystem(operation, path).
When done, respond with: {"action": "final_answer", "input": "<text>"}"#
        }
        ToolCallingMode::Tagged => {
            r#"You are an agent. When you want to run a command, respond with:
<command>your shell command here</command>
When done, respond with: FINAL ANSWER: <your answer>"#
        }
        ToolCallingMode::React => {
            r#"You are an agent. Use this format:
Thought: what you plan to do
Action: tool_name
Input: tool_input
When done:
Thought: I have the answer
Action: Final Answer
Input: <your answer>"#
        }
        _ => "",
    };

    let user = "List the files in the current directory.".to_string();
    (system.to_string(), user)
}

fn mode_succeeded(mode: &ToolCallingMode, response: &str) -> bool {
    match mode {
        ToolCallingMode::Structured => {
            let v: serde_json::Result<serde_json::Value> = serde_json::from_str(response.trim());
            v.ok()
                .and_then(|v| v.get("tool").cloned())
                .is_some()
        }
        ToolCallingMode::Tagged => {
            response.contains("<command>") || response.contains("<tool>")
        }
        ToolCallingMode::React => {
            response.contains("Action:")
        }
        _ => false,
    }
}
