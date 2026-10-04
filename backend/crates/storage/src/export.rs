//! Project export and import (FR-007, FR-024). Formats are rendered by the API layer.

use std::collections::HashMap;

use tm_domain::{
    AssignmentTarget, Component, Feature, Model, ModelGraph, ModelStatus, Origin, Project,
    TestCase, TestCaseData,
};
use uuid::Uuid;

use crate::models::{insert_model, load_model, remap_ids};
use crate::organization::FeatureFields;
use crate::test_cases::{insert_test_case, load_test_case, NewAssignment};
use crate::{Result, StorageError, Store};

/// What to export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportScope {
    /// Whole project.
    Project,
    /// One component.
    Component(Uuid),
    /// One feature.
    Feature(Uuid),
}

/// Exported feature with its models and test cases.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportFeature {
    /// Feature.
    pub feature: Feature,
    /// Models with graphs.
    pub models: Vec<Model>,
    /// Test cases with assignments.
    pub test_cases: Vec<TestCase>,
}

/// Exported component.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportComponent {
    /// Component.
    pub component: Component,
    /// Features.
    pub features: Vec<ExportFeature>,
}

/// Everything in scope, nested Component > Feature > Model/TestCase.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportBundle {
    /// Exported project.
    pub project: Project,
    /// Components in scope.
    pub components: Vec<ExportComponent>,
}

/// Model to import. `source_id` and graph ids are only used to resolve references.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportModel {
    /// Id in the source export.
    pub source_id: Uuid,
    /// Name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Status.
    pub status: ModelStatus,
    /// Graph with source ids.
    pub graph: ModelGraph,
}

/// Test case to import, referencing source ids.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportTestCase {
    /// Fields.
    pub data: TestCaseData,
    /// Origin.
    pub origin: Origin,
    /// Source model id it was generated from.
    pub generated_from_model_id: Option<Uuid>,
    /// Assignments with source ids.
    pub assignments: Vec<NewAssignment>,
}

/// Feature to import.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportFeature {
    /// Fields.
    pub fields: FeatureFields,
    /// Models.
    pub models: Vec<ImportModel>,
    /// Test cases.
    pub test_cases: Vec<ImportTestCase>,
}

/// Component to import.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportComponent {
    /// Name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Features.
    pub features: Vec<ImportFeature>,
}

/// Number of entities created by an import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportCounts {
    /// Components.
    pub components: i64,
    /// Features.
    pub features: i64,
    /// Models.
    pub models: i64,
    /// Test cases.
    pub test_cases: i64,
}

