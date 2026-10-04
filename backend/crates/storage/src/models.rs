//! Models, their graphs, version history, states and transitions (FR-003, FR-010 to FR-014).
//!
//! Every change to a model's metadata or graph increments the model `version` and stores a full
//! snapshot in `model_versions`.

use std::collections::{HashMap, HashSet};

use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgConnection, Row};
use tm_domain::{
    Model, ModelGraph, ModelSnapshot, ModelStatus, ModelSummary, ModelVersion, Position, State,
    Transition, Variable,
};
use uuid::Uuid;

use crate::{audit, missing_or_conflict, parse_col, Result, StorageError, Store};

/// Model metadata. `None` keeps the stored value on update.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelMeta {
    /// Name.
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Status.
    pub status: Option<ModelStatus>,
}

/// A state with its owning model and assignment count.
#[derive(Debug, Clone, PartialEq)]
pub struct StateRecord {
    /// The state.
    pub state: State,
    /// Owning model.
    pub model_id: Uuid,
    /// Distinct assigned test cases.
    pub test_case_count: i64,
}

/// A transition with its owning model and assignment count.
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionRecord {
    /// The transition.
    pub transition: Transition,
    /// Owning model.
    pub model_id: Uuid,
    /// Distinct assigned test cases.
    pub test_case_count: i64,
}

const SUMMARY_SELECT: &str = "SELECT m.*,
    (SELECT count(*) FROM states s WHERE s.model_id = m.id) AS state_count,
    (SELECT count(*) FROM transitions t WHERE t.model_id = m.id) AS transition_count,
    (SELECT count(DISTINCT a.test_case_id) FROM assignments a WHERE a.model_id = m.id) AS test_case_count
    FROM models m";

const STATE_SELECT: &str = "SELECT s.*,
    (SELECT count(DISTINCT a.test_case_id) FROM assignments a WHERE a.state_id = s.id) AS test_case_count
    FROM states s";

const TRANSITION_SELECT: &str = "SELECT t.*,
    (SELECT count(DISTINCT a.test_case_id) FROM assignments a WHERE a.transition_id = t.id) AS test_case_count
    FROM transitions t";

fn summary(row: &PgRow) -> Result<ModelSummary, sqlx::Error> {
    Ok(ModelSummary {
        audit: audit(row)?,
        feature_id: row.try_get("feature_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        status: parse_col(row, "status")?,
        state_count: row.try_get("state_count")?,
        transition_count: row.try_get("transition_count")?,
        test_case_count: row.try_get("test_case_count")?,
    })
}

fn state(row: &PgRow) -> Result<State, sqlx::Error> {
    let x: Option<f64> = row.try_get("pos_x")?;
    let y: Option<f64> = row.try_get("pos_y")?;
    Ok(State {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        kind: parse_col(row, "kind")?,
        position: x.zip(y).map(|(x, y)| Position { x, y }),
    })
}

fn transition(row: &PgRow) -> Result<Transition, sqlx::Error> {
    Ok(Transition {
        id: row.try_get("id")?,
        from: row.try_get("from_state")?,
        to: row.try_get("to_state")?,
        event: row.try_get("event")?,
        guard: row.try_get("guard")?,
        action: row.try_get("action")?,
        expected: row.try_get("expected")?,
    })
}

fn state_record(row: &PgRow) -> Result<StateRecord, sqlx::Error> {
    Ok(StateRecord {
        state: state(row)?,
        model_id: row.try_get("model_id")?,
        test_case_count: row.try_get("test_case_count")?,
    })
}

fn transition_record(row: &PgRow) -> Result<TransitionRecord, sqlx::Error> {
    Ok(TransitionRecord {
        transition: transition(row)?,
        model_id: row.try_get("model_id")?,
        test_case_count: row.try_get("test_case_count")?,
    })
}

pub(crate) async fn load_summary(conn: &mut PgConnection, id: Uuid) -> Result<ModelSummary> {
    let sql = format!("{SUMMARY_SELECT} WHERE m.id = $1");
    let row = sqlx::query(&sql)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("model"))?;
    Ok(summary(&row)?)
}

