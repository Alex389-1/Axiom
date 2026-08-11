pub mod ollama;
pub mod llamacpp;

pub use ollama::OllamaProvider;
pub use llamacpp::LlamaCppProvider;

use async_trait::async_trait;
use axiom_core::{errors::Result, types::ModelInfo};
use futures::Stream;
use std::pin::Pin;

/// A single token chunk from a streaming response.
#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    pub done: bool,
}

/// Request to the model.
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub model: String,
    pub messages: Vec<ProviderMessage>,
    /// If Some, attempt constrained JSON output matching this schema.
    pub json_schema: Option<serde_json::Value>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// A single message in the conversation as the provider expects it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderMessage {
    pub role: String, // "system" | "user" | "assistant" | "tool"
    pub content: String,
}

/// Response from a non-streaming generation.
#[derive(Debug, Clone)]
pub struct GenerateResponse {
    pub text: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<Token>> + Send>>;

/// Provider trait — implemented by OllamaProvider, LlamaCppProvider.
#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse>;

    async fn stream(&self, req: GenerateRequest) -> Result<TokenStream>;

    /// Whether this provider supports constrained / schema-guided decoding.
    fn supports_constrained_decoding(&self) -> bool {
        false
    }

    /// Generate with schema-constrained output (JSON mode).
    /// Falls back to regular generate if not supported.
    async fn generate_constrained(
        &self,
        req: GenerateRequest,
    ) -> Result<GenerateResponse> {
        // Default: just regular generate — providers override this.
        self.generate(req).await
    }

    /// Health-check: returns Ok if the provider is reachable.
    async fn health_check(&self) -> Result<()>;
}
