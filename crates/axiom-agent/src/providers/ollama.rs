use async_trait::async_trait;
use axiom_core::{
    errors::{AxiomError, Result},
    types::ModelInfo,
};
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::{debug, error};

use super::{GenerateRequest, GenerateResponse, ModelProvider, Token, TokenStream};

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut url: String = base_url.into().trim_end_matches('/').to_string();
        if url.contains("localhost") {
            url = url.replace("localhost", "127.0.0.1");
        }
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("HTTP client"),
            base_url: url,
        }
    }
}

// ─── Ollama API types ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OllamaTagsRequest;

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Deserialize)]
struct OllamaModelEntry {
    name: String,
    size: Option<u64>,
    modified_at: Option<String>,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize, Deserialize, Clone)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaChatChunk {
    message: Option<OllamaMessage>,
    done: bool,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp: OllamaTagsResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(format!("Ollama unreachable: {}", e)))?
            .json()
            .await
            .map_err(|e| AxiomError::Provider(format!("Ollama response error: {}", e)))?;

        Ok(resp
            .models
            .into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                provider: "ollama".into(),
                size_bytes: m.size,
                modified_at: m
                    .modified_at
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
            })
            .collect())
    }

    async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse> {
        let messages: Vec<OllamaMessage> = req
            .messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let body = OllamaChatRequest {
            model: req.model.clone(),
            messages,
            stream: false,
            format: None,
            options: Some(OllamaOptions {
                temperature: req.temperature,
                num_predict: req.max_tokens,
            }),
        };

        let url = format!("{}/api/chat", self.base_url);
        let chunk: OllamaChatChunk = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?;

        Ok(GenerateResponse {
            text: chunk.message.map(|m| m.content).unwrap_or_default(),
            input_tokens: chunk.prompt_eval_count,
            output_tokens: chunk.eval_count,
        })
    }

    async fn stream(&self, req: GenerateRequest) -> Result<TokenStream> {
        let messages: Vec<OllamaMessage> = req
            .messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let body = OllamaChatRequest {
            model: req.model.clone(),
            messages,
            stream: true,
            format: None,
            options: Some(OllamaOptions {
                temperature: req.temperature,
                num_predict: req.max_tokens,
            }),
        };

        let url = format!("{}/api/chat", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?;

        let byte_stream = resp.bytes_stream();

        struct State<S> {
            byte_stream: S,
            buffer: String,
            done: bool,
        }

        let state = State {
            byte_stream,
            buffer: String::new(),
            done: false,
        };

        let token_stream = futures::stream::unfold(state, |mut st| async move {
            if st.done {
                return None;
            }

            loop {
                if let Some(pos) = st.buffer.find('\n') {
                    let line = st.buffer[..pos].trim().to_string();
                    st.buffer.drain(..=pos);

                    if line.is_empty() {
                        continue;
                    }

                    if let Ok(c) = serde_json::from_str::<OllamaChatChunk>(&line) {
                        let is_done = c.done;
                        if is_done {
                            st.done = true;
                        }
                        let tok = Token {
                            text: c.message.map(|m| m.content).unwrap_or_default(),
                            done: is_done,
                        };
                        return Some((Ok(tok), st));
                    }
                }

                match st.byte_stream.next().await {
                    Some(Ok(chunk)) => {
                        st.buffer.push_str(&String::from_utf8_lossy(&chunk));
                    }
                    Some(Err(e)) => {
                        st.done = true;
                        return Some((Err(AxiomError::Streaming(e.to_string())), st));
                    }
                    None => {
                        st.done = true;
                        let line = st.buffer.trim().to_string();
                        st.buffer.clear();
                        if !line.is_empty() {
                            if let Ok(c) = serde_json::from_str::<OllamaChatChunk>(&line) {
                                let tok = Token {
                                    text: c.message.map(|m| m.content).unwrap_or_default(),
                                    done: c.done,
                                };
                                return Some((Ok(tok), st));
                            }
                        }
                        return None;
                    }
                }
            }
        });

        Ok(Box::pin(token_stream))
    }

    fn supports_constrained_decoding(&self) -> bool {
        true // Ollama supports `format: json` and JSON schema mode
    }

    async fn generate_constrained(&self, req: GenerateRequest) -> Result<GenerateResponse> {
        let messages: Vec<OllamaMessage> = req
            .messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        // Use Ollama's JSON schema format constraint
        let format = req.json_schema.clone().unwrap_or(serde_json::json!("json"));

        let body = OllamaChatRequest {
            model: req.model.clone(),
            messages,
            stream: false,
            format: Some(format),
            options: Some(OllamaOptions {
                temperature: req.temperature,
                num_predict: req.max_tokens,
            }),
        };

        let url = format!("{}/api/chat", self.base_url);
        let chunk: OllamaChatChunk = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| AxiomError::Provider(e.to_string()))?;

        Ok(GenerateResponse {
            text: chunk.message.map(|m| m.content).unwrap_or_default(),
            input_tokens: chunk.prompt_eval_count,
            output_tokens: chunk.eval_count,
        })
    }

    async fn health_check(&self) -> Result<()> {
        let url = format!("{}/api/tags", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| AxiomError::Provider(format!("Ollama health check failed: {}", e)))?;
        Ok(())
    }
}
