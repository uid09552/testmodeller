//! Traceability: backlog items traced to test cases, model elements and implementations (FR-026, FR-027).
//!
//! Gap rules live here so the API, the UI and the CSV export share them.

use std::collections::HashMap;

use sqlx::Row;
use tm_domain::AssignmentTarget;
use uuid::Uuid;

use crate::{Result, Store};

/// Which gap to narrow the trace to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceGap {
    /// Test cases without a backlog link.
    Untraced,
    /// Backlog items with a test case lacking an implementation link.
    Unimplemented,
    /// Backlog items none of whose test cases is assigned to a state or transition.
    NoElements,
}

/// Scope and gap of a traceability query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TraceFilter {
    /// Only test cases of features of this component.
    pub component_id: Option<Uuid>,
    /// Only test cases of this feature.
    pub feature_id: Option<Uuid>,
    /// Narrow to a gap; counts in the summary are not narrowed.
    pub gap: Option<TraceGap>,
}

/// Model element a test case is assigned to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceElement {
    /// Model containing the element.
    pub model_id: Uuid,
    /// State or transition.
    pub target: AssignmentTarget,
}

/// A test case in a trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceTestCase {
    /// Test case id.
    pub id: Uuid,
    /// Name.
    pub name: String,
    /// Owning feature.
    pub feature_id: Uuid,
    /// Component of that feature.
    pub component_id: Uuid,
    /// Implementation link as entered.
    pub implementation_url: Option<String>,
    /// Assigned states and transitions.
    pub elements: Vec<TraceElement>,
    /// Latest imported result.
    pub last_result: Option<tm_domain::TestResult>,
}

/// A backlog item and the test cases linking it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceItem {
    /// Normalised backlog URL.
    pub backlog_url: String,
    /// Linking test cases, oldest first.
    pub test_cases: Vec<TraceTestCase>,
}

impl TraceItem {
    /// True when no test case of the item is assigned to a state or transition.
    pub fn covers_no_element(&self) -> bool {
        self.test_cases.iter().all(|t| t.elements.is_empty())
    }

    /// True when a test case of the item has no implementation link.
    pub fn has_unimplemented(&self) -> bool {
        self.test_cases
            .iter()
            .any(|t| t.implementation_url.is_none())
    }
}

/// Counts over the scoped test cases, before any gap narrowing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TraceSummary {
    /// Distinct backlog items.
    pub backlog_items: i64,
    /// Test cases in scope.
    pub test_cases: i64,
    /// Test cases without a backlog link.
    pub untraced: i64,
    /// Test cases with a backlog link but no implementation link.
    pub unimplemented: i64,
    /// Backlog items covering no model element.
    pub no_elements: i64,
}

/// Result of a traceability query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Traceability {
    /// Backlog items, ordered by their first test case.
    pub items: Vec<TraceItem>,
    /// Test cases without a backlog link.
    pub untraced: Vec<TraceTestCase>,
    /// Counts.
    pub summary: TraceSummary,
}

impl Store {
    /// Traces the backlog items of a project. Reads only the project's own test cases.
    pub async fn traceability(
        &self,
        project_id: Uuid,
        filter: TraceFilter,
    ) -> Result<Traceability> {
        self.get_project(project_id).await?;
        let rows = sqlx::query(
            "SELECT t.id, t.name, t.feature_id, f.component_id, t.implementation_url, t.backlog_key
             FROM test_cases t
             JOIN features f ON f.id = t.feature_id
             JOIN components c ON c.id = f.component_id
             WHERE c.project_id = $1
               AND ($2::uuid IS NULL OR f.component_id = $2)
               AND ($3::uuid IS NULL OR t.feature_id = $3)
             ORDER BY t.created_at, t.id",
        )
        .bind(project_id)
        .bind(filter.component_id)
        .bind(filter.feature_id)
        .fetch_all(&self.pool)
        .await?;

        let ids: Vec<Uuid> = rows.iter().map(|r| r.get("id")).collect();
        let assigned = sqlx::query(
            "SELECT test_case_id, model_id, state_id, transition_id FROM assignments
             WHERE test_case_id = ANY($1) ORDER BY created_at, step_order",
        )
        .bind(&ids)
        .fetch_all(&self.pool)
        .await?;
        let mut elements: HashMap<Uuid, Vec<TraceElement>> = HashMap::new();
        for row in &assigned {
            let state: Option<Uuid> = row.try_get("state_id")?;
            let transition: Option<Uuid> = row.try_get("transition_id")?;
            let target = match (state, transition) {
                (Some(s), _) => AssignmentTarget::State(s),
                (None, Some(t)) => AssignmentTarget::Transition(t),
                (None, None) => continue,
            };
            let list = elements.entry(row.try_get("test_case_id")?).or_default();
            let element = TraceElement {
                model_id: row.try_get("model_id")?,
                target,
            };
            if !list.contains(&element) {
                list.push(element);
            }
        }

        let mut conn = self.pool.acquire().await?;
        let mut latest = crate::results::latest_results(&mut conn, &ids).await?;

        let mut items: Vec<TraceItem> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut untraced = Vec::new();
        for row in &rows {
            let id: Uuid = row.try_get("id")?;
            let tc = TraceTestCase {
                id,
                name: row.try_get("name")?,
                feature_id: row.try_get("feature_id")?,
                component_id: row.try_get("component_id")?,
                implementation_url: row.try_get("implementation_url")?,
                elements: elements.remove(&id).unwrap_or_default(),
                last_result: latest.remove(&id),
            };
            match row.try_get::<Option<String>, _>("backlog_key")? {
                None => untraced.push(tc),
                Some(key) => match index.get(&key) {
                    Some(&i) => items[i].test_cases.push(tc),
                    None => {
                        index.insert(key.clone(), items.len());
                        items.push(TraceItem {
                            backlog_url: key,
                            test_cases: vec![tc],
                        });
                    }
                },
            }
        }

        let summary = TraceSummary {
            backlog_items: items.len() as i64,
            test_cases: rows.len() as i64,
            untraced: untraced.len() as i64,
            unimplemented: items
                .iter()
                .flat_map(|i| &i.test_cases)
                .filter(|t| t.implementation_url.is_none())
                .count() as i64,
            no_elements: items.iter().filter(|i| i.covers_no_element()).count() as i64,
        };
        match filter.gap {
            None => {}
            Some(TraceGap::Untraced) => items.clear(),
            Some(TraceGap::Unimplemented) => {
                items.retain(TraceItem::has_unimplemented);
                untraced.clear();
            }
            Some(TraceGap::NoElements) => {
                items.retain(TraceItem::covers_no_element);
                untraced.clear();
            }
        }
        Ok(Traceability {
            items,
            untraced,
            summary,
        })
    }
}
