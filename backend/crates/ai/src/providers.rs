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

/// Whether a response body is a web page rather than an API answer.
///
/// A base URL pointing at a website instead of its API is the most common way
/// to misconfigure a provider, and the reply is a page of HTML. Reporting that
/// as "invalid response" and quoting the markup helps nobody.
fn looks_like_html(body: &str) -> bool {
    let head = body.trim_start().get(..200).unwrap_or(body.trim_start());
    let head = head.to_ascii_lowercase();
    head.starts_with("<!doctype html") || head.starts_with("<html") || head.contains("<head")
}

/// The error for a body that is a web page, naming the likely cause.
fn html_response_error(status: u16) -> AiError {
    AiError::Provider {
        status,
        message: "the endpoint returned a web page instead of JSON. The base URL is                   probably pointing at a website rather than at the provider's API —                   for Ollama use the host it runs on with the /v1 path, such as                   http://localhost:11434/v1, not ollama.com"
            .to_owned(),
    }
}

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
    if looks_like_html(&text) {
        return Err(html_response_error(status.as_u16()));
    }
    if !status.is_success() {
        let message: String = text.chars().take(ERROR_BODY_CHARS).collect();
        let message = if message.trim().is_empty() {
            "the provider gave no explanation".to_owned()
        } else {
            message
        };
        return Err(AiError::Provider {
            status: status.as_u16(),
            message,
        });
    }
    serde_json::from_str(&text).map_err(|_| {
        // The body is not quoted: it may be long, and it may contain the
        // prompt echoed back.
        AiError::Transport(
            "the endpoint answered with something that is not JSON. Check that the base              URL points at the provider's API."
                .to_owned(),
        )
    })
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

/// Beta that enables server-side refusal fallbacks (`fallbacks: "default"`).
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// Whether `model` accepts server-side refusal fallbacks. Other models reject
/// the field, so it is sent only to these: Claude Opus 5 and 5.5, Claude
/// Sonnet 5.5, and the Fable and Mythos 5 families.
fn supports_server_fallback(model: &str) -> bool {
    [
        "claude-opus-5",
        "claude-sonnet-5-5",
        "claude-fable-5",
        "claude-mythos-5",
    ]
    .iter()
    .any(|p| model.starts_with(p))
}

impl AnthropicProvider {
    /// The Messages API request body, and whether it needs [`FALLBACK_BETA`].
    fn request_body(&self, req: &LlmRequest) -> (serde_json::Value, bool) {
        let mut body = json!({
            "model": self.model,
            "max_tokens": req.max_tokens,
            "system": req.system,
            "messages": [{"role": "user", "content": req.user}],
        });
        let fallback = supports_server_fallback(&self.model);
        if fallback {
            // Route a refusal to a fallback model server-side, chosen by the
            // refusal's category, instead of failing the job.
            body["fallbacks"] = json!("default");
        }
        (body, fallback)
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, req: &LlmRequest) -> Result<String, AiError> {
        let (body, fallback) = self.request_body(req);
        let mut builder = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01");
        if fallback {
            builder = builder.header("anthropic-beta", FALLBACK_BETA);
        }
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
#[cfg(test)]
mod tests {
    use super::*;

    fn anthropic(model: &str) -> AnthropicProvider {
        AnthropicProvider::new(reqwest::Client::new(), None, "k".into(), Some(model))
    }

    fn request() -> LlmRequest {
        LlmRequest {
            system: "s".into(),
            user: "u".into(),
            max_tokens: 100,
        }
    }

    #[test]
    fn asks_for_server_side_fallbacks_on_models_that_accept_them() {
        for model in [
            "claude-opus-5-5",
            "claude-opus-5",
            "claude-sonnet-5-5",
            "claude-fable-5-2",
        ] {
            let (body, beta) = anthropic(model).request_body(&request());
            assert!(beta, "{model}");
            assert_eq!(body["fallbacks"], "default", "{model}");
        }
    }

    #[test]
    fn leaves_fallbacks_out_for_other_models() {
        for model in [
            "claude-haiku-4-5",
            "claude-sonnet-5",
            "claude-opus-4-8",
            "claude-haiku-5-5",
        ] {
            let (body, beta) = anthropic(model).request_body(&request());
            assert!(!beta, "{model}");
            assert!(body.get("fallbacks").is_none(), "{model}");
        }
    }

    #[test]
    fn the_default_model_gets_fallbacks() {
        let p = AnthropicProvider::new(reqwest::Client::new(), None, "k".into(), None);
        assert!(p.request_body(&request()).1);
    }

    #[test]
    fn recognises_a_web_page() {
        // What a provider base URL pointing at a website returns.
        assert!(looks_like_html("<!doctype html>\n<html class=\"h-full\">"));
        assert!(looks_like_html("  <html><head><title>Ollama</title>"));
        assert!(looks_like_html("<!DOCTYPE HTML><head>"));
    }

    #[test]
    fn does_not_mistake_json_for_a_web_page() {
        assert!(!looks_like_html(
            r#"{"choices":[{"message":{"content":"<p>hi</p>"}}]}"#
        ));
        assert!(!looks_like_html("{\"error\":\"model not found\"}"));
        assert!(!looks_like_html(""));
    }

    #[test]
    fn the_html_error_says_what_to_fix_without_quoting_the_page() {
        let AiError::Provider { message, .. } = html_response_error(404) else {
            panic!("expected a provider error");
        };
        assert!(message.contains("/v1"));
        assert!(!message.contains('<'));
    }
}
