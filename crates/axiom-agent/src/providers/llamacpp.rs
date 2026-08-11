use async_trait::async_trait;
use axiom_core::{
    errors::{AxiomError, Result},
    types::ModelInfo,
};
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use super::{GenerateRequest, GenerateResponse, ModelProvider, Token, TokenStream};

/// OpenAI-compatible provider for llama.cpp and LM Studio.
pub struct LlamaCppProvider {
    client: Client,
    base_url: String,
}

impl LlamaCppProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("HTTP client"),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: Option<ChatMessage>,
    delta: Option<Delta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelData>,
}

#[derive(Deserialize)]
struct ModelData {
    id: String,
}

#[async_trait]
impl ModelProvider for LlamaCppProvider {
    fn name(&self) -> &str {
        "llamacpp"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/v1/models", self.base_url);
        let resp: ModelsResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(format!("llama.cpp unreachable: {}", e)))?
            .json()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?;

        Ok(resp
            .data
            .into_iter()
            .map(|m| ModelInfo {
                name: m.id,
                provider: "llamacpp".into(),
                size_bytes: None,
                modified_at: None,
            })
            .collect())
    }

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse> {
        let messages: Vec<ChatMessage> = req
            .messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let body = ChatRequest {
            model: req.model.clone(),
            messages,
            stream: false,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            response_format: None,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp: ChatResponse = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?;

        let text = resp
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .map(|m| m.content)
            .unwrap_or_default();

        Ok(GenerateResponse {
            text,
            input_tokens: resp.usage.as_ref().and_then(|u| u.prompt_tokens),
            output_tokens: resp.usage.as_ref().and_then(|u| u.completion_tokens),
        })
    }

    async fn stream(&self, req: GenerateRequest) -> Result<TokenStream> {
        let messages: Vec<ChatMessage> = req
            .messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let body = ChatRequest {
            model: req.model.clone(),
            messages,
            stream: true,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            response_format: None,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?;

        let byte_stream = resp.bytes_stream();
        let token_stream = byte_stream.flat_map(|chunk| {
            let chunk = match chunk {
                Ok(b) => b,
                Err(e) => {
                    return futures::stream::once(futures::future::ready(Err(
                        AxiomError::Streaming(e.to_string()),
                    )))
                    .boxed();
                }
            };

            let tokens: Vec<Result<Token>> = String::from_utf8_lossy(&chunk)
                .lines()
                .filter_map(|line| {
                    let line = line.trim().strip_prefix("data: ")?;
                    if line == "[DONE]" {
                        return Some(Ok(Token {
                            text: String::new(),
                            done: true,
                        }));
                    }
                    let resp: ChatResponse = serde_json::from_str(line).ok()?;
                    let text = resp
                        .choices
                        .into_iter()
                        .next()
                        .and_then(|c| c.delta)
                        .and_then(|d| d.content)
                        .unwrap_or_default();
                    Some(Ok(Token { text, done: false }))
                })
                .collect();
            futures::stream::iter(tokens).boxed()
        });

        Ok(Box::pin(token_stream))
    }

    fn supports_constrained_decoding(&self) -> bool {
        true // OpenAI-compat supports response_format: json_object
    }

    async fn generate_constrained(&self, req: GenerateRequest) -> Result<GenerateResponse> {
        let messages: Vec<ChatMessage> = req
            .messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let body = ChatRequest {
            model: req.model.clone(),
            messages,
            stream: false,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            response_format: Some(ResponseFormat {
                kind: "json_object".into(),
            }),
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp: ChatResponse = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?;

        let text = resp
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .map(|m| m.content)
            .unwrap_or_default();

        Ok(GenerateResponse {
            text,
            input_tokens: resp.usage.as_ref().and_then(|u| u.prompt_tokens),
            output_tokens: resp.usage.as_ref().and_then(|u| u.completion_tokens),
        })
    }

    async fn health_check(&self) -> Result<()> {
        let url = format!("{}/v1/models", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(format!("llama.cpp health check failed: {}", e)))?;
        Ok(())
    }
}
