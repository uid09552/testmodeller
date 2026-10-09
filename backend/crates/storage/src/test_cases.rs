//! Test cases and their assignments to model states and transitions (FR-005, FR-006, FR-022).

use std::collections::{HashMap, HashSet};

use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgConnection, Row};
use tm_domain::links::backlog_key;
use tm_domain::{
    Assignment, AssignmentTarget, Origin, TestCase, TestCaseData, TestCaseStatus, TestStep,
};
use uuid::Uuid;

use crate::results::latest_results;
use crate::{
    audit, into_page, keyset, missing_or_conflict, parse_col, parse_opt_col, Page, PageRequest,
    Result, StorageError, Store,
};

/// Assignment to create; the test case is implied by context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewAssignment {
    /// Model containing the target.
    pub model_id: Uuid,
    /// State or transition.
    pub target: AssignmentTarget,
    /// Optional 1-based test step.
    pub step_order: Option<i32>,
}

/// Filters for listing test cases of a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TestCaseFilter {
    /// Only test cases assigned somewhere in this model.
    pub model_id: Option<Uuid>,
    /// Only this status.
    pub status: Option<TestCaseStatus>,
    /// Only this origin.
    pub origin: Option<Origin>,
    /// Only test cases without assignments.
    pub unassigned: bool,
}

fn test_case(row: &PgRow) -> Result<TestCase, sqlx::Error> {
    let steps: Json<Vec<TestStep>> = row.try_get("steps")?;
    Ok(TestCase {
        audit: audit(row)?,
        feature_id: row.try_get("feature_id")?,
        data: TestCaseData {
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            preconditions: row.try_get("preconditions")?,
            priority: parse_opt_col(row, "priority")?,
            status: parse_col(row, "status")?,
            tags: row.try_get("tags")?,
            steps: steps.0,
            implementation_url: row.try_get("implementation_url")?,
            backlog_url: row.try_get("backlog_url")?,
        },
        origin: parse_col(row, "origin")?,
        generated_from_model_id: row.try_get("generated_from_model_id")?,
        assignments: Vec::new(),
        last_result: None,
    })
}

fn assignment(row: &PgRow) -> Result<Assignment, sqlx::Error> {
    let state: Option<Uuid> = row.try_get("state_id")?;
    let transition: Option<Uuid> = row.try_get("transition_id")?;
    let target = match (state, transition) {
        (Some(s), _) => AssignmentTarget::State(s),
        (None, Some(t)) => AssignmentTarget::Transition(t),
        (None, None) => {
            return Err(sqlx::Error::ColumnDecode {
                index: "state_id".into(),
                source: "assignment without target".into(),
            })
        }
    };
    Ok(Assignment {
        test_case_id: row.try_get("test_case_id")?,
        model_id: row.try_get("model_id")?,
        target,
        step_order: row.try_get("step_order")?,
    })
}

fn target_ids(target: AssignmentTarget) -> (Option<Uuid>, Option<Uuid>) {
    match target {
        AssignmentTarget::State(s) => (Some(s), None),
        AssignmentTarget::Transition(t) => (None, Some(t)),
    }
}

/// Grouping key of a backlog link, stored beside it.
fn key_of(url: &Option<String>) -> Option<String> {
    url.as_deref().and_then(backlog_key)
}

/// Normalizes step order to 1..n in list order.
fn numbered(steps: &[TestStep]) -> Vec<TestStep> {
    steps
        .iter()
        .enumerate()
        .map(|(i, s)| TestStep {
            order: i as i32 + 1,
            ..s.clone()
        })
        .collect()
}

async fn attach_assignments(conn: &mut PgConnection, cases: &mut [TestCase]) -> Result<()> {
    let ids: Vec<Uuid> = cases.iter().map(|c| c.audit.id).collect();
    let rows = sqlx::query(
        "SELECT * FROM assignments WHERE test_case_id = ANY($1) ORDER BY created_at, step_order",
    )
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await?;
    let mut by_case: HashMap<Uuid, Vec<Assignment>> = HashMap::new();
    for row in &rows {
        let a = assignment(row)?;
        by_case.entry(a.test_case_id).or_default().push(a);
    }
    let mut latest = latest_results(conn, &ids).await?;
    for c in cases {
        c.assignments = by_case.remove(&c.audit.id).unwrap_or_default();
        c.last_result = latest.remove(&c.audit.id);
    }
    Ok(())
}

