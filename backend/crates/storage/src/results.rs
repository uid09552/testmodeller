//! Imported test results (FR-050..FR-053). Matching happens in the API; this records and reads.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{PgConnection, Row};
use tm_domain::TestResult;
use uuid::Uuid;

use crate::{parse_col, Result, StorageError, Store};

/// A result to record; the run is given separately.
#[derive(Debug, Clone, PartialEq)]
pub struct NewResult {
    /// Matched test case.
    pub test_case_id: Uuid,
    /// Outcome.
    pub status: tm_domain::ResultStatus,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Failure or error message.
    pub message: Option<String>,
    /// When it ran.
    pub executed_at: DateTime<Utc>,
}

pub(crate) fn test_result(row: &PgRow) -> Result<TestResult, sqlx::Error> {
    Ok(TestResult {
        id: row.try_get("id")?,
        test_case_id: row.try_get("test_case_id")?,
        run_id: row.try_get("run_id")?,
        status: parse_col(row, "status")?,
        duration_ms: row.try_get("duration_ms")?,
        message: row.try_get("message")?,
        executed_at: row.try_get("executed_at")?,
        source: row.try_get("source")?,
    })
}

/// Latest result of each of `ids`: newest run first, ties by import time.
pub(crate) async fn latest_results(
    conn: &mut PgConnection,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, TestResult>> {
    let rows = sqlx::query(
        "SELECT DISTINCT ON (test_case_id) * FROM test_results WHERE test_case_id = ANY($1)
         ORDER BY test_case_id, executed_at DESC, imported_at DESC",
    )
    .bind(ids)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|r| test_result(r).map(|t| (t.test_case_id, t)))
        .collect::<Result<_, sqlx::Error>>()
        .map_err(Into::into)
}

impl Store {
    /// `(id, name)` of every test case of a project, for matching.
    pub async fn project_test_case_names(&self, project_id: Uuid) -> Result<Vec<(Uuid, String)>> {
        self.get_project(project_id).await?;
        let rows = sqlx::query(
            "SELECT t.id, t.name FROM test_cases t
             JOIN features f ON f.id = t.feature_id
             JOIN components c ON c.id = f.component_id
             WHERE c.project_id = $1",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| Ok((r.try_get("id")?, r.try_get("name")?)))
            .collect::<Result<_, sqlx::Error>>()
            .map_err(Into::into)
    }

    /// Which of `test_case_ids` already have a result for `run_id`.
    pub async fn recorded_in_run(
        &self,
        test_case_ids: &[Uuid],
        run_id: &str,
    ) -> Result<HashSet<Uuid>> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT test_case_id FROM test_results WHERE run_id = $2 AND test_case_id = ANY($1)",
        )
        .bind(test_case_ids)
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(ids.into_iter().collect())
    }

    /// Records results of one run for test cases of `project_id`; a test case that already has
    /// a result for the run is left alone. Returns how many were recorded.
    pub async fn record_results(
        &self,
        project_id: Uuid,
        run_id: &str,
        source: &str,
        results: &[NewResult],
    ) -> Result<u64> {
        if results.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await?;
        let tenant: String = sqlx::query_scalar("SELECT tenant_id FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound("project"))?;
        let ids: Vec<Uuid> = results.iter().map(|_| Uuid::new_v4()).collect();
        let cases: Vec<Uuid> = results.iter().map(|r| r.test_case_id).collect();
        let statuses: Vec<&str> = results.iter().map(|r| r.status.as_str()).collect();
        let durations: Vec<Option<i64>> = results.iter().map(|r| r.duration_ms).collect();
        let messages: Vec<Option<String>> = results.iter().map(|r| r.message.clone()).collect();
        let times: Vec<DateTime<Utc>> = results.iter().map(|r| r.executed_at).collect();
        // Only test cases of this project are written, whatever the caller passed.
        let done = sqlx::query(
            "INSERT INTO test_results
                 (id, tenant_id, test_case_id, run_id, status, duration_ms, message, executed_at, source)
             SELECT u.id, $1, u.tc, $2, u.st, u.d, u.m, u.at, $3
             FROM UNNEST($4::uuid[], $5::uuid[], $6::text[], $7::int8[], $8::text[], $9::timestamptz[])
                  AS u(id, tc, st, d, m, at)
             JOIN test_cases t ON t.id = u.tc
             JOIN features f ON f.id = t.feature_id
             JOIN components c ON c.id = f.component_id
             WHERE c.project_id = $10
             ON CONFLICT (test_case_id, run_id) DO NOTHING",
        )
        .bind(&tenant)
        .bind(run_id)
        .bind(source)
        .bind(&ids)
        .bind(&cases)
        .bind(&statuses)
        .bind(&durations)
        .bind(&messages)
        .bind(&times)
        .bind(project_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(done.rows_affected())
    }

    /// Result history of a test case, newest first.
    pub async fn test_results(&self, test_case_id: Uuid, limit: i64) -> Result<Vec<TestResult>> {
        self.get_test_case(test_case_id).await?;
        let rows = sqlx::query(
            "SELECT * FROM test_results WHERE test_case_id = $1
             ORDER BY executed_at DESC, imported_at DESC LIMIT $2",
        )
        .bind(test_case_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(test_result)
            .collect::<Result<_, sqlx::Error>>()
            .map_err(Into::into)
    }

    /// States and transitions of the given models that are covered and whose covering test
    /// cases all have a latest result of `passed`.
    pub async fn passing_elements(
        &self,
        model_ids: &[Uuid],
    ) -> Result<(HashSet<Uuid>, HashSet<Uuid>)> {
        let rows = sqlx::query(
            "WITH latest AS (
                 SELECT DISTINCT ON (r.test_case_id) r.test_case_id, r.status
                 FROM test_results r
                 WHERE r.test_case_id IN (SELECT test_case_id FROM assignments WHERE model_id = ANY($1))
                 ORDER BY r.test_case_id, r.executed_at DESC, r.imported_at DESC)
             SELECT a.state_id, a.transition_id
             FROM assignments a LEFT JOIN latest l ON l.test_case_id = a.test_case_id
             WHERE a.model_id = ANY($1)
             GROUP BY a.state_id, a.transition_id
             HAVING bool_and(l.status IS NOT DISTINCT FROM 'passed')",
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
}
