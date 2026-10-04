//! Provider adapters. Prompts and keys are never logged.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::AiError;

/// A single completion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmRequest {
    /// System prompt.
    pub system: String,
    /// User message.
    pub user: String,
    /// Output token budget.
    pub max_tokens: u32,
}

/// Provider-agnostic LLM interface.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Returns the model's text answer.
    async fn complete(&self, req: &LlmRequest) -> Result<String, AiError>;
    /// Model id recorded as proposal `source`.
    fn model_id(&self) -> String;
}

const ERROR_BODY_CHARS: usize = 300;

async fn post_json(builder: reqwest::RequestBuilder, body: &Value) -> Result<Value, AiError> {
    let resp = builder
        .json(body)
        .send()
        .await
        .map_err(|e| AiError::Transport(e.without_url().to_string()))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| AiError::Transport(e.without_url().to_string()))?;
    if !status.is_success() {
        return Err(AiError::Provider {
            status: status.as_u16(),
            message: text.chars().take(ERROR_BODY_CHARS).collect(),
        });
    }
    serde_json::from_str(&text).map_err(|e| AiError::Transport(format!("invalid response: {e}")))
}

/// Anthropic Messages API adapter.
pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    /// Default model when none is configured.
    pub const DEFAULT_MODEL: &'static str = "claude-opus-5-5";
    /// Default API base URL.
    pub const DEFAULT_BASE_URL: &'static str = "https://api.anthropic.com";

    /// Creates the adapter.
    pub fn new(
        client: reqwest::Client,
        base_url: Option<&str>,
        api_key: String,
        model: Option<&str>,
    ) -> Self {
        Self {
            client,
            base_url: base_url
                .unwrap_or(Self::DEFAULT_BASE_URL)
                .trim_end_matches('/')
                .to_owned(),
            api_key,
            model: model.unwrap_or(Self::DEFAULT_MODEL).to_owned(),
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, req: &LlmRequest) -> Result<String, AiError> {
        let body = json!({
            "model": self.model,
            "max_tokens": req.max_tokens,
            "system": req.system,
            "messages": [{"role": "user", "content": req.user}],
            // Route refusals to a fallback model server-side instead of failing the job.
            "fallbacks": "default",
        });
        let builder = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "server-side-fallback-2026-07-01");
        let resp = post_json(builder, &body).await?;
        if resp["stop_reason"] == "refusal" {
            return Err(AiError::Refused);
        }
        let text: String = resp["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect();
        if text.is_empty() {
            return Err(AiError::InvalidOutput("empty response".into()));
        }
        Ok(text)
    }

    fn model_id(&self) -> String {
        self.model.clone()
    }
}

/// OpenAI-compatible chat completions adapter (also used for local servers).
pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl OpenAiCompatibleProvider {
    /// Creates the adapter. `base_url` should include the version path, e.g. `http://localhost:11434/v1`.
    pub fn new(
        client: reqwest::Client,
        base_url: &str,
        api_key: Option<String>,
        model: &str,
    ) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
            model: model.to_owned(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn complete(&self, req: &LlmRequest) -> Result<String, AiError> {
        let body = json!({
            "model": self.model,
            "max_tokens": req.max_tokens,
            "response_format": {"type": "json_object"},
            "messages": [
                {"role": "system", "content": req.system},
                {"role": "user", "content": req.user},
            ],
        });
        let mut builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url));
        if let Some(key) = &self.api_key {
            builder = builder.bearer_auth(key);
        }
        let resp = post_json(builder, &body).await?;
        resp["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| AiError::InvalidOutput("missing message content".into()))
    }

    fn model_id(&self) -> String {
        self.model.clone()
    }
}