pub(crate) async fn load_graph(conn: &mut PgConnection, id: Uuid) -> Result<ModelGraph> {
    let variables: Json<Vec<Variable>> =
        sqlx::query_scalar("SELECT variables FROM models WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("model"))?;
    let states = sqlx::query("SELECT * FROM states WHERE model_id = $1 ORDER BY sort_order, id")
        .bind(id)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(state)
        .collect::<Result<Vec<_>, _>>()?;
    let transitions =
        sqlx::query("SELECT * FROM transitions WHERE model_id = $1 ORDER BY sort_order, id")
            .bind(id)
            .fetch_all(&mut *conn)
            .await?
            .iter()
            .map(transition)
            .collect::<Result<Vec<_>, _>>()?;
    Ok(ModelGraph {
        variables: variables.0,
        states,
        transitions,
    })
}

pub(crate) async fn load_model(conn: &mut PgConnection, id: Uuid) -> Result<Model> {
    let summary = load_summary(conn, id).await?;
    let graph = load_graph(conn, id).await?;
    let rows = sqlx::query(
        "SELECT state_id, transition_id, count(DISTINCT test_case_id) AS n
         FROM assignments WHERE model_id = $1 GROUP BY state_id, transition_id",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let mut state_counts = HashMap::new();
    let mut transition_counts = HashMap::new();
    for row in rows {
        let n: i64 = row.try_get("n")?;
        if let Some(s) = row.try_get::<Option<Uuid>, _>("state_id")? {
            state_counts.insert(s, n);
        }
        if let Some(t) = row.try_get::<Option<Uuid>, _>("transition_id")? {
            transition_counts.insert(t, n);
        }
    }
    Ok(Model {
        summary,
        graph,
        state_test_case_counts: state_counts,
        transition_test_case_counts: transition_counts,
    })
}

/// Increments the model version; with `expected` set, only if it matches.
async fn bump_version(conn: &mut PgConnection, id: Uuid, expected: Option<i32>) -> Result<i32> {
    let version: Option<i32> = sqlx::query_scalar(
        "UPDATE models SET version = version + 1, updated_at = now()
         WHERE id = $1 AND ($2::int IS NULL OR version = $2) RETURNING version",
    )
    .bind(id)
    .bind(expected)
    .fetch_optional(&mut *conn)
    .await?;
    match version {
        Some(v) => Ok(v),
        None => Err(missing_or_conflict(conn, "models", "model", id).await),
    }
}

/// Stores a snapshot of the model's current state under its current version.
async fn record_version(conn: &mut PgConnection, id: Uuid, summary_text: &str) -> Result<()> {
    let meta = load_summary(conn, id).await?;
    let graph = load_graph(conn, id).await?;
    let snapshot = ModelSnapshot {
        name: meta.name,
        description: meta.description,
        status: meta.status,
        graph,
    };
    sqlx::query(
        "INSERT INTO model_versions (model_id, version, summary, snapshot) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(meta.audit.version)
    .bind(summary_text)
    .bind(Json(&snapshot))
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn bump_and_record(
    conn: &mut PgConnection,
    id: Uuid,
    expected: Option<i32>,
    summary_text: &str,
) -> Result<()> {
    bump_version(conn, id, expected).await?;
    record_version(conn, id, summary_text).await
}

/// Locks the model row and returns it, failing with `NotFound` if absent.
async fn lock_model(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query("SELECT 1 FROM models WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("model"))?;
    Ok(())
}

async fn ensure_ids_free(
    conn: &mut PgConnection,
    model_id: Uuid,
    graph: &ModelGraph,
) -> Result<()> {
    let state_ids: Vec<Uuid> = graph.states.iter().map(|s| s.id).collect();
    let transition_ids: Vec<Uuid> = graph.transitions.iter().map(|t| t.id).collect();
    let taken: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM states WHERE id = ANY($2) AND model_id <> $1)
             OR EXISTS (SELECT 1 FROM transitions WHERE id = ANY($3) AND model_id <> $1)",
    )
    .bind(model_id)
    .bind(&state_ids)
    .bind(&transition_ids)
    .fetch_one(&mut *conn)
    .await?;
    if taken {
        return Err(StorageError::Invalid(
            "a state or transition id is already used by another model".into(),
        ));
    }
    Ok(())
}

async fn upsert_states(
    conn: &mut PgConnection,
    model_id: Uuid,
    states: &[State],
    first_order: i32,
) -> Result<()> {
    if states.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = states.iter().map(|s| s.id).collect();
    let names: Vec<&str> = states.iter().map(|s| s.name.as_str()).collect();
    let descriptions: Vec<Option<&str>> = states.iter().map(|s| s.description.as_deref()).collect();
    let kinds: Vec<&str> = states.iter().map(|s| s.kind.as_str()).collect();
    let xs: Vec<Option<f64>> = states.iter().map(|s| s.position.map(|p| p.x)).collect();
    let ys: Vec<Option<f64>> = states.iter().map(|s| s.position.map(|p| p.y)).collect();
    let orders: Vec<i32> = (0..states.len() as i32).map(|i| first_order + i).collect();
    sqlx::query(
        "INSERT INTO states (id, model_id, name, description, kind, pos_x, pos_y, sort_order)
         SELECT u.id, $1, u.name, u.description, u.kind, u.x, u.y, u.ord
         FROM UNNEST($2::uuid[], $3::text[], $4::text[], $5::text[], $6::float8[], $7::float8[], $8::int4[])
             AS u(id, name, description, kind, x, y, ord)
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, description = EXCLUDED.description,
             kind = EXCLUDED.kind, pos_x = EXCLUDED.pos_x, pos_y = EXCLUDED.pos_y,
             sort_order = EXCLUDED.sort_order",
    )
    .bind(model_id)
    .bind(&ids)
    .bind(&names)
    .bind(&descriptions)
    .bind(&kinds)
    .bind(&xs)
    .bind(&ys)
    .bind(&orders)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn upsert_transitions(
    conn: &mut PgConnection,
    model_id: Uuid,
    transitions: &[Transition],
    first_order: i32,
) -> Result<()> {
    if transitions.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = transitions.iter().map(|t| t.id).collect();
    let froms: Vec<Uuid> = transitions.iter().map(|t| t.from).collect();
    let tos: Vec<Uuid> = transitions.iter().map(|t| t.to).collect();
    let events: Vec<&str> = transitions.iter().map(|t| t.event.as_str()).collect();
    let guards: Vec<Option<&str>> = transitions.iter().map(|t| t.guard.as_deref()).collect();
    let actions: Vec<Option<&str>> = transitions.iter().map(|t| t.action.as_deref()).collect();
    let expected: Vec<Option<&str>> = transitions.iter().map(|t| t.expected.as_deref()).collect();
    let orders: Vec<i32> = (0..transitions.len() as i32)
        .map(|i| first_order + i)
        .collect();
    sqlx::query(
        "INSERT INTO transitions (id, model_id, from_state, to_state, event, guard, action, expected, sort_order)
         SELECT u.id, $1, u.f, u.t, u.event, u.guard, u.action, u.expected, u.ord
         FROM UNNEST($2::uuid[], $3::uuid[], $4::uuid[], $5::text[], $6::text[], $7::text[], $8::text[], $9::int4[])
             AS u(id, f, t, event, guard, action, expected, ord)
         ON CONFLICT (id) DO UPDATE SET from_state = EXCLUDED.from_state, to_state = EXCLUDED.to_state,
             event = EXCLUDED.event, guard = EXCLUDED.guard, action = EXCLUDED.action,
             expected = EXCLUDED.expected, sort_order = EXCLUDED.sort_order",
    )
    .bind(model_id)
    .bind(&ids)
    .bind(&froms)
    .bind(&tos)
    .bind(&events)
    .bind(&guards)
    .bind(&actions)
    .bind(&expected)
    .bind(&orders)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Replaces states, transitions and variables, keeping ids that remain. Assignments to removed
/// elements are deleted by foreign key cascade.
pub(crate) async fn replace_graph(
    conn: &mut PgConnection,
    model_id: Uuid,
    graph: &ModelGraph,
) -> Result<()> {
    ensure_ids_free(conn, model_id, graph).await?;
    let state_ids: Vec<Uuid> = graph.states.iter().map(|s| s.id).collect();
    let transition_ids: Vec<Uuid> = graph.transitions.iter().map(|t| t.id).collect();
    // Upsert first so kept transitions re-pointed to new states survive the state cleanup.
    upsert_states(conn, model_id, &graph.states, 0).await?;
    upsert_transitions(conn, model_id, &graph.transitions, 0).await?;
    sqlx::query("DELETE FROM transitions WHERE model_id = $1 AND NOT (id = ANY($2))")
        .bind(model_id)
        .bind(&transition_ids)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM states WHERE model_id = $1 AND NOT (id = ANY($2))")
        .bind(model_id)
        .bind(&state_ids)
        .execute(&mut *conn)
        .await?;
    sqlx::query("UPDATE models SET variables = $2 WHERE id = $1")
        .bind(model_id)
        .bind(Json(&graph.variables))
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Inserts a model with its graph and records version 1.
pub(crate) async fn insert_model(
    conn: &mut PgConnection,
    feature_id: Uuid,
    name: &str,
    description: Option<&str>,
    status: ModelStatus,
    graph: &ModelGraph,
) -> Result<Uuid> {
    let exists = sqlx::query("SELECT 1 FROM features WHERE id = $1")
        .bind(feature_id)
        .fetch_optional(&mut *conn)
        .await?;
    if exists.is_none() {
        return Err(StorageError::NotFound("feature"));
    }
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO models (id, feature_id, name, description, status) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(feature_id)
    .bind(name)
    .bind(description)
    .bind(status.as_str())
    .execute(&mut *conn)
    .await?;
    replace_graph(conn, id, graph).await?;
    record_version(conn, id, "Created").await?;
    Ok(id)
}

/// Appends new states and transitions to an existing model (used for accepted AI proposals).
pub(crate) async fn append_elements(
    conn: &mut PgConnection,
    model_id: Uuid,
    states: &[State],
    transitions: &[Transition],
) -> Result<()> {
    lock_model(conn, model_id).await?;
    let mut graph = load_graph(conn, model_id).await?;
    let first_state = graph.states.len() as i32;
    let first_transition = graph.transitions.len() as i32;
    graph.states.extend_from_slice(states);
    graph.transitions.extend_from_slice(transitions);
    let issues = tm_domain::validation::structural_issues(&graph);
    if let Some(issue) = issues.first() {
        return Err(StorageError::Invalid(issue.message.clone()));
    }
    ensure_ids_free(conn, model_id, &graph).await?;
    upsert_states(conn, model_id, states, first_state).await?;
    upsert_transitions(conn, model_id, transitions, first_transition).await?;
    bump_and_record(conn, model_id, None, "AI proposal accepted").await
}

impl Store {
    /// Lists model summaries of a feature.
    pub async fn list_models(&self, feature_id: Uuid) -> Result<Vec<ModelSummary>> {
        self.get_feature(feature_id).await?;
        let sql = format!("{SUMMARY_SELECT} WHERE m.feature_id = $1 ORDER BY m.created_at, m.id");
        let rows = sqlx::query(&sql)
            .bind(feature_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(summary).collect::<Result<Vec<_>, _>>()?)
    }

    /// Creates a model with an optional graph. The graph must be structurally valid.
    pub async fn create_model(
        &self,
        feature_id: Uuid,
        name: &str,
        meta: ModelMeta,
        graph: &ModelGraph,
    ) -> Result<Model> {
        let mut tx = self.pool.begin().await?;
        let id = insert_model(
            &mut tx,
            feature_id,
            name,
            meta.description.as_deref(),
            meta.status.unwrap_or(ModelStatus::Draft),
            graph,
        )
        .await?;
        let model = load_model(&mut tx, id).await?;
        tx.commit().await?;
        Ok(model)
    }

    /// Gets a model with its graph.
    pub async fn get_model(&self, id: Uuid) -> Result<Model> {
        let mut conn = self.pool.acquire().await?;
        load_model(&mut conn, id).await
    }

    /// Graph of a model.
    pub async fn get_model_graph(&self, id: Uuid) -> Result<ModelGraph> {
        let mut conn = self.pool.acquire().await?;
        load_graph(&mut conn, id).await
    }

    /// Replaces metadata and graph atomically if `expected_version` matches.
    pub async fn replace_model(
        &self,
        id: Uuid,
        expected_version: i32,
        name: &str,
        meta: ModelMeta,
        graph: &ModelGraph,
    ) -> Result<Model> {
        let mut tx = self.pool.begin().await?;
        bump_version(&mut tx, id, Some(expected_version)).await?;
        sqlx::query(
            "UPDATE models SET name = $2, description = $3, status = COALESCE($4, status)
             WHERE id = $1",
        )
        .bind(id)
        .bind(name)
        .bind(&meta.description)
        .bind(meta.status.map(|s| s.as_str()))
        .execute(&mut *tx)
        .await?;
        replace_graph(&mut tx, id, graph).await?;
        record_version(&mut tx, id, "Graph saved").await?;
        let model = load_model(&mut tx, id).await?;
        tx.commit().await?;
        Ok(model)
    }

    /// Updates name, description or status if `expected_version` matches.
    pub async fn update_model_meta(
        &self,
        id: Uuid,
        expected_version: i32,
        meta: ModelMeta,
    ) -> Result<ModelSummary> {
        let mut tx = self.pool.begin().await?;
        bump_version(&mut tx, id, Some(expected_version)).await?;
        sqlx::query(
            "UPDATE models SET name = COALESCE($2, name), description = COALESCE($3, description),
                 status = COALESCE($4, status)
             WHERE id = $1",
        )
        .bind(id)
        .bind(&meta.name)
        .bind(&meta.description)
        .bind(meta.status.map(|s| s.as_str()))
        .execute(&mut *tx)
        .await?;
        record_version(&mut tx, id, "Metadata updated").await?;
        let summary = load_summary(&mut tx, id).await?;
        tx.commit().await?;
        Ok(summary)
    }

    /// Deletes a model; assignments are removed, test cases kept.
    pub async fn delete_model(&self, id: Uuid) -> Result<()> {
        let done = sqlx::query("DELETE FROM models WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(StorageError::NotFound("model"));
        }
        Ok(())
    }

    /// Copies a model with fresh ids, optionally into another feature. Assignments are not copied.
    pub async fn duplicate_model(
        &self,
        id: Uuid,
        name: Option<&str>,
        feature_id: Option<Uuid>,
    ) -> Result<Model> {
        let mut tx = self.pool.begin().await?;
        let source = load_summary(&mut tx, id).await?;
        let graph = load_graph(&mut tx, id).await?;
        let (copy, _) = remap_ids(&graph);
        let name = name.map_or_else(|| format!("{} (copy)", source.name), str::to_owned);
        let new_id = insert_model(
            &mut tx,
            feature_id.unwrap_or(source.feature_id),
            &name,
            source.description.as_deref(),
            source.status,
            &copy,
        )
        .await?;
        let model = load_model(&mut tx, new_id).await?;
        tx.commit().await?;
        Ok(model)
    }

    /// Version history, newest first.
    pub async fn model_versions(&self, id: Uuid) -> Result<Vec<ModelVersion>> {
        self.get_model_graph(id).await?;
        let rows = sqlx::query(
            "SELECT version, created_at, summary FROM model_versions
             WHERE model_id = $1 ORDER BY version DESC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(ModelVersion {
                    version: r.try_get("version")?,
                    created_at: r.try_get("created_at")?,
                    summary: r.try_get("summary")?,
                })
            })
            .collect()
    }

    /// Restores a previous version as a new version.
    pub async fn restore_model_version(&self, id: Uuid, version: i32) -> Result<Model> {
        let mut tx = self.pool.begin().await?;
        lock_model(&mut tx, id).await?;
        let snapshot: Json<ModelSnapshot> = sqlx::query_scalar(
            "SELECT snapshot FROM model_versions WHERE model_id = $1 AND version = $2",
        )
        .bind(id)
        .bind(version)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(StorageError::NotFound("model version"))?;
        let snapshot = snapshot.0;
        bump_version(&mut tx, id, None).await?;
        sqlx::query("UPDATE models SET name = $2, description = $3, status = $4 WHERE id = $1")
            .bind(id)
            .bind(&snapshot.name)
            .bind(&snapshot.description)
            .bind(snapshot.status.as_str())
            .execute(&mut *tx)
            .await?;
        replace_graph(&mut tx, id, &snapshot.graph).await?;
        record_version(&mut tx, id, &format!("Restored version {version}")).await?;
        let model = load_model(&mut tx, id).await?;
        tx.commit().await?;
        Ok(model)
    }

    /// Lists states of a model.
    pub async fn list_states(&self, model_id: Uuid) -> Result<Vec<StateRecord>> {
        self.get_model_graph(model_id).await?;
        let sql = format!("{STATE_SELECT} WHERE s.model_id = $1 ORDER BY s.sort_order, s.id");
        let rows = sqlx::query(&sql)
            .bind(model_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(state_record)
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Gets a state.
    pub async fn get_state(&self, id: Uuid) -> Result<StateRecord> {
        let sql = format!("{STATE_SELECT} WHERE s.id = $1");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("state"))?;
        Ok(state_record(&row)?)
    }

    /// Adds a state to a model.
    pub async fn add_state(&self, model_id: Uuid, state: State) -> Result<StateRecord> {
        let mut tx = self.pool.begin().await?;
        lock_model(&mut tx, model_id).await?;
        let taken = sqlx::query("SELECT 1 FROM states WHERE id = $1")
            .bind(state.id)
            .fetch_optional(&mut *tx)
            .await?;
        if taken.is_some() {
            return Err(StorageError::Invalid(format!(
                "state id {} already exists",
                state.id
            )));
        }
        let order: i32 = sqlx::query_scalar(
            "SELECT COALESCE(max(sort_order) + 1, 0) FROM states WHERE model_id = $1",
        )
        .bind(model_id)
        .fetch_one(&mut *tx)
        .await?;
        upsert_states(&mut tx, model_id, std::slice::from_ref(&state), order).await?;
        bump_and_record(
            &mut tx,
            model_id,
            None,
            &format!("State '{}' added", state.name),
        )
        .await?;
        tx.commit().await?;
        self.get_state(state.id).await
    }

    /// Updates a state; `None` description/position keep stored values.
    pub async fn update_state(&self, id: Uuid, update: State) -> Result<StateRecord> {
        let mut tx = self.pool.begin().await?;
        let model_id: Uuid = sqlx::query_scalar("SELECT model_id FROM states WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound("state"))?;
        lock_model(&mut tx, model_id).await?;
        sqlx::query(
            "UPDATE states SET name = $2, kind = $3, description = COALESCE($4, description),
                 pos_x = COALESCE($5, pos_x), pos_y = COALESCE($6, pos_y)
             WHERE id = $1",
        )
        .bind(id)
        .bind(&update.name)
        .bind(update.kind.as_str())
        .bind(&update.description)
        .bind(update.position.map(|p| p.x))
        .bind(update.position.map(|p| p.y))
        .execute(&mut *tx)
        .await?;
        bump_and_record(
            &mut tx,
            model_id,
            None,
            &format!("State '{}' updated", update.name),
        )
        .await?;
        tx.commit().await?;
        self.get_state(id).await
    }

    /// Deletes a state with its incident transitions and their assignments.
    pub async fn delete_state(&self, id: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT model_id, name FROM states WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound("state"))?;
        let model_id: Uuid = row.try_get("model_id")?;
        let name: String = row.try_get("name")?;
        lock_model(&mut tx, model_id).await?;
        sqlx::query("DELETE FROM states WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        bump_and_record(&mut tx, model_id, None, &format!("State '{name}' deleted")).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Lists transitions of a model.
    pub async fn list_transitions(&self, model_id: Uuid) -> Result<Vec<TransitionRecord>> {
        self.get_model_graph(model_id).await?;
        let sql = format!("{TRANSITION_SELECT} WHERE t.model_id = $1 ORDER BY t.sort_order, t.id");
        let rows = sqlx::query(&sql)
            .bind(model_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(transition_record)
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Gets a transition.
    pub async fn get_transition(&self, id: Uuid) -> Result<TransitionRecord> {
        let sql = format!("{TRANSITION_SELECT} WHERE t.id = $1");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("transition"))?;
        Ok(transition_record(&row)?)
    }

    async fn ensure_states_in_model(
        conn: &mut PgConnection,
        model_id: Uuid,
        t: &Transition,
    ) -> Result<()> {
        let ids: Vec<Uuid> = HashSet::from([t.from, t.to]).into_iter().collect();
        let found: i64 =
            sqlx::query_scalar("SELECT count(*) FROM states WHERE model_id = $1 AND id = ANY($2)")
                .bind(model_id)
                .bind(&ids)
                .fetch_one(&mut *conn)
                .await?;
        if found != ids.len() as i64 {
            return Err(StorageError::Invalid(
                "transition must connect states of the same model".into(),
            ));
        }
        Ok(())
    }

    /// Adds a transition to a model.
    pub async fn add_transition(&self, model_id: Uuid, t: Transition) -> Result<TransitionRecord> {
        let mut tx = self.pool.begin().await?;
        lock_model(&mut tx, model_id).await?;
        Self::ensure_states_in_model(&mut tx, model_id, &t).await?;
        let taken = sqlx::query("SELECT 1 FROM transitions WHERE id = $1")
            .bind(t.id)
            .fetch_optional(&mut *tx)
            .await?;
        if taken.is_some() {
            return Err(StorageError::Invalid(format!(
                "transition id {} already exists",
                t.id
            )));
        }
        let order: i32 = sqlx::query_scalar(
            "SELECT COALESCE(max(sort_order) + 1, 0) FROM transitions WHERE model_id = $1",
        )
        .bind(model_id)
        .fetch_one(&mut *tx)
        .await?;
        upsert_transitions(&mut tx, model_id, std::slice::from_ref(&t), order).await?;
        bump_and_record(
            &mut tx,
            model_id,
            None,
            &format!("Transition '{}' added", t.event),
        )
        .await?;
        tx.commit().await?;
        self.get_transition(t.id).await
    }

    /// Updates a transition; `None` guard/action/expected keep stored values.
    pub async fn update_transition(&self, id: Uuid, t: Transition) -> Result<TransitionRecord> {
        let mut tx = self.pool.begin().await?;
        let model_id: Uuid = sqlx::query_scalar("SELECT model_id FROM transitions WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound("transition"))?;
        lock_model(&mut tx, model_id).await?;
        Self::ensure_states_in_model(&mut tx, model_id, &t).await?;
        sqlx::query(
            "UPDATE transitions SET from_state = $2, to_state = $3, event = $4,
                 guard = COALESCE($5, guard), action = COALESCE($6, action),
                 expected = COALESCE($7, expected)
             WHERE id = $1",
        )
        .bind(id)
        .bind(t.from)
        .bind(t.to)
        .bind(&t.event)
        .bind(&t.guard)
        .bind(&t.action)
        .bind(&t.expected)
        .execute(&mut *tx)
        .await?;
        bump_and_record(
            &mut tx,
            model_id,
            None,
            &format!("Transition '{}' updated", t.event),
        )
        .await?;
        tx.commit().await?;
        self.get_transition(id).await
    }

    /// Deletes a transition and its assignments.
    pub async fn delete_transition(&self, id: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT model_id, event FROM transitions WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound("transition"))?;
        let model_id: Uuid = row.try_get("model_id")?;
        let event: String = row.try_get("event")?;
        lock_model(&mut tx, model_id).await?;
        sqlx::query("DELETE FROM transitions WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        bump_and_record(
            &mut tx,
            model_id,
            None,
            &format!("Transition '{event}' deleted"),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Ids and graphs of all models of a feature.
    pub async fn feature_graphs(&self, feature_id: Uuid) -> Result<Vec<(Uuid, ModelGraph)>> {
        self.get_feature(feature_id).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM models WHERE feature_id = $1 ORDER BY created_at, id",
        )
        .bind(feature_id)
        .fetch_all(&self.pool)
        .await?;
        self.graphs(ids).await
    }

    /// Ids and graphs of all models of a component.
    pub async fn component_graphs(&self, component_id: Uuid) -> Result<Vec<(Uuid, ModelGraph)>> {
        self.get_component(component_id).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT m.id FROM models m JOIN features f ON f.id = m.feature_id
             WHERE f.component_id = $1 ORDER BY m.created_at, m.id",
        )
        .bind(component_id)
        .fetch_all(&self.pool)
        .await?;
        self.graphs(ids).await
    }

    async fn graphs(&self, ids: Vec<Uuid>) -> Result<Vec<(Uuid, ModelGraph)>> {
        let mut conn = self.pool.acquire().await?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            out.push((id, load_graph(&mut conn, id).await?));
        }
        Ok(out)
    }
}

/// Copies a graph with fresh state and transition ids; returns the copy and the old-to-new id map.
pub(crate) fn remap_ids(graph: &ModelGraph) -> (ModelGraph, HashMap<Uuid, Uuid>) {
    let mut map: HashMap<Uuid, Uuid> = graph
        .states
        .iter()
        .map(|s| (s.id, Uuid::new_v4()))
        .collect();
    map.extend(graph.transitions.iter().map(|t| (t.id, Uuid::new_v4())));
    let remap = |id: Uuid| map.get(&id).copied().unwrap_or(id);
    let copy = ModelGraph {
        variables: graph.variables.clone(),
        states: graph
            .states
            .iter()
            .map(|s| State {
                id: remap(s.id),
                ..s.clone()
            })
            .collect(),
        transitions: graph
            .transitions
            .iter()
            .map(|t| Transition {
                id: remap(t.id),
                from: remap(t.from),
                to: remap(t.to),
                ..t.clone()
            })
            .collect(),
    };
    (copy, map)
}
