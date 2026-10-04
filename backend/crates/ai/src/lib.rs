//! LLM provider trait, prompt templates and proposal validation. See docs/specification/07-ai-integration.md.
//!
//! LLM output is parsed into domain types and validated; invalid output is rejected, never
//! repaired silently (one bounded retry with the validation error).

mod parse;
mod prompts;
mod providers;

pub use parse::{parse_proposals, ProposalContent, ProposalDraft};
pub use prompts::{render_prompt, PromptRequest, ProposalContext, PROMPT_VERSION};
pub use providers::{AnthropicProvider, LlmProvider, LlmRequest, OpenAiCompatibleProvider};

use tm_domain::ProposalKind;

/// Errors from the AI integration.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// HTTP or transport failure.
    #[error("provider request failed: {0}")]
    Transport(String),
    /// Provider returned an error status.
    #[error("provider returned status {status}: {message}")]
    Provider {
        /// HTTP status.
        status: u16,
        /// Provider message (truncated).
        message: String,
    },
    /// Provider declined to answer.
    #[error("provider refused the request")]
    Refused,
    /// Output was not valid JSON of the expected shape or violated domain rules.
    #[error("invalid model output: {0}")]
    InvalidOutput(String),
}

/// Calls the provider and parses the result, retrying once with the validation error (07-ai-integration).
pub async fn generate_proposals(
    provider: &dyn LlmProvider,
    kind: ProposalKind,
    request: &PromptRequest,
    max_tokens: u32,
) -> Result<Vec<ProposalDraft>, AiError> {
    let (system, user) = render_prompt(kind, request);
    let mut llm = LlmRequest {
        system,
        user,
        max_tokens,
    };
    let first = provider.complete(&llm).await?;
    match parse_proposals(kind, &first, &request.context) {
        Ok(drafts) => Ok(drafts),
        Err(AiError::InvalidOutput(reason)) => {
            llm.user.push_str(&format!(
                "\n\nYour previous answer was rejected: {reason}\nReturn corrected JSON only."
            ));
            let second = provider.complete(&llm).await?;
            parse_proposals(kind, &second, &request.context)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeProvider {
        answers: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl LlmProvider for FakeProvider {
        async fn complete(&self, _req: &LlmRequest) -> Result<String, AiError> {
            self.answers
                .lock()
                .map_err(|_| AiError::Transport("poisoned".into()))?
                .pop()
                .ok_or_else(|| AiError::Transport("no answer".into()))
        }

        fn model_id(&self) -> String {
            "fake".into()
        }
    }

    fn request() -> PromptRequest {
        PromptRequest {
            context: ProposalContext {
                feature_name: "Login".into(),
                feature_description: None,
                scenario_description: Some("User logs in".into()),
                model: None,
                existing_test_cases: vec![],
            },
            prompt: None,
            count: 1,
        }
    }

    #[tokio::test]
    async fn retries_once_on_invalid_output() {
        let good = r#"{"proposals":[{"rationale":"r","scenarioDescription":"text"}]}"#;
        let provider = FakeProvider {
            answers: Mutex::new(vec![good.into(), "not json".into()]),
        };
        let drafts =
            generate_proposals(&provider, ProposalKind::FeatureDescription, &request(), 100)
                .await
                .unwrap();
        assert_eq!(drafts.len(), 1);
    }

    #[tokio::test]
    async fn fails_after_second_invalid_output() {
        let provider = FakeProvider {
            answers: Mutex::new(vec!["{}".into(), "nope".into()]),
        };
        let err = generate_proposals(&provider, ProposalKind::FeatureDescription, &request(), 100)
            .await
            .unwrap_err();
        assert!(matches!(err, AiError::InvalidOutput(_)));
    }
}
