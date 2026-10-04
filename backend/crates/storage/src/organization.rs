//! Projects, components, features, explorer tree and search (FR-001 to FR-004).

use std::collections::HashMap;

use sqlx::postgres::PgRow;
use sqlx::Row;
use tm_domain::{Component, Feature, ModelStatus, Project};
use uuid::Uuid;

use crate::{
    audit, into_page, keyset, missing_or_conflict, parse_col, Page, PageRequest, Result,
    StorageError, Store,
};

/// Name and description of a project or component. On update, `None` keeps the stored value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedFields {
    /// Name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
}

/// Editable feature fields. On update, `None` keeps the stored value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureFields {
    /// Name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Scenario description (markdown).
    pub scenario_description: Option<String>,
    /// Tags.
    pub tags: Option<Vec<String>>,
}

/// Model node of the explorer tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeModel {
    /// Id.
    pub id: Uuid,
    /// Name.
    pub name: String,
    /// Status.
    pub status: ModelStatus,
}

/// Feature node of the explorer tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeFeature {
    /// Id.
    pub id: Uuid,
    /// Name.
    pub name: String,
    /// Models.
    pub models: Vec<TreeModel>,
}

/// Component node of the explorer tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeComponent {
    /// Id.
    pub id: Uuid,
    /// Name.
    pub name: String,
    /// Features.
    pub features: Vec<TreeFeature>,
}

/// Searchable entity type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchType {
    /// Component.
    Component,
    /// Feature.
    Feature,
    /// Model.
    Model,
    /// Test case.
    TestCase,
}

/// Search result.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    /// Entity type.
    pub hit_type: SearchType,
    /// Id.
    pub id: Uuid,
    /// Name.
    pub name: String,
    /// Description excerpt.
    pub snippet: Option<String>,
}

const SEARCH_LIMIT: i64 = 50;

fn project(row: &PgRow) -> Result<Project, sqlx::Error> {
    Ok(Project {
        audit: audit(row)?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
    })
}