pub(crate) async fn load_test_case(conn: &mut PgConnection, id: Uuid) -> Result<TestCase> {
    let row = sqlx::query("SELECT * FROM test_cases WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("test case"))?;
    let mut cases = vec![test_case(&row)?];
    attach_assignments(conn, &mut cases).await?;
    Ok(cases.remove(0))
}

/// Checks that every target belongs to its model and every model to `feature_id` (422 otherwise).
async fn validate_assignments(
    conn: &mut PgConnection,
    feature_id: Uuid,
    step_count: usize,
    assignments: &[NewAssignment],
) -> Result<()> {
    if assignments.is_empty() {
        return Ok(());
    }
    let model_ids: Vec<Uuid> = assignments.iter().map(|a| a.model_id).collect();
    let model_features: HashMap<Uuid, Uuid> =
        sqlx::query("SELECT id, feature_id FROM models WHERE id = ANY($1)")
            .bind(&model_ids)
            .fetch_all(&mut *conn)
            .await?
            .iter()
            .map(|r| Ok((r.try_get("id")?, r.try_get("feature_id")?)))
            .collect::<Result<_, sqlx::Error>>()?;
    let (states, transitions): (Vec<_>, Vec<_>) =
        assignments.iter().map(|a| target_ids(a.target)).unzip();
    let states: Vec<Uuid> = states.into_iter().flatten().collect();
    let transitions: Vec<Uuid> = transitions.into_iter().flatten().collect();
    let element_models: HashMap<Uuid, Uuid> = sqlx::query(
        "SELECT id, model_id FROM states WHERE id = ANY($1)
         UNION ALL SELECT id, model_id FROM transitions WHERE id = ANY($2)",
    )
    .bind(&states)
    .bind(&transitions)
    .fetch_all(&mut *conn)
    .await?
    .iter()
    .map(|r| Ok((r.try_get("id")?, r.try_get("model_id")?)))
    .collect::<Result<_, sqlx::Error>>()?;

    for a in assignments {
        if model_features.get(&a.model_id) != Some(&feature_id) {
            return Err(StorageError::Invalid(format!(
                "model {} does not belong to the test case's feature",
                a.model_id
            )));
        }
        let (kind, id) = match a.target {
            AssignmentTarget::State(id) => ("state", id),
            AssignmentTarget::Transition(id) => ("transition", id),
        };
        if element_models.get(&id) != Some(&a.model_id) {
            return Err(StorageError::Invalid(format!(
                "{kind} {id} does not belong to model {}",
                a.model_id
            )));
        }
        if let Some(order) = a.step_order {
            if order < 1 || order as usize > step_count {
                return Err(StorageError::Invalid(format!(
                    "stepOrder {order} is outside the test case's {step_count} steps"
                )));
            }
        }
    }
    Ok(())
}

async fn write_assignments(
    conn: &mut PgConnection,
    test_case_id: Uuid,
    assignments: &[NewAssignment],
) -> Result<()> {
    sqlx::query("DELETE FROM assignments WHERE test_case_id = $1")
        .bind(test_case_id)
        .execute(&mut *conn)
        .await?;
    insert_assignments(conn, test_case_id, assignments).await
}

async fn insert_assignments(
    conn: &mut PgConnection,
    test_case_id: Uuid,
    assignments: &[NewAssignment],
) -> Result<()> {
    if assignments.is_empty() {
        return Ok(());
    }
    let models: Vec<Uuid> = assignments.iter().map(|a| a.model_id).collect();
    let (states, transitions): (Vec<Option<Uuid>>, Vec<Option<Uuid>>) =
        assignments.iter().map(|a| target_ids(a.target)).unzip();
    let orders: Vec<Option<i32>> = assignments.iter().map(|a| a.step_order).collect();
    sqlx::query(
        "INSERT INTO assignments (test_case_id, model_id, state_id, transition_id, step_order)
         SELECT $1, u.m, u.s, u.t, u.o
         FROM UNNEST($2::uuid[], $3::uuid[], $4::uuid[], $5::int4[]) AS u(m, s, t, o)
         ON CONFLICT DO NOTHING",
    )
    .bind(test_case_id)
    .bind(&models)
    .bind(&states)
    .bind(&transitions)
    .bind(&orders)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Inserts a test case with validated assignments.
pub(crate) async fn insert_test_case(
    conn: &mut PgConnection,
    feature_id: Uuid,
    data: &TestCaseData,
    origin: Origin,
    generated_from_model_id: Option<Uuid>,
    assignments: &[NewAssignment],
) -> Result<Uuid> {
    let exists = sqlx::query("SELECT 1 FROM features WHERE id = $1")
        .bind(feature_id)
        .fetch_optional(&mut *conn)
        .await?;
    if exists.is_none() {
        return Err(StorageError::NotFound("feature"));
    }
    validate_assignments(conn, feature_id, data.steps.len(), assignments).await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO test_cases (id, feature_id, name, description, preconditions, priority,
             status, tags, origin, generated_from_model_id, steps,
             implementation_url, backlog_url, backlog_key)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(id)
    .bind(feature_id)
    .bind(&data.name)
    .bind(&data.description)
    .bind(&data.preconditions)
    .bind(data.priority.map(|p| p.as_str()))
    .bind(data.status.as_str())
    .bind(&data.tags)
    .bind(origin.as_str())
    .bind(generated_from_model_id)
    .bind(Json(numbered(&data.steps)))
    .bind(&data.implementation_url)
    .bind(&data.backlog_url)
    .bind(key_of(&data.backlog_url))
    .execute(&mut *conn)
    .await?;
    insert_assignments(conn, id, assignments).await?;
    Ok(id)
}

impl Store {
    /// Lists test cases of a feature.
    pub async fn list_test_cases(
        &self,
        feature_id: Uuid,
        filter: TestCaseFilter,
        page: PageRequest,
    ) -> Result<Page<TestCase>> {
        self.get_feature(feature_id).await?;
        let (ts, id) = page.cursor_parts();
        let sql = format!(
            "SELECT * FROM test_cases t WHERE feature_id = $1
               AND ($2::uuid IS NULL OR EXISTS (SELECT 1 FROM assignments a
                    WHERE a.test_case_id = t.id AND a.model_id = $2))
               AND ($3::text IS NULL OR status = $3)
               AND ($4::text IS NULL OR origin = $4)
               AND (NOT $5 OR NOT EXISTS (SELECT 1 FROM assignments a WHERE a.test_case_id = t.id))
               AND {}
             ORDER BY created_at, id LIMIT $8",
            keyset(6, 7)
        );
        let mut conn = self.pool.acquire().await?;
        let rows = sqlx::query(&sql)
            .bind(feature_id)
            .bind(filter.model_id)
            .bind(filter.status.map(|s| s.as_str()))
            .bind(filter.origin.map(|o| o.as_str()))
            .bind(filter.unassigned)
            .bind(ts)
            .bind(id)
            .bind(page.limit + 1)
            .fetch_all(&mut *conn)
            .await?;
        let items = rows.iter().map(test_case).collect::<Result<Vec<_>, _>>()?;
        let mut page = into_page(items, page.limit, |c| (c.audit.created_at, c.audit.id));
        attach_assignments(&mut conn, &mut page.items).await?;
        Ok(page)
    }

    /// Creates a test case.
    pub async fn create_test_case(
        &self,
        feature_id: Uuid,
        data: &TestCaseData,
        origin: Origin,
        assignments: &[NewAssignment],
    ) -> Result<TestCase> {
        let mut tx = self.pool.begin().await?;
        let id = insert_test_case(&mut tx, feature_id, data, origin, None, assignments).await?;
        let tc = load_test_case(&mut tx, id).await?;
        tx.commit().await?;
        Ok(tc)
    }

    /// Gets a test case with assignments.
    pub async fn get_test_case(&self, id: Uuid) -> Result<TestCase> {
        let mut conn = self.pool.acquire().await?;
        load_test_case(&mut conn, id).await
    }

    /// Replaces a test case if `expected_version` matches. `assignments: None` keeps them,
    /// dropping only those that point at steps that no longer exist.
    pub async fn replace_test_case(
        &self,
        id: Uuid,
        expected_version: i32,
        data: &TestCaseData,
        assignments: Option<&[NewAssignment]>,
    ) -> Result<TestCase> {
        let mut tx = self.pool.begin().await?;
        let feature_id: Option<Uuid> = sqlx::query_scalar(
            "UPDATE test_cases SET name = $3, description = $4, preconditions = $5, priority = $6,
                 status = $7, tags = $8, steps = $9, implementation_url = $10, backlog_url = $11,
                 backlog_key = $12, version = version + 1, updated_at = now()
             WHERE id = $1 AND version = $2 RETURNING feature_id",
        )
        .bind(id)
        .bind(expected_version)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.preconditions)
        .bind(data.priority.map(|p| p.as_str()))
        .bind(data.status.as_str())
        .bind(&data.tags)
        .bind(Json(numbered(&data.steps)))
        .bind(&data.implementation_url)
        .bind(&data.backlog_url)
        .bind(key_of(&data.backlog_url))
        .fetch_optional(&mut *tx)
        .await?;
        let Some(feature_id) = feature_id else {
            return Err(missing_or_conflict(&mut tx, "test_cases", "test case", id).await);
        };
        match assignments {
            Some(list) => {
                validate_assignments(&mut tx, feature_id, data.steps.len(), list).await?;
                write_assignments(&mut tx, id, list).await?;
            }
            None => {
                sqlx::query("DELETE FROM assignments WHERE test_case_id = $1 AND step_order > $2")
                    .bind(id)
                    .bind(data.steps.len() as i32)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        let tc = load_test_case(&mut tx, id).await?;
        tx.commit().await?;
        Ok(tc)
    }

    /// Deletes a test case.
    pub async fn delete_test_case(&self, id: Uuid) -> Result<()> {
        let done = sqlx::query("DELETE FROM test_cases WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(StorageError::NotFound("test case"));
        }
        Ok(())
    }

    /// Moves a test case to another feature, clearing its assignments.
    pub async fn move_test_case(&self, id: Uuid, feature_id: Uuid) -> Result<TestCase> {
        self.get_feature(feature_id).await?;
        let mut tx = self.pool.begin().await?;
        let done = sqlx::query(
            "UPDATE test_cases SET feature_id = $2, generated_from_model_id = NULL,
                 version = version + 1, updated_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(feature_id)
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() == 0 {
            return Err(StorageError::NotFound("test case"));
        }
        write_assignments(&mut tx, id, &[]).await?;
        let tc = load_test_case(&mut tx, id).await?;
        tx.commit().await?;
        Ok(tc)
    }

    /// Assignments of a test case.
    pub async fn list_assignments(&self, test_case_id: Uuid) -> Result<Vec<Assignment>> {
        Ok(self.get_test_case(test_case_id).await?.assignments)
    }

    /// Replaces all assignments of a test case.
    pub async fn replace_assignments(
        &self,
        test_case_id: Uuid,
        assignments: &[NewAssignment],
    ) -> Result<Vec<Assignment>> {
        let mut tx = self.pool.begin().await?;
        let tc = load_test_case(&mut tx, test_case_id).await?;
        validate_assignments(&mut tx, tc.feature_id, tc.data.steps.len(), assignments).await?;
        write_assignments(&mut tx, test_case_id, assignments).await?;
        let tc = load_test_case(&mut tx, test_case_id).await?;
        tx.commit().await?;
        Ok(tc.assignments)
    }

    /// Assigns a test case to a state or transition (idempotent).
    pub async fn assign(&self, test_case_id: Uuid, target: AssignmentTarget) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let tc = load_test_case(&mut tx, test_case_id).await?;
        let (sql, entity) = match target {
            AssignmentTarget::State(_) => ("SELECT model_id FROM states WHERE id = $1", "state"),
            AssignmentTarget::Transition(_) => (
                "SELECT model_id FROM transitions WHERE id = $1",
                "transition",
            ),
        };
        let (AssignmentTarget::State(id) | AssignmentTarget::Transition(id)) = target;
        let model_id: Uuid = sqlx::query_scalar(sql)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound(entity))?;
        let new = [NewAssignment {
            model_id,
            target,
            step_order: None,
        }];
        validate_assignments(&mut tx, tc.feature_id, tc.data.steps.len(), &new).await?;
        insert_assignments(&mut tx, test_case_id, &new).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Removes all assignments of a test case to a state or transition.
    pub async fn unassign(&self, test_case_id: Uuid, target: AssignmentTarget) -> Result<()> {
        let (state, transition) = target_ids(target);
        sqlx::query(
            "DELETE FROM assignments WHERE test_case_id = $1
               AND state_id IS NOT DISTINCT FROM $2 AND transition_id IS NOT DISTINCT FROM $3",
        )
        .bind(test_case_id)
        .bind(state)
        .bind(transition)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Test cases assigned to a state or transition.
    pub async fn test_cases_for_target(&self, target: AssignmentTarget) -> Result<Vec<TestCase>> {
        let (state, transition) = target_ids(target);
        match target {
            AssignmentTarget::State(id) => self.get_state(id).await.map(|_| ())?,
            AssignmentTarget::Transition(id) => self.get_transition(id).await.map(|_| ())?,
        }
        self.test_cases_where(
            "SELECT DISTINCT t.* FROM test_cases t JOIN assignments a ON a.test_case_id = t.id
             WHERE ($1::uuid IS NOT NULL AND a.state_id = $1)
                OR ($2::uuid IS NOT NULL AND a.transition_id = $2)
             ORDER BY t.created_at, t.id",
            state,
            transition,
        )
        .await
    }

    /// Test cases assigned anywhere in a model.
    pub async fn test_cases_for_model(&self, model_id: Uuid) -> Result<Vec<TestCase>> {
        self.get_model_graph(model_id).await?;
        self.test_cases_where(
            "SELECT DISTINCT t.* FROM test_cases t JOIN assignments a ON a.test_case_id = t.id
             WHERE a.model_id = $1 AND $2::uuid IS NULL
             ORDER BY t.created_at, t.id",
            Some(model_id),
            None,
        )
        .await
    }

    /// Test cases generated from a model (origin `generated`), with their assignments.
    pub async fn generated_test_cases(&self, model_id: Uuid) -> Result<Vec<TestCase>> {
        self.test_cases_where(
            "SELECT t.* FROM test_cases t
             WHERE t.generated_from_model_id = $1 AND t.origin = 'generated' AND $2::uuid IS NULL
             ORDER BY t.created_at, t.id",
            Some(model_id),
            None,
        )
        .await
    }

    async fn test_cases_where(
        &self,
        sql: &str,
        a: Option<Uuid>,
        b: Option<Uuid>,
    ) -> Result<Vec<TestCase>> {
        let mut conn = self.pool.acquire().await?;
        let rows = sqlx::query(sql)
            .bind(a)
            .bind(b)
            .fetch_all(&mut *conn)
            .await?;
        let mut items = rows.iter().map(test_case).collect::<Result<Vec<_>, _>>()?;
        attach_assignments(&mut conn, &mut items).await?;
        Ok(items)
    }

    /// States and transitions of the given models that have at least one assignment.
    pub async fn covered_elements(
        &self,
        model_ids: &[Uuid],
    ) -> Result<(HashSet<Uuid>, HashSet<Uuid>)> {
        let rows = sqlx::query(
            "SELECT DISTINCT state_id, transition_id FROM assignments WHERE model_id = ANY($1)",
        )
        .bind(model_ids)
        .fetch_all(&self.pool)
        .await?;
        let mut states = HashSet::new();
        let mut transitions = HashSet::new();
        for row in rows {
            if let Some(s) = row.try_get::<Option<Uuid>, _>("state_id")? {
                states.insert(s);
            }
            if let Some(t) = row.try_get::<Option<Uuid>, _>("transition_id")? {
                transitions.insert(t);
            }
        }
        Ok((states, transitions))
    }

    /// Persists generated test cases with origin `generated` and their assignments.
    pub async fn save_generated(
        &self,
        model_id: Uuid,
        cases: &[(TestCaseData, Vec<NewAssignment>)],
    ) -> Result<Vec<TestCase>> {
        let mut tx = self.pool.begin().await?;
        let feature_id: Uuid = sqlx::query_scalar("SELECT feature_id FROM models WHERE id = $1")
            .bind(model_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound("model"))?;
        let mut ids = Vec::with_capacity(cases.len());
        for (data, assignments) in cases {
            ids.push(
                insert_test_case(
                    &mut tx,
                    feature_id,
                    data,
                    Origin::Generated,
                    Some(model_id),
                    assignments,
                )
                .await?,
            );
        }
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            out.push(load_test_case(&mut tx, id).await?);
        }
        tx.commit().await?;
        Ok(out)
    }
}
