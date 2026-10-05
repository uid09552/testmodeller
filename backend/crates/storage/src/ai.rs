//! Proposals, jobs and AI settings (FR-030 to FR-034). API keys are never persisted.

use sqlx::postgres::PgRow;
use sqlx::Row;
use tm_domain::{
    AiSettings, Job, JobStatus, ModelGraph, ModelStatus, Proposal, ProposalStatus, State,
    TestCaseData, Transition,
};
use uuid::Uuid;

use crate::models::{append_elements, insert_model};
use crate::test_cases::insert_test_case;
use crate::{into_page, keyset, parse_col, Page, PageRequest, Result, StorageError, Store};

/// Filters for listing proposals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProposalFilter {
    /// Only this status.
    pub status: Option<ProposalStatus>,
    /// Only this feature.
    pub feature_id: Option<Uuid>,
    /// Only this model.
    pub model_id: Option<Uuid>,
}

/// Entity change performed when a proposal is accepted.
#[derive(Debug, Clone, PartialEq)]
pub enum Acceptance {
    /// Create a new model in a feature.
    CreateModel {
        /// Target feature.
        feature_id: Uuid,
        /// Model name.
        name: String,
        /// Model description.
        description: Option<String>,
        /// Graph with fresh ids.
        graph: ModelGraph,
    },
    /// Add states and transitions to an existing model.
    AddToModel {
        /// Target model.
        model_id: Uuid,
        /// New states.
        states: Vec<State>,
        /// New transitions.
        transitions: Vec<Transition>,
    },
    /// Create a test case with origin `ai`.
    CreateTestCase {
        /// Target feature.
        feature_id: Uuid,
        /// Test case fields.
        data: TestCaseData,
        /// Model the proposal was made for.
        model_id: Option<Uuid>,
    },
    /// Set a feature's scenario description.
    SetScenarioDescription {
        /// Target feature.
        feature_id: Uuid,
        /// New text.
        text: String,
    },
}

fn proposal(row: &PgRow) -> Result<Proposal, sqlx::Error> {
    Ok(Proposal {
        id: row.try_get("id")?,
        project_id: row.try_get("project_id")?,
        feature_id: row.try_get("feature_id")?,
        model_id: row.try_get("model_id")?,
        kind: parse_col(row, "kind")?,
        status: parse_col(row, "status")?,
        payload: row.try_get("payload")?,
        rationale: row.try_get("rationale")?,
        source: row.try_get("source")?,
        resulting_entity_id: row.try_get("resulting_entity_id")?,
        created_at: row.try_get("created_at")?,
    })
}

fn job(row: &PgRow) -> Result<Job, sqlx::Error> {
    Ok(Job {
        id: row.try_get("id")?,
        status: parse_col(row, "status")?,
        proposal_ids: row.try_get("proposal_ids")?,
        error: row.try_get("error")?,
    })
}

impl Store {
    /// Creates a queued job owned by `tenant`.
    ///
    /// The tenant is stored on the job itself: a job hangs off no project, so
    /// `GET /jobs/{id}` has nothing else to scope by (FR-044).
    pub async fn create_job(&self, tenant: &str) -> Result<Job> {
        let row = sqlx::query(
            "INSERT INTO jobs (id, status, tenant_id) VALUES ($1, 'queued', $2) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(tenant)
        .fetch_one(&self.pool)
        .await?;
        Ok(job(&row)?)
    }

    /// Updates job status, created proposals and error.
    pub async fn update_job(
        &self,
        id: Uuid,
        status: JobStatus,
        proposal_ids: &[Uuid],
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE jobs SET status = $2, proposal_ids = $3, error = $4, updated_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(status.as_str())
        .bind(proposal_ids)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Gets a job.
    pub async fn get_job(&self, id: Uuid) -> Result<Job> {
        let row = sqlx::query("SELECT * FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("job"))?;
        Ok(job(&row)?)
    }

    /// Marks jobs left queued or running by a previous process as failed.
    pub async fn fail_interrupted_jobs(&self) -> Result<u64> {
        let done = sqlx::query(
            "UPDATE jobs SET status = 'failed', error = 'interrupted by server restart',
                 updated_at = now()
             WHERE status IN ('queued', 'running')",
        )
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected())
    }

    /// Stores pending proposals; returns their ids.
    pub async fn insert_proposals(&self, proposals: &[Proposal]) -> Result<Vec<Uuid>> {
        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::with_capacity(proposals.len());
        for p in proposals {
            sqlx::query(
                "INSERT INTO proposals (id, project_id, feature_id, model_id, kind, status, payload,
                     rationale, source)
                 VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $8)",
            )
            .bind(p.id)
            .bind(p.project_id)
            .bind(p.feature_id)
            .bind(p.model_id)
            .bind(p.kind.as_str())
            .bind(&p.payload)
            .bind(&p.rationale)
            .bind(&p.source)
            .execute(&mut *tx)
            .await?;
            ids.push(p.id);
        }
        tx.commit().await?;
        Ok(ids)
    }

