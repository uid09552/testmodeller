//! Orchestrates AI proposal jobs (FR-030 to FR-034, NFR-003).
//!
//! The API key comes from `TM_AI_API_KEY` or `PUT /settings/ai` and is held in memory only.
//! Prompt and response content are never logged.

use std::sync::Arc;

use axum::http::StatusCode;
use chrono::Utc;
use tm_ai::{
    AnthropicProvider, LlmProvider, OpenAiCompatibleProvider, PromptRequest, ProposalContent,
    ProposalContext, PROMPT_VERSION,
};
use tm_domain::{AiProvider, AiSettings, Job, JobStatus, Proposal, ProposalKind, ProposalStatus};
use tm_storage::{Acceptance, Store, TestCaseFilter};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::dto::{ElementsPayload, FeatureDescriptionPayload, ModelInput, TestCaseInput};
use crate::error::{ApiError, ApiResult};
use crate::secrets::SecretStore;

/// Default output token budget when none is configured.
const DEFAULT_MAX_TOKENS: u32 = 16_000;
/// Default number of proposals.
const DEFAULT_COUNT: u32 = 3;
/// Upper bound on existing test case names sent as context.
const MAX_CONTEXT_TEST_CASES: i64 = 100;

/// Shared AI state.
pub struct AiService {
    store: Store,
    http: reqwest::Client,
    /// Working copy of the provider key, so a request does not decrypt.
    api_key: RwLock<Option<String>>,
    /// Seals the key for the database.
    secrets: Arc<SecretStore>,
}

/// Everything a background job needs.
struct JobSpec {
    job_id: Uuid,
    kind: ProposalKind,
    request: PromptRequest,
    max_tokens: u32,
    project_id: Uuid,
    feature_id: Uuid,
    model_id: Option<Uuid>,
}

/// Validated proposal request.
pub struct ProposalJobRequest {
    /// Kind.
    pub kind: ProposalKind,
    /// Feature context.
    pub feature_id: Uuid,
    /// Model context.
    pub model_id: Option<Uuid>,
    /// User instruction.
    pub prompt: Option<String>,
    /// Number of proposals.
    pub count: Option<u32>,
}

impl AiService {
    /// Creates the service with an optional initial API key.
    pub fn new(store: Store, api_key: Option<String>, secrets: Arc<SecretStore>) -> Self {
        Self {
            store,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .unwrap_or_default(),
            api_key: RwLock::new(api_key.filter(|k| !k.is_empty())),
            secrets,
        }
    }

    /// Loads the stored provider key, so a restart does not lose it.
    ///
    /// `TM_AI_API_KEY` wins when both are set: an operator passing a key
    /// explicitly means it, and it is the way to recover from a rotated
    /// `TM_SECRET_KEY`.
    pub async fn load_persisted_key(&self) {
        if self.api_key.read().await.is_some() {
            tracing::info!("using the API key from the environment, not the stored one");
            return;
        }
        let stored = match self.store.get_ai_secret().await {
            Ok(Some(v)) => v,
            Ok(None) => return,
            Err(e) => {
                tracing::error!(error = %e, "cannot read the stored AI key");
                return;
            }
        };
        match self.secrets.open(&stored.0, &stored.1) {
            Ok(key) => {
                *self.api_key.write().await = Some(key);
                tracing::info!("loaded the stored AI provider key");
            }
            // Never fatal: the server runs fine without AI, and the user can
            // set the key again.
            Err(e) => tracing::error!(error = %e, "cannot use the stored AI key"),
        }
    }

    /// Whether an API key is configured.
    pub async fn secret_configured(&self) -> bool {
        self.api_key.read().await.is_some()
    }

