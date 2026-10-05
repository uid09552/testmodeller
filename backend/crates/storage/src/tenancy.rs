//! Tenant lookups (FR-044, NFR-007).
//!
//! The tenant lives on `projects.tenant_id`; every other entity is reachable
//! only through a project. These lookups resolve an entity id to the tenant
//! that owns it, so the API can reject a cross-tenant request before any
//! handler runs. See docs/adr/0005-jwt-auth-and-tenancy.md.

use uuid::Uuid;

use crate::{Result, StorageError, Store};

/// The kinds of entity an API path can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantScope {
    /// `/projects/{id}`
    Project,
    /// `/components/{id}`
    Component,
    /// `/features/{id}`
    Feature,
    /// `/models/{id}`
    Model,
    /// `/states/{id}`
    State,
    /// `/transitions/{id}`
    Transition,
    /// `/test-cases/{id}`
    TestCase,
    /// `/proposals/{id}`
    Proposal,
    /// `/jobs/{id}`
    Job,
}

impl TenantScope {
    /// Entity name used in "not found" errors.
    pub fn entity(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Component => "component",
            Self::Feature => "feature",
            Self::Model => "model",
            Self::State => "state",
            Self::Transition => "transition",
            Self::TestCase => "test case",
            Self::Proposal => "proposal",
            Self::Job => "job",
        }
    }

    /// SQL returning the owning tenant for `$1`.
    ///
    /// Each query walks up to `projects`, except a job, which carries its own
    /// tenant because it hangs off no project.
    fn sql(self) -> &'static str {
        match self {
            Self::Project => "SELECT tenant_id FROM projects WHERE id = $1",
            Self::Component => {
                "SELECT p.tenant_id FROM components c
                 JOIN projects p ON p.id = c.project_id
                 WHERE c.id = $1"
            }
            Self::Feature => {
                "SELECT p.tenant_id FROM features f
                 JOIN components c ON c.id = f.component_id
                 JOIN projects p ON p.id = c.project_id
                 WHERE f.id = $1"
            }
            Self::Model => {
                "SELECT p.tenant_id FROM models m
                 JOIN features f ON f.id = m.feature_id
                 JOIN components c ON c.id = f.component_id
                 JOIN projects p ON p.id = c.project_id
                 WHERE m.id = $1"
            }
            Self::State => {
                "SELECT p.tenant_id FROM states s
                 JOIN models m ON m.id = s.model_id
                 JOIN features f ON f.id = m.feature_id
                 JOIN components c ON c.id = f.component_id
                 JOIN projects p ON p.id = c.project_id
                 WHERE s.id = $1"
            }
            Self::Transition => {
                "SELECT p.tenant_id FROM transitions t
                 JOIN models m ON m.id = t.model_id
                 JOIN features f ON f.id = m.feature_id
                 JOIN components c ON c.id = f.component_id
                 JOIN projects p ON p.id = c.project_id
                 WHERE t.id = $1"
            }
            Self::TestCase => {
                "SELECT p.tenant_id FROM test_cases tc
                 JOIN features f ON f.id = tc.feature_id
                 JOIN components c ON c.id = f.component_id
                 JOIN projects p ON p.id = c.project_id
                 WHERE tc.id = $1"
            }
            Self::Proposal => {
                "SELECT p.tenant_id FROM proposals pr
                 JOIN projects p ON p.id = pr.project_id
                 WHERE pr.id = $1"
            }
            Self::Job => "SELECT tenant_id FROM jobs WHERE id = $1",
        }
    }
}

impl Store {
    /// The tenant owning `id`, or `NotFound` when no such entity exists.
    ///
    /// A job created before tenancy existed has no tenant and is reported as
    /// not found rather than as belonging to everyone.
    pub async fn tenant_of(&self, scope: TenantScope, id: Uuid) -> Result<String> {
        let tenant: Option<String> = sqlx::query_scalar(scope.sql())
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .flatten();
        tenant.ok_or(StorageError::NotFound(scope.entity()))
    }
}