fn component(row: &PgRow) -> Result<Component, sqlx::Error> {
    Ok(Component {
        audit: audit(row)?,
        project_id: row.try_get("project_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
    })
}

pub(crate) fn feature(row: &PgRow) -> Result<Feature, sqlx::Error> {
    Ok(Feature {
        audit: audit(row)?,
        component_id: row.try_get("component_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        scenario_description: row.try_get("scenario_description")?,
        tags: row.try_get("tags")?,
    })
}

/// Escapes `%`, `_` and `\` for use in `ILIKE`.
fn like_pattern(q: &str) -> String {
    let mut out = String::with_capacity(q.len() + 2);
    out.push('%');
    for c in q.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

impl Store {
    /// Lists projects.
    pub async fn list_projects(&self, page: PageRequest) -> Result<Page<Project>> {
        let (ts, id) = page.cursor_parts();
        let sql = format!(
            "SELECT * FROM projects WHERE {} ORDER BY created_at, id LIMIT $3",
            keyset(1, 2)
        );
        let rows = sqlx::query(&sql)
            .bind(ts)
            .bind(id)
            .bind(page.limit + 1)
            .fetch_all(&self.pool)
            .await?;
        let items = rows.iter().map(project).collect::<Result<Vec<_>, _>>()?;
        Ok(into_page(items, page.limit, |p| {
            (p.audit.created_at, p.audit.id)
        }))
    }

    /// Creates a project.
    pub async fn create_project(&self, input: NamedFields) -> Result<Project> {
        let row = sqlx::query(
            "INSERT INTO projects (id, name, description) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(&self.pool)
        .await?;
        Ok(project(&row)?)
    }

    /// Gets a project.
    pub async fn get_project(&self, id: Uuid) -> Result<Project> {
        let row = sqlx::query("SELECT * FROM projects WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("project"))?;
        Ok(project(&row)?)
    }

    /// Updates a project if `expected_version` matches.
    pub async fn update_project(
        &self,
        id: Uuid,
        expected_version: i32,
        input: NamedFields,
    ) -> Result<Project> {
        let mut conn = self.pool.acquire().await?;
        let row = sqlx::query(
            "UPDATE projects SET name = $3, description = COALESCE($4, description),
                 version = version + 1, updated_at = now()
             WHERE id = $1 AND version = $2 RETURNING *",
        )
        .bind(id)
        .bind(expected_version)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_optional(&mut *conn)
        .await?;
        match row {
            Some(row) => Ok(project(&row)?),
            None => Err(missing_or_conflict(&mut conn, "projects", "project", id).await),
        }
    }

    /// Deletes a project and everything in it.
    pub async fn delete_project(&self, id: Uuid) -> Result<()> {
        let done = sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(StorageError::NotFound("project"));
        }
        Ok(())
    }

    /// Lists components of a project.
    pub async fn list_components(
        &self,
        project_id: Uuid,
        page: PageRequest,
    ) -> Result<Page<Component>> {
        self.get_project(project_id).await?;
        let (ts, id) = page.cursor_parts();
        let sql = format!(
            "SELECT * FROM components WHERE project_id = $1 AND {} ORDER BY created_at, id LIMIT $4",
            keyset(2, 3)
        );
        let rows = sqlx::query(&sql)
            .bind(project_id)
            .bind(ts)
            .bind(id)
            .bind(page.limit + 1)
            .fetch_all(&self.pool)
            .await?;
        let items = rows.iter().map(component).collect::<Result<Vec<_>, _>>()?;
        Ok(into_page(items, page.limit, |c| {
            (c.audit.created_at, c.audit.id)
        }))
    }

    /// Creates a component.
    pub async fn create_component(
        &self,
        project_id: Uuid,
        input: NamedFields,
    ) -> Result<Component> {
        self.get_project(project_id).await?;
        let row = sqlx::query(
            "INSERT INTO components (id, project_id, name, description) VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(project_id)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(&self.pool)
        .await?;
        Ok(component(&row)?)
    }

    /// Gets a component.
    pub async fn get_component(&self, id: Uuid) -> Result<Component> {
        let row = sqlx::query("SELECT * FROM components WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("component"))?;
        Ok(component(&row)?)
    }

    /// Updates a component if `expected_version` matches.
    pub async fn update_component(
        &self,
        id: Uuid,
        expected_version: i32,
        input: NamedFields,
    ) -> Result<Component> {
        let mut conn = self.pool.acquire().await?;
        let row = sqlx::query(
            "UPDATE components SET name = $3, description = COALESCE($4, description),
                 version = version + 1, updated_at = now()
             WHERE id = $1 AND version = $2 RETURNING *",
        )
        .bind(id)
        .bind(expected_version)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_optional(&mut *conn)
        .await?;
        match row {
            Some(row) => Ok(component(&row)?),
            None => Err(missing_or_conflict(&mut conn, "components", "component", id).await),
        }
    }

    /// Deletes a component; fails with `Conflict` if it has features and `cascade` is false.
    pub async fn delete_component(&self, id: Uuid, cascade: bool) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let exists = sqlx::query("SELECT 1 FROM components WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Err(StorageError::NotFound("component"));
        }
        if !cascade {
            let has_features =
                sqlx::query("SELECT 1 FROM features WHERE component_id = $1 LIMIT 1")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if has_features.is_some() {
                return Err(StorageError::Conflict(
                    "component has features; use cascade=true to delete them".into(),
                ));
            }
        }
        sqlx::query("DELETE FROM components WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Lists features of a component, optionally filtered by tag.
    pub async fn list_features(
        &self,
        component_id: Uuid,
        tag: Option<&str>,
        page: PageRequest,
    ) -> Result<Page<Feature>> {
        self.get_component(component_id).await?;
        let (ts, id) = page.cursor_parts();
        let sql = format!(
            "SELECT * FROM features
             WHERE component_id = $1 AND ($2::text IS NULL OR $2 = ANY(tags)) AND {}
             ORDER BY created_at, id LIMIT $5",
            keyset(3, 4)
        );
        let rows = sqlx::query(&sql)
            .bind(component_id)
            .bind(tag)
            .bind(ts)
            .bind(id)
            .bind(page.limit + 1)
            .fetch_all(&self.pool)
            .await?;
        let items = rows.iter().map(feature).collect::<Result<Vec<_>, _>>()?;
        Ok(into_page(items, page.limit, |f| {
            (f.audit.created_at, f.audit.id)
        }))
    }

    /// Creates a feature.
    pub async fn create_feature(
        &self,
        component_id: Uuid,
        input: FeatureFields,
    ) -> Result<Feature> {
        self.get_component(component_id).await?;
        let row = sqlx::query(
            "INSERT INTO features (id, component_id, name, description, scenario_description, tags)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(component_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.scenario_description)
        .bind(input.tags.unwrap_or_default())
        .fetch_one(&self.pool)
        .await?;
        Ok(feature(&row)?)
    }

    /// Gets a feature.
    pub async fn get_feature(&self, id: Uuid) -> Result<Feature> {
        let row = sqlx::query("SELECT * FROM features WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("feature"))?;
        Ok(feature(&row)?)
    }

    /// Project a feature belongs to.
    pub async fn feature_project_id(&self, feature_id: Uuid) -> Result<Uuid> {
        sqlx::query_scalar(
            "SELECT c.project_id FROM features f JOIN components c ON c.id = f.component_id
             WHERE f.id = $1",
        )
        .bind(feature_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound("feature"))
    }

    /// Updates a feature if `expected_version` matches.
    pub async fn update_feature(
        &self,
        id: Uuid,
        expected_version: i32,
        input: FeatureFields,
    ) -> Result<Feature> {
        let mut conn = self.pool.acquire().await?;
        let row = sqlx::query(
            "UPDATE features SET name = $3, description = COALESCE($4, description),
                 scenario_description = COALESCE($5, scenario_description),
                 tags = COALESCE($6, tags), version = version + 1, updated_at = now()
             WHERE id = $1 AND version = $2 RETURNING *",
        )
        .bind(id)
        .bind(expected_version)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.scenario_description)
        .bind(&input.tags)
        .fetch_optional(&mut *conn)
        .await?;
        match row {
            Some(row) => Ok(feature(&row)?),
            None => Err(missing_or_conflict(&mut conn, "features", "feature", id).await),
        }
    }

    /// Deletes a feature; fails with `Conflict` if it has models or test cases and `cascade` is false.
    pub async fn delete_feature(&self, id: Uuid, cascade: bool) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let exists = sqlx::query("SELECT 1 FROM features WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Err(StorageError::NotFound("feature"));
        }
        if !cascade {
            let non_empty: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM models WHERE feature_id = $1)
                     OR EXISTS (SELECT 1 FROM test_cases WHERE feature_id = $1)",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            if non_empty {
                return Err(StorageError::Conflict(
                    "feature has models or test cases; use cascade=true to delete them".into(),
                ));
            }
        }
        sqlx::query("DELETE FROM features WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Moves a feature (with its models and test cases) to another component.
    pub async fn move_feature(&self, id: Uuid, component_id: Uuid) -> Result<Feature> {
        self.get_component(component_id).await?;
        let row = sqlx::query(
            "UPDATE features SET component_id = $2, version = version + 1, updated_at = now()
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(component_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound("feature"))?;
        Ok(feature(&row)?)
    }

    /// Component > Feature > Model tree of a project.
    pub async fn project_tree(&self, project_id: Uuid) -> Result<Vec<TreeComponent>> {
        self.get_project(project_id).await?;
        let components = sqlx::query(
            "SELECT id, name FROM components WHERE project_id = $1 ORDER BY created_at, id",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        let features = sqlx::query(
            "SELECT f.id, f.name, f.component_id FROM features f
             JOIN components c ON c.id = f.component_id
             WHERE c.project_id = $1 ORDER BY f.created_at, f.id",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        let models = sqlx::query(
            "SELECT m.id, m.name, m.status, m.feature_id FROM models m
             JOIN features f ON f.id = m.feature_id
             JOIN components c ON c.id = f.component_id
             WHERE c.project_id = $1 ORDER BY m.created_at, m.id",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        let mut models_by_feature: HashMap<Uuid, Vec<TreeModel>> = HashMap::new();
        for row in &models {
            models_by_feature
                .entry(row.try_get("feature_id")?)
                .or_default()
                .push(TreeModel {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    status: parse_col(row, "status")?,
                });
        }
        let mut features_by_component: HashMap<Uuid, Vec<TreeFeature>> = HashMap::new();
        for row in &features {
            let id: Uuid = row.try_get("id")?;
            features_by_component
                .entry(row.try_get("component_id")?)
                .or_default()
                .push(TreeFeature {
                    id,
                    name: row.try_get("name")?,
                    models: models_by_feature.remove(&id).unwrap_or_default(),
                });
        }
        components
            .iter()
            .map(|row| {
                let id: Uuid = row.try_get("id")?;
                Ok(TreeComponent {
                    id,
                    name: row.try_get("name")?,
                    features: features_by_component.remove(&id).unwrap_or_default(),
                })
            })
            .collect()
    }

    /// Case-insensitive substring search over names and descriptions.
    pub async fn search(
        &self,
        project_id: Uuid,
        q: &str,
        types: &[SearchType],
    ) -> Result<Vec<SearchHit>> {
        self.get_project(project_id).await?;
        let want = |t| types.is_empty() || types.contains(&t);
        let pattern = like_pattern(q);
        let mut hits = Vec::new();
        let queries = [
            (
                SearchType::Component,
                "SELECT c.id, c.name, c.description AS snippet FROM components c
                 WHERE c.project_id = $1 AND (c.name ILIKE $2 OR c.description ILIKE $2)
                 ORDER BY c.name LIMIT $3",
            ),
            (
                SearchType::Feature,
                "SELECT f.id, f.name, COALESCE(f.description, f.scenario_description) AS snippet
                 FROM features f JOIN components c ON c.id = f.component_id
                 WHERE c.project_id = $1 AND (f.name ILIKE $2 OR f.description ILIKE $2
                     OR f.scenario_description ILIKE $2
                     OR EXISTS (SELECT 1 FROM unnest(f.tags) t WHERE t ILIKE $2))
                 ORDER BY f.name LIMIT $3",
            ),
            (
                SearchType::Model,
                "SELECT m.id, m.name, m.description AS snippet
                 FROM models m JOIN features f ON f.id = m.feature_id
                 JOIN components c ON c.id = f.component_id
                 WHERE c.project_id = $1 AND (m.name ILIKE $2 OR m.description ILIKE $2)
                 ORDER BY m.name LIMIT $3",
            ),
            (
                SearchType::TestCase,
                "SELECT t.id, t.name, t.description AS snippet
                 FROM test_cases t JOIN features f ON f.id = t.feature_id
                 JOIN components c ON c.id = f.component_id
                 WHERE c.project_id = $1 AND (t.name ILIKE $2 OR t.description ILIKE $2)
                 ORDER BY t.name LIMIT $3",
            ),
        ];
        for (hit_type, sql) in queries {
            if !want(hit_type) {
                continue;
            }
            let rows = sqlx::query(sql)
                .bind(project_id)
                .bind(&pattern)
                .bind(SEARCH_LIMIT)
                .fetch_all(&self.pool)
                .await?;
            for row in rows {
                let snippet: Option<String> = row.try_get("snippet")?;
                hits.push(SearchHit {
                    hit_type,
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    snippet: snippet.map(|s| s.chars().take(200).collect()),
                });
            }
        }
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::like_pattern;

    #[test]
    fn like_pattern_escapes_wildcards() {
        assert_eq!(like_pattern("a%b_c\\"), "%a\\%b\\_c\\\\%");
    }
}
