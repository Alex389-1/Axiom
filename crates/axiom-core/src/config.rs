use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level application configuration, loaded from `~/.config/axiom/config.toml`
/// and optionally overridden by per-project `.local-agent/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub agent: AgentConfig,
    pub model: ModelConfig,
    pub terminal: TerminalConfig,
    pub tools: ToolsConfig,
    pub security: SecurityConfig,
    pub permissions: PermissionsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Hard cap on agent loop iterations per turn.
    pub max_steps: u32,
    /// Maximum tokens to include in model context (prompt + history + retrieved files).
    pub max_context_tokens: usize,
    /// Controls when the permission dialog appears.
    pub confirmation_mode: ConfirmationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationMode {
    /// Always ask before any tool call.
    Always,
    /// Ask only for dangerous tool calls (default).
    Dangerous,
    /// Never ask (trust all — not recommended).
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelConfig {
    pub provider: ProviderKind,
    pub model: String,
    /// Ollama base URL
    pub ollama_url: String,
    /// llama.cpp / OpenAI-compatible base URL
    pub llamacpp_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Ollama,
    LlamaCpp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Which shell to spawn. Empty = auto-detect from $SHELL.
    pub shell: String,
    /// Keep PTY alive after GUI close.
    pub persistent: bool,
    /// Number of scrollback lines to preserve.
    pub scrollback_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolsConfig {
    pub filesystem: bool,
    pub terminal: bool,
    pub git: bool,
    pub process: bool,
    pub browser: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub allow_network: bool,
    pub allow_sudo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PermissionsConfig {
    /// Default scope for new grants: prompt | session | project
    pub session_scope_default: String,
}

// ─── Defaults ────────────────────────────────────────────────────────────────

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            model: ModelConfig::default(),
            terminal: TerminalConfig::default(),
            tools: ToolsConfig::default(),
            security: SecurityConfig::default(),
            permissions: PermissionsConfig::default(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 20,
            max_context_tokens: 16_000,
            confirmation_mode: ConfirmationMode::Dangerous,
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Ollama,
            model: "qwen2.5-coder:14b".into(),
            ollama_url: "http://127.0.0.1:11434".into(),
            llamacpp_url: "http://127.0.0.1:18881".into(),
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: String::new(), // empty → auto-detect
            persistent: true,
            scrollback_lines: 10_000,
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            filesystem: true,
            terminal: true,
            git: true,
            process: true,
            browser: false,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_sudo: false,
        }
    }
}

impl Default for PermissionsConfig {
    fn default() -> Self {
        Self {
            session_scope_default: "prompt".into(),
        }
    }
}

// ─── Load / save ─────────────────────────────────────────────────────────────

impl AppConfig {
    /// Returns the global config file path: `~/.config/axiom/config.toml`
    pub fn global_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "axiom", "axiom")
            .map(|pd| pd.config_dir().join("config.toml"))
    }

    /// Returns the per-project config path: `<project>/.local-agent/config.toml`
    pub fn project_path(project_root: &std::path::Path) -> PathBuf {
        project_root.join(".local-agent").join("config.toml")
    }

    /// Load config, merging global → project override.
    pub fn load(project_root: Option<&std::path::Path>) -> Self {
        let mut cfg = Self::load_file(Self::global_path().as_deref()).unwrap_or_default();
        if let Some(root) = project_root {
            let project_path = Self::project_path(root);
            if let Some(override_cfg) = Self::load_file(Some(&project_path)) {
                cfg.merge(override_cfg);
            }
        }
        cfg
    }

    fn load_file(path: Option<&std::path::Path>) -> Option<Self> {
        let path = path?;
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    fn merge(&mut self, other: Self) {
        // Simple field-level merge: non-default values from `other` override `self`.
        // For MVP this is a shallow merge — project config wins on any set field.
        *self = other;
    }

    pub fn save_global(&self) -> anyhow::Result<()> {
        let path = Self::global_path().ok_or_else(|| anyhow::anyhow!("No config dir"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