impl Store {
    /// Loads everything in scope for export.
    pub async fn export_bundle(
        &self,
        project_id: Uuid,
        scope: ExportScope,
    ) -> Result<ExportBundle> {
        let project = self.get_project(project_id).await?;
        let mut conn = self.pool.acquire().await?;
        let component_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT c.id FROM components c WHERE c.project_id = $1
               AND ($2::uuid IS NULL OR c.id = $2)
               AND ($3::uuid IS NULL OR EXISTS (SELECT 1 FROM features f
                    WHERE f.component_id = c.id AND f.id = $3))
             ORDER BY c.created_at, c.id",
        )
        .bind(project_id)
        .bind(match scope {
            ExportScope::Component(id) => Some(id),
            _ => None,
        })
        .bind(match scope {
            ExportScope::Feature(id) => Some(id),
            _ => None,
        })
        .fetch_all(&mut *conn)
        .await?;
        if component_ids.is_empty() && scope != ExportScope::Project {
            return Err(StorageError::NotFound(match scope {
                ExportScope::Feature(_) => "feature",
                _ => "component",
            }));
        }
        let mut components = Vec::with_capacity(component_ids.len());
        for cid in component_ids {
            let component = self.get_component(cid).await?;
            let feature_ids: Vec<Uuid> = sqlx::query_scalar(
                "SELECT id FROM features WHERE component_id = $1 AND ($2::uuid IS NULL OR id = $2)
                 ORDER BY created_at, id",
            )
            .bind(cid)
            .bind(match scope {
                ExportScope::Feature(id) => Some(id),
                _ => None,
            })
            .fetch_all(&mut *conn)
            .await?;
            let mut features = Vec::with_capacity(feature_ids.len());
            for fid in feature_ids {
                let feature = self.get_feature(fid).await?;
                let model_ids: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM models WHERE feature_id = $1 ORDER BY created_at, id",
                )
                .bind(fid)
                .fetch_all(&mut *conn)
                .await?;
                let mut models = Vec::with_capacity(model_ids.len());
                for mid in model_ids {
                    models.push(load_model(&mut conn, mid).await?);
                }
                let case_ids: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM test_cases WHERE feature_id = $1 ORDER BY created_at, id",
                )
                .bind(fid)
                .fetch_all(&mut *conn)
                .await?;
                let mut test_cases = Vec::with_capacity(case_ids.len());
                for tid in case_ids {
                    test_cases.push(load_test_case(&mut conn, tid).await?);
                }
                features.push(ExportFeature {
                    feature,
                    models,
                    test_cases,
                });
            }
            components.push(ExportComponent {
                component,
                features,
            });
        }
        Ok(ExportBundle {
            project,
            components,
        })
    }

    /// Imports components into a project with fresh ids, in one transaction.
    pub async fn import_bundle(
        &self,
        project_id: Uuid,
        components: &[ImportComponent],
    ) -> Result<ImportCounts> {
        self.get_project(project_id).await?;
        let mut tx = self.pool.begin().await?;
        let mut counts = ImportCounts::default();
        for c in components {
            let component_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO components (id, project_id, name, description) VALUES ($1, $2, $3, $4)",
            )
            .bind(component_id)
            .bind(project_id)
            .bind(&c.name)
            .bind(&c.description)
            .execute(&mut *tx)
            .await?;
            counts.components += 1;
            for f in &c.features {
                let feature_id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO features (id, component_id, name, description, scenario_description, tags)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(feature_id)
                .bind(component_id)
                .bind(&f.fields.name)
                .bind(&f.fields.description)
                .bind(&f.fields.scenario_description)
                .bind(f.fields.tags.clone().unwrap_or_default())
                .execute(&mut *tx)
                .await?;
                counts.features += 1;

                let mut ids: HashMap<Uuid, Uuid> = HashMap::new();
                for m in &f.models {
                    let (graph, map) = remap_ids(&m.graph);
                    let issues = tm_domain::validation::structural_issues(&graph);
                    if let Some(issue) = issues.first() {
                        return Err(StorageError::Invalid(format!(
                            "model '{}': {}",
                            m.name, issue.message
                        )));
                    }
                    let model_id = insert_model(
                        &mut tx,
                        feature_id,
                        &m.name,
                        m.description.as_deref(),
                        m.status,
                        &graph,
                    )
                    .await?;
                    ids.insert(m.source_id, model_id);
                    ids.extend(map);
                    counts.models += 1;
                }
                let resolve = |id: Uuid| {
                    ids.get(&id).copied().ok_or_else(|| {
                        StorageError::Invalid(format!("unknown reference {id} in import"))
                    })
                };
                for tc in &f.test_cases {
                    let assignments = tc
                        .assignments
                        .iter()
                        .map(|a| {
                            Ok(NewAssignment {
                                model_id: resolve(a.model_id)?,
                                target: match a.target {
                                    AssignmentTarget::State(s) => {
                                        AssignmentTarget::State(resolve(s)?)
                                    }
                                    AssignmentTarget::Transition(t) => {
                                        AssignmentTarget::Transition(resolve(t)?)
                                    }
                                },
                                step_order: a.step_order,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let generated_from = tc
                        .generated_from_model_id
                        .and_then(|id| ids.get(&id).copied());
                    insert_test_case(
                        &mut tx,
                        feature_id,
                        &tc.data,
                        tc.origin,
                        generated_from,
                        &assignments,
                    )
                    .await?;
                    counts.test_cases += 1;
                }
            }
        }
        tx.commit().await?;
        Ok(counts)
    }
}