    /// Replaces the API key; an empty string clears it.
    ///
    /// Always persisted, encrypted, so it survives a restart. The key is never
    /// logged, here or anywhere else.
    pub async fn set_api_key(&self, key: String) -> ApiResult<()> {
        let key = Some(key).filter(|k| !k.is_empty());
        let sealed = match &key {
            Some(k) => Some(self.secrets.seal(k).map_err(|e| {
                tracing::error!(error = %e, "cannot seal the AI key");
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "cannot store the API key securely",
                )
            })?),
            None => None,
        };
        // Persisted before the working copy is swapped: if the write fails the
        // caller is told, rather than getting a key that works until restart.
        self.store
            .put_ai_secret(sealed.as_ref().map(|(n, c)| (n.as_slice(), c.as_slice())))
            .await?;
        *self.api_key.write().await = key;
        Ok(())
    }

    async fn provider(&self, settings: &AiSettings) -> Option<Box<dyn LlmProvider>> {
        let key = self.api_key.read().await.clone();
        let base = settings.base_url.as_deref();
        let model = settings.model.as_deref();
        match settings.provider {
            AiProvider::None => None,
            AiProvider::Anthropic => Some(Box::new(AnthropicProvider::new(
                self.http.clone(),
                base,
                key?,
                model,
            ))),
            AiProvider::OpenAiCompatible | AiProvider::Local => Some(Box::new(
                OpenAiCompatibleProvider::new(self.http.clone(), base?, key, model?),
            )),
        }
    }

    /// Validates the request, creates a job and runs it in the background.
    ///
    /// The job is stamped with `tenant` so `GET /jobs/{id}` can be scoped to
    /// it (FR-044).
    pub async fn start_job(
        self: &Arc<Self>,
        tenant: &str,
        req: ProposalJobRequest,
    ) -> ApiResult<Job> {
        let feature = self.store.get_feature(req.feature_id).await?;
        let project_id = self.store.feature_project_id(req.feature_id).await?;
        let model = match req.model_id {
            Some(id) => {
                let m = self.store.get_model(id).await?;
                if m.summary.feature_id != feature.audit.id {
                    return Err(ApiError::unprocessable(
                        "model does not belong to the feature",
                    ));
                }
                Some(m)
            }
            None => None,
        };
        if req.kind == ProposalKind::StatesAndTransitions && model.is_none() {
            return Err(ApiError::unprocessable(
                "modelId is required for states-and-transitions proposals",
            ));
        }
        let settings = self.store.get_ai_settings().await?;
        let Some(provider) = self.provider(&settings).await else {
            return Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "no AI provider configured; see PUT /settings/ai",
            ));
        };
        let existing = self
            .store
            .list_test_cases(
                req.feature_id,
                TestCaseFilter {
                    model_id: req.model_id,
                    ..Default::default()
                },
                tm_storage::PageRequest {
                    limit: MAX_CONTEXT_TEST_CASES,
                    after: None,
                },
            )
            .await?;
        let request = PromptRequest {
            context: ProposalContext {
                feature_name: feature.name.clone(),
                feature_description: feature.description.clone(),
                scenario_description: feature.scenario_description.clone(),
                model: model
                    .as_ref()
                    .map(|m| (m.summary.name.clone(), m.graph.clone())),
                existing_test_cases: existing.items.iter().map(|t| t.data.name.clone()).collect(),
            },
            prompt: req.prompt,
            count: req.count.unwrap_or(DEFAULT_COUNT),
        };
        let max_tokens = settings
            .max_tokens_per_request
            .and_then(|t| u32::try_from(t).ok())
            .filter(|t| *t > 0)
            .unwrap_or(DEFAULT_MAX_TOKENS);

        let job = self.store.create_job(tenant).await?;
        let spec = JobSpec {
            job_id: job.id,
            kind: req.kind,
            request,
            max_tokens,
            project_id,
            feature_id: req.feature_id,
            model_id: req.model_id,
        };
        let service = Arc::clone(self);
        tokio::spawn(async move { service.run_job(spec, provider).await });
        Ok(job)
    }

    async fn run_job(&self, spec: JobSpec, provider: Box<dyn LlmProvider>) {
        let JobSpec {
            job_id,
            kind,
            request,
            max_tokens,
            project_id,
            feature_id,
            model_id,
        } = spec;
        tracing::info!(%job_id, kind = kind.as_str(), prompt_version = PROMPT_VERSION, "AI job started");
        if let Err(e) = self
            .store
            .update_job(job_id, JobStatus::Running, &[], None)
            .await
        {
            tracing::error!(%job_id, error = %e, "cannot mark job running");
        }
        let outcome =
            tm_ai::generate_proposals(provider.as_ref(), kind, &request, max_tokens).await;
        let result = match outcome {
            Ok(drafts) => {
                let proposals: Vec<Proposal> = drafts
                    .into_iter()
                    .map(|draft| Proposal {
                        id: Uuid::new_v4(),
                        project_id,
                        feature_id: Some(feature_id),
                        model_id,
                        kind,
                        status: ProposalStatus::Pending,
                        payload: payload_of(&draft.content),
                        rationale: draft.rationale,
                        source: Some(provider.model_id()),
                        resulting_entity_id: None,
                        created_at: Utc::now(),
                    })
                    .collect();
                self.store
                    .insert_proposals(&proposals)
                    .await
                    .map_err(|e| e.to_string())
            }
            Err(e) => Err(e.to_string()),
        };
        let update = match &result {
            Ok(ids) => {
                tracing::info!(%job_id, proposals = ids.len(), "AI job succeeded");
                self.store
                    .update_job(job_id, JobStatus::Succeeded, ids, None)
                    .await
            }
            Err(msg) => {
                tracing::warn!(%job_id, error = %msg, "AI job failed");
                self.store
                    .update_job(job_id, JobStatus::Failed, &[], Some(msg))
                    .await
            }
        };
        if let Err(e) = update {
            tracing::error!(%job_id, error = %e, "cannot record job result");
        }
    }
}