    /// Lists proposals of a project.
    pub async fn list_proposals(
        &self,
        project_id: Uuid,
        filter: ProposalFilter,
        page: PageRequest,
    ) -> Result<Page<Proposal>> {
        self.get_project(project_id).await?;
        let (ts, id) = page.cursor_parts();
        let sql = format!(
            "SELECT * FROM proposals WHERE project_id = $1
               AND ($2::text IS NULL OR status = $2)
               AND ($3::uuid IS NULL OR feature_id = $3)
               AND ($4::uuid IS NULL OR model_id = $4)
               AND {}
             ORDER BY created_at, id LIMIT $7",
            keyset(5, 6)
        );
        let rows = sqlx::query(&sql)
            .bind(project_id)
            .bind(filter.status.map(|s| s.as_str()))
            .bind(filter.feature_id)
            .bind(filter.model_id)
            .bind(ts)
            .bind(id)
            .bind(page.limit + 1)
            .fetch_all(&self.pool)
            .await?;
        let items = rows.iter().map(proposal).collect::<Result<Vec<_>, _>>()?;
        Ok(into_page(items, page.limit, |p| (p.created_at, p.id)))
    }

    /// Gets a proposal.
    pub async fn get_proposal(&self, id: Uuid) -> Result<Proposal> {
        let row = sqlx::query("SELECT * FROM proposals WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("proposal"))?;
        Ok(proposal(&row)?)
    }

    /// Accepts a pending proposal: applies `acceptance` and records the (possibly edited) payload
    /// in one transaction. Fails with `Conflict` if the proposal is not pending.
    pub async fn accept_proposal(
        &self,
        id: Uuid,
        payload: &serde_json::Value,
        acceptance: Acceptance,
    ) -> Result<Proposal> {
        let mut tx = self.pool.begin().await?;
        let status: ProposalStatus = {
            let row = sqlx::query("SELECT status FROM proposals WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(StorageError::NotFound("proposal"))?;
            parse_col(&row, "status")?
        };
        if status != ProposalStatus::Pending {
            return Err(StorageError::Conflict(format!(
                "proposal is already {status}"
            )));
        }
        let resulting = match acceptance {
            Acceptance::CreateModel {
                feature_id,
                name,
                description,
                graph,
            } => {
                insert_model(
                    &mut tx,
                    feature_id,
                    &name,
                    description.as_deref(),
                    ModelStatus::Draft,
                    &graph,
                )
                .await?
            }
            Acceptance::AddToModel {
                model_id,
                states,
                transitions,
            } => {
                append_elements(&mut tx, model_id, &states, &transitions).await?;
                model_id
            }
            Acceptance::CreateTestCase {
                feature_id,
                data,
                model_id,
            } => {
                insert_test_case(
                    &mut tx,
                    feature_id,
                    &data,
                    tm_domain::Origin::Ai,
                    model_id,
                    &[],
                )
                .await?
            }
            Acceptance::SetScenarioDescription { feature_id, text } => {
                let done = sqlx::query(
                    "UPDATE features SET scenario_description = $2, version = version + 1,
                         updated_at = now()
                     WHERE id = $1",
                )
                .bind(feature_id)
                .bind(&text)
                .execute(&mut *tx)
                .await?;
                if done.rows_affected() == 0 {
                    return Err(StorageError::NotFound("feature"));
                }
                feature_id
            }
        };
        let row = sqlx::query(
            "UPDATE proposals SET status = 'accepted', payload = $2, resulting_entity_id = $3
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(payload)
        .bind(resulting)
        .fetch_one(&mut *tx)
        .await?;
        let accepted = proposal(&row)?;
        tx.commit().await?;
        Ok(accepted)
    }

    /// Rejects a pending proposal.
    pub async fn reject_proposal(&self, id: Uuid, reason: Option<&str>) -> Result<Proposal> {
        let row = sqlx::query(
            "UPDATE proposals SET status = 'rejected', reject_reason = $2
             WHERE id = $1 AND status = 'pending' RETURNING *",
        )
        .bind(id)
        .bind(reason)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => Ok(proposal(&row)?),
            None => {
                let existing = self.get_proposal(id).await?;
                Err(StorageError::Conflict(format!(
                    "proposal is already {}",
                    existing.status
                )))
            }
        }
    }

    /// Current AI settings (defaults if never saved).
    pub async fn get_ai_settings(&self) -> Result<AiSettings> {
        let row = sqlx::query("SELECT * FROM ai_settings WHERE id")
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(AiSettings::default());
        };
        Ok(AiSettings {
            provider: parse_col(&row, "provider")?,
            base_url: row.try_get("base_url")?,
            model: row.try_get("model")?,
            max_tokens_per_request: row.try_get("max_tokens_per_request")?,
        })
    }

    /// The server's data key, generating and storing one on first use.
    ///
    /// Only reached when `TM_SECRET_KEY` is not configured. `ON CONFLICT DO
    /// NOTHING` plus a re-read makes two servers starting at once agree on one
    /// key rather than each overwriting the other's.
    pub async fn get_or_create_data_key(&self, generated: &[u8]) -> Result<Vec<u8>> {
        sqlx::query(
            "INSERT INTO server_secrets (id, data_key) VALUES (true, $1)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(generated)
        .execute(&self.pool)
        .await?;
        let key: Vec<u8> = sqlx::query_scalar("SELECT data_key FROM server_secrets WHERE id")
            .fetch_one(&self.pool)
            .await?;
        Ok(key)
    }

    /// The stored provider key as `(nonce, ciphertext)`, if one was saved.
    ///
    /// The value is encrypted; only the API layer can open it (see
    /// `tm_api::secrets`). Storage never sees the key itself.
    pub async fn get_ai_secret(&self) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        let row = sqlx::query("SELECT api_key_nonce, api_key_ciphertext FROM ai_settings WHERE id")
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else { return Ok(None) };
        let nonce: Option<Vec<u8>> = row.try_get("api_key_nonce")?;
        let ciphertext: Option<Vec<u8>> = row.try_get("api_key_ciphertext")?;
        Ok(nonce.zip(ciphertext))
    }

    /// Stores, or with `None` clears, the encrypted provider key.
    pub async fn put_ai_secret(&self, secret: Option<(&[u8], &[u8])>) -> Result<()> {
        let (nonce, ciphertext) = match secret {
            Some((n, c)) => (Some(n), Some(c)),
            None => (None, None),
        };
        // The settings row may not exist yet, so this upserts rather than
        // updates: a key can be configured before anything else is.
        sqlx::query(
            "INSERT INTO ai_settings (id, api_key_nonce, api_key_ciphertext)
             VALUES (true, $1, $2)
             ON CONFLICT (id) DO UPDATE SET api_key_nonce = EXCLUDED.api_key_nonce,
                 api_key_ciphertext = EXCLUDED.api_key_ciphertext",
        )
        .bind(nonce)
        .bind(ciphertext)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Saves AI settings.
    pub async fn put_ai_settings(&self, settings: &AiSettings) -> Result<AiSettings> {
        sqlx::query(
            // The key columns are deliberately absent: saving settings must
            // not disturb a key stored in the same row.
            "INSERT INTO ai_settings (id, provider, base_url, model, max_tokens_per_request)
             VALUES (true, $1, $2, $3, $4)
             ON CONFLICT (id) DO UPDATE SET provider = EXCLUDED.provider,
                 base_url = EXCLUDED.base_url, model = EXCLUDED.model,
                 max_tokens_per_request = EXCLUDED.max_tokens_per_request",
        )
        .bind(settings.provider.as_str())
        .bind(&settings.base_url)
        .bind(&settings.model)
        .bind(settings.max_tokens_per_request)
        .execute(&self.pool)
        .await?;
        self.get_ai_settings().await
    }
}