/// Serializes validated content in the shape of the target entity input.
fn payload_of(content: &ProposalContent) -> serde_json::Value {
    let value = match content {
        ProposalContent::Model {
            name,
            description,
            graph,
        } => serde_json::to_value(ModelInput::from_graph(name, description.as_deref(), graph)),
        ProposalContent::Elements {
            states,
            transitions,
        } => serde_json::to_value(ElementsPayload {
            states: states.iter().map(Into::into).collect(),
            transitions: transitions.iter().map(Into::into).collect(),
        }),
        ProposalContent::TestCase(data) => serde_json::to_value(TestCaseInput::from_data(data)),
        ProposalContent::FeatureDescription(text) => {
            serde_json::to_value(FeatureDescriptionPayload {
                scenario_description: text.clone(),
            })
        }
    };
    value.unwrap_or(serde_json::Value::Null)
}

fn parse_payload<T: serde::de::DeserializeOwned>(payload: &serde_json::Value) -> ApiResult<T> {
    serde_json::from_value(payload.clone()).map_err(|e| {
        ApiError::unprocessable(format!("payload does not match the proposal kind: {e}"))
    })
}

/// Converts a (possibly edited) payload into the entity change to apply on acceptance.
pub fn acceptance_for(proposal: &Proposal, payload: &serde_json::Value) -> ApiResult<Acceptance> {
    let feature_id = proposal
        .feature_id
        .ok_or_else(|| ApiError::unprocessable("the proposal's feature no longer exists"))?;
    match proposal.kind {
        ProposalKind::Model => {
            let input: ModelInput = parse_payload(payload)?;
            let (name, meta, graph) = input.into_parts()?;
            let issues = tm_domain::validate(&graph);
            if tm_domain::validation::has_errors(&issues) {
                return Err(
                    ApiError::unprocessable("proposed model is invalid").with_errors(
                        issues
                            .iter()
                            .map(|i| crate::error::FieldError {
                                field: i.code.into(),
                                message: i.message.clone(),
                            })
                            .collect(),
                    ),
                );
            }
            Ok(Acceptance::CreateModel {
                feature_id,
                name,
                description: meta.description,
                graph,
            })
        }
        ProposalKind::StatesAndTransitions => {
            let model_id = proposal
                .model_id
                .ok_or_else(|| ApiError::unprocessable("the proposal's model no longer exists"))?;
            let input: ElementsPayload = parse_payload(payload)?;
            let graph = ModelInput::graph(Vec::new(), input.states, input.transitions)?;
            Ok(Acceptance::AddToModel {
                model_id,
                states: graph.states,
                transitions: graph.transitions,
            })
        }
        ProposalKind::TestCases => {
            let input: TestCaseInput = parse_payload(payload)?;
            let (data, _) = input.into_parts()?;
            Ok(Acceptance::CreateTestCase {
                feature_id,
                data,
                model_id: proposal.model_id,
            })
        }
        ProposalKind::FeatureDescription => {
            let input: FeatureDescriptionPayload = parse_payload(payload)?;
            Ok(Acceptance::SetScenarioDescription {
                feature_id,
                text: input.scenario_description,
            })
        }
    }
}
