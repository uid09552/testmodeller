//! Threat models (FR-060..FR-076). See docs/specification/09-threat-modeling.md.
//!
//! Reads load the whole threat model as one aggregate; computed values are
//! derived from it in the domain. Every write is scoped by `threat_model_id`, so
//! an item of another threat model is reported as not found.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::postgres::PgRow;
use sqlx::{PgConnection, Row};
use tm_domain::threat::arch::Architecture;
use tm_domain::threat::assurance::{Evidence, EvidenceFields};
use tm_domain::threat::rules::Candidate;
use tm_domain::threat::templates::NewSecurityTest;
use tm_domain::threat::{
    ArchComponent, ArchComponentFields, Asset, AssetFields, Control, ControlFields,
    EvidenceOwnerKind, Flow, FlowFields, Provenance, Rating, SecurityTest, SecurityTestFields,
    SecurityTestOrigin, SecurityTestOutcome, SecurityTestResult, SecurityTestStatus,
    SecurityTestStep, TargetKind, TemplateSource, Threat, ThreatFields, ThreatModel,
    ThreatModelFields, ThreatOrigin, ThreatStatus, ThreatTarget, Thresholds, Zone, ZoneFields,
};
use uuid::Uuid;

use crate::{audit, parse_col, parse_opt_col, Result, StorageError, Store};

/// A threat model with everything in it.
#[derive(Debug, Clone)]
pub struct ThreatModelAggregate {
    /// The threat model.
    pub model: ThreatModel,
    /// Zones, components, assets, flows.
    pub arch: Architecture,
    /// Threats, by number.
    pub threats: Vec<Threat>,
    /// Controls, by number.
    pub controls: Vec<Control>,
    /// Security tests, by number.
    pub tests: Vec<SecurityTest>,
    /// Evidence of controls and test results.
    pub evidence: Vec<Evidence>,
    /// Every rule key ever used.
    pub rule_keys: HashSet<String>,
}

fn unique_violation(e: sqlx::Error, message: impl FnOnce() -> String) -> StorageError {
    match &e {
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
            StorageError::Conflict(message())
        }
        _ => e.into(),
    }
}

fn threat_model(row: &PgRow) -> Result<ThreatModel> {
    let thresholds: serde_json::Value = row.try_get("thresholds")?;
    Ok(ThreatModel {
        audit: audit(row)?,
        project_id: row.try_get("project_id")?,
        fields: ThreatModelFields {
            name: row.try_get("name")?,
            scope: row.try_get("scope")?,
            objectives: row.try_get("objectives")?,
            assumptions: row.try_get("assumptions")?,
            exclusions: row.try_get("exclusions")?,
            ai_enabled: row.try_get("ai_enabled")?,
        },
        thresholds: serde_json::from_value::<Thresholds>(thresholds)?,
        layout: row.try_get("layout")?,
        layout_version: row.try_get("layout_version")?,
    })
}

fn provenance(row: &PgRow) -> Result<Provenance, sqlx::Error> {
    Ok(Provenance {
        origin: parse_col(row, "origin")?,
        source: row.try_get("ai_source")?,
    })
}

fn u8_col(row: &PgRow, col: &str) -> Result<u8, sqlx::Error> {
    let v: i16 = row.try_get(col)?;
    Ok(u8::try_from(v).unwrap_or(0))
}

fn rating(row: &PgRow, l: &str, i: &str) -> Result<Option<Rating>, sqlx::Error> {
    let a: Option<i16> = row.try_get(l)?;
    let b: Option<i16> = row.try_get(i)?;
    Ok(match (a, b) {
        (Some(a), Some(b)) => Some(Rating {
            likelihood: u8::try_from(a).unwrap_or(1),
            impact: u8::try_from(b).unwrap_or(1),
        }),
        _ => None,
    })
}

fn rating_parts(r: Option<Rating>) -> (Option<i16>, Option<i16>) {
    r.map_or((None, None), |r| {
        (Some(i16::from(r.likelihood)), Some(i16::from(r.impact)))
    })
}

/// Groups `(parent, child)` rows into a map of ordered child lists.
fn group(rows: Vec<(Uuid, Uuid)>) -> HashMap<Uuid, Vec<Uuid>> {
    let mut m: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (p, c) in rows {
        m.entry(p).or_default().push(c);
    }
    m
}

async fn pairs(conn: &mut PgConnection, sql: &str, tm: Uuid) -> Result<Vec<(Uuid, Uuid)>> {
    Ok(sqlx::query_as(sql).bind(tm).fetch_all(conn).await?)
}

/// Fails with `Invalid` unless every id is a row of `table` in the threat model.
async fn ensure_in(
    conn: &mut PgConnection,
    table: &str,
    tm: Uuid,
    ids: &[Uuid],
    field: &str,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let distinct: HashSet<&Uuid> = ids.iter().collect();
    let sql = format!("SELECT count(*) FROM {table} WHERE threat_model_id = $1 AND id = ANY($2)");
    let n: i64 = sqlx::query_scalar(&sql)
        .bind(tm)
        .bind(ids)
        .fetch_one(&mut *conn)
        .await?;
    if usize::try_from(n).unwrap_or(0) != distinct.len() {
        return Err(StorageError::Invalid(format!(
            "{field}: every id must belong to this threat model"
        )));
    }
    Ok(())
}

/// Replaces the children of `parent` in a link table.
async fn replace_links(
    conn: &mut PgConnection,
    table: &str,
    parent_col: &str,
    child_col: &str,
    parent: Uuid,
    ids: &[Uuid],
) -> Result<()> {
    sqlx::query(&format!("DELETE FROM {table} WHERE {parent_col} = $1"))
        .bind(parent)
        .execute(&mut *conn)
        .await?;
    let mut seen = HashSet::new();
    for (pos, id) in ids.iter().enumerate() {
        if !seen.insert(id) {
            continue;
        }
        sqlx::query(&format!(
            "INSERT INTO {table} ({parent_col}, {child_col}, position) VALUES ($1, $2, $3)"
        ))
        .bind(parent)
        .bind(id)
        .bind(i32::try_from(pos).unwrap_or(i32::MAX))
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// `NotFound` or `VersionMismatch` after an update in a threat model touched no row.
async fn missing_or_conflict_in(
    conn: &mut PgConnection,
    table: &'static str,
    entity: &'static str,
    tm: Uuid,
    id: Uuid,
) -> StorageError {
    let sql = format!("SELECT 1 FROM {table} WHERE id = $1 AND threat_model_id = $2");
    match sqlx::query(&sql)
        .bind(id)
        .bind(tm)
        .fetch_optional(conn)
        .await
    {
        Ok(Some(_)) => StorageError::VersionMismatch,
        Ok(None) => StorageError::NotFound(entity),
        Err(e) => e.into(),
    }
}

async fn delete_in(
    conn: &mut PgConnection,
    table: &'static str,
    entity: &'static str,
    tm: Uuid,
    id: Uuid,
) -> Result<()> {
    let sql = format!("DELETE FROM {table} WHERE id = $1 AND threat_model_id = $2");
    let done = sqlx::query(&sql).bind(id).bind(tm).execute(conn).await?;
    if done.rows_affected() == 0 {
        return Err(StorageError::NotFound(entity));
    }
    Ok(())
}

async fn next_number(conn: &mut PgConnection, tm: Uuid, counter: &str) -> Result<i32> {
    let sql = format!(
        "UPDATE threat_models SET {counter} = {counter} + 1 WHERE id = $1 RETURNING {counter} - 1"
    );
    sqlx::query_scalar(&sql)
        .bind(tm)
        .fetch_optional(conn)
        .await?
        .ok_or(StorageError::NotFound("threat model"))
}

async fn check_project_link(
    conn: &mut PgConnection,
    tm: Uuid,
    f: &ArchComponentFields,
) -> Result<()> {
    if let Some(c) = f.linked_component_id {
        let ok = sqlx::query(
            "SELECT 1 FROM components c JOIN threat_models t ON t.project_id = c.project_id
             WHERE c.id = $1 AND t.id = $2",
        )
        .bind(c)
        .bind(tm)
        .fetch_optional(&mut *conn)
        .await?;
        if ok.is_none() {
            return Err(StorageError::Invalid(
                "linkedComponentId: must be a component of the same project".into(),
            ));
        }
    }
    if let Some(feat) = f.linked_feature_id {
        let ok = sqlx::query(
            "SELECT 1 FROM features f JOIN components c ON c.id = f.component_id
             JOIN threat_models t ON t.project_id = c.project_id
             WHERE f.id = $1 AND t.id = $2",
        )
        .bind(feat)
        .bind(tm)
        .fetch_optional(&mut *conn)
        .await?;
        if ok.is_none() {
            return Err(StorageError::Invalid(
                "linkedFeatureId: must be a feature of the same project".into(),
            ));
        }
    }
    if let Some(z) = f.zone_id {
        ensure_in(conn, "tm_zones", tm, &[z], "zoneId").await?;
    }
    Ok(())
}

impl Store {
    // ---- Threat model ----

    /// Creates the threat model of a project; `Conflict` if it has one.
    pub async fn create_threat_model(
        &self,
        project_id: Uuid,
        fields: ThreatModelFields,
        thresholds: Thresholds,
    ) -> Result<ThreatModel> {
        let tenant: String = sqlx::query_scalar("SELECT tenant_id FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("project"))?;
        let row = sqlx::query(
            "INSERT INTO threat_models (id, project_id, tenant_id, name, scope, objectives,
                 assumptions, exclusions, thresholds, ai_enabled)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(project_id)
        .bind(tenant)
        .bind(&fields.name)
        .bind(&fields.scope)
        .bind(&fields.objectives)
        .bind(&fields.assumptions)
        .bind(&fields.exclusions)
        .bind(serde_json::to_value(&thresholds)?)
        .bind(fields.ai_enabled)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| unique_violation(e, || "the project already has a threat model".into()))?;
        threat_model(&row)
    }

    /// The threat model of a project.
    pub async fn threat_model_of_project(&self, project_id: Uuid) -> Result<ThreatModel> {
        let row = sqlx::query("SELECT * FROM threat_models WHERE project_id = $1")
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("threat model"))?;
        threat_model(&row)
    }

    /// Gets a threat model.
    pub async fn get_threat_model(&self, id: Uuid) -> Result<ThreatModel> {
        let row = sqlx::query("SELECT * FROM threat_models WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("threat model"))?;
        threat_model(&row)
    }

    /// Replaces the scope fields and thresholds if `expected_version` matches.
    pub async fn update_threat_model(
        &self,
        id: Uuid,
        expected_version: i32,
        fields: ThreatModelFields,
        thresholds: Thresholds,
    ) -> Result<ThreatModel> {
        let mut conn = self.pool.acquire().await?;
        let row = sqlx::query(
            "UPDATE threat_models SET name = $3, scope = $4, objectives = $5, assumptions = $6,
                 exclusions = $7, thresholds = $8, ai_enabled = $9, version = version + 1,
                 updated_at = now()
             WHERE id = $1 AND version = $2 RETURNING *",
        )
        .bind(id)
        .bind(expected_version)
        .bind(&fields.name)
        .bind(&fields.scope)
        .bind(&fields.objectives)
        .bind(&fields.assumptions)
        .bind(&fields.exclusions)
        .bind(serde_json::to_value(&thresholds)?)
        .bind(fields.ai_enabled)
        .fetch_optional(&mut *conn)
        .await?;
        match row {
            Some(row) => threat_model(&row),
            None => Err(
                crate::missing_or_conflict(&mut conn, "threat_models", "threat model", id).await,
            ),
        }
    }

    /// Replaces the diagram layout if `expected_layout_version` matches;
    /// returns the new layout version.
    pub async fn save_threat_model_layout(
        &self,
        id: Uuid,
        expected_layout_version: i32,
        layout: Option<serde_json::Value>,
    ) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE threat_models SET layout = $3, layout_version = layout_version + 1
             WHERE id = $1 AND layout_version = $2 RETURNING layout_version",
        )
        .bind(id)
        .bind(expected_layout_version)
        .bind(layout)
        .fetch_optional(&self.pool)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => {
                self.get_threat_model(id).await?;
                Err(StorageError::VersionMismatch)
            }
        }
    }

    /// Deletes a threat model and everything in it.
    pub async fn delete_threat_model(&self, id: Uuid) -> Result<()> {
        let done = sqlx::query("DELETE FROM threat_models WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(StorageError::NotFound("threat model"));
        }
        Ok(())
    }

    /// Loads a threat model with everything in it.
    pub async fn load_threat_model(&self, id: Uuid) -> Result<ThreatModelAggregate> {
        let mut conn = self.pool.acquire().await?;
        let model = {
            let row = sqlx::query("SELECT * FROM threat_models WHERE id = $1")
                .bind(id)
                .fetch_optional(&mut *conn)
                .await?
                .ok_or(StorageError::NotFound("threat model"))?;
            threat_model(&row)?
        };

        let zones = sqlx::query(
            "SELECT * FROM tm_zones WHERE threat_model_id = $1 ORDER BY created_at, id",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(|r| {
            Ok(Zone {
                audit: audit(r)?,
                provenance: provenance(r)?,
                fields: ZoneFields {
                    name: r.try_get("name")?,
                    description: r.try_get("description")?,
                    kind: parse_col(r, "kind")?,
                    trust_level: u8_col(r, "trust_level")?,
                },
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

        let mut comp_assets = group(
            pairs(
                &mut conn,
                "SELECT l.component_id, l.asset_id FROM tm_component_assets l
                 JOIN tm_components c ON c.id = l.component_id
                 WHERE c.threat_model_id = $1 ORDER BY l.position",
                id,
            )
            .await?,
        );
        let components = sqlx::query(
            "SELECT * FROM tm_components WHERE threat_model_id = $1 ORDER BY created_at, id",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(|r| {
            let cid: Uuid = r.try_get("id")?;
            Ok(ArchComponent {
                audit: audit(r)?,
                provenance: provenance(r)?,
                fields: ArchComponentFields {
                    name: r.try_get("name")?,
                    description: r.try_get("description")?,
                    component_type: parse_col(r, "component_type")?,
                    owner: r.try_get("owner")?,
                    technology: r.try_get("technology")?,
                    criticality: parse_opt_col(r, "criticality")?,
                    internet_facing: r.try_get("internet_facing")?,
                    zone_id: r.try_get("zone_id")?,
                    linked_component_id: r.try_get("linked_component_id")?,
                    linked_feature_id: r.try_get("linked_feature_id")?,
                    asset_ids: comp_assets.remove(&cid).unwrap_or_default(),
                },
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

        let assets = sqlx::query(
            "SELECT * FROM tm_assets WHERE threat_model_id = $1 ORDER BY created_at, id",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(|r| {
            Ok(Asset {
                audit: audit(r)?,
                provenance: provenance(r)?,
                fields: AssetFields {
                    name: r.try_get("name")?,
                    description: r.try_get("description")?,
                    category: parse_col(r, "category")?,
                    owner: r.try_get("owner")?,
                    confidentiality: parse_opt_col(r, "confidentiality")?,
                    integrity: parse_opt_col(r, "integrity")?,
                    availability: parse_opt_col(r, "availability")?,
                    personal_data: r.try_get("personal_data")?,
                    classification: parse_opt_col(r, "classification")?,
                    data_location: r.try_get("data_location")?,
                    retention: r.try_get("retention")?,
                    encryption_at_rest: parse_col(r, "encryption_at_rest")?,
                },
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

        let mut flow_assets = group(
            pairs(
                &mut conn,
                "SELECT l.flow_id, l.asset_id FROM tm_flow_assets l
                 JOIN tm_flows f ON f.id = l.flow_id
                 WHERE f.threat_model_id = $1 ORDER BY l.position",
                id,
            )
            .await?,
        );
        let flows = sqlx::query(
            "SELECT * FROM tm_flows WHERE threat_model_id = $1 ORDER BY created_at, id",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(|r| {
            let fid: Uuid = r.try_get("id")?;
            Ok(Flow {
                audit: audit(r)?,
                provenance: provenance(r)?,
                fields: FlowFields {
                    source_id: r.try_get("source_id")?,
                    target_id: r.try_get("target_id")?,
                    relationship: parse_col(r, "relationship")?,
                    label: r.try_get("label")?,
                    protocol: r.try_get("protocol")?,
                    bidirectional: r.try_get("bidirectional")?,
                    authentication: r.try_get("authentication")?,
                    encrypted: parse_col(r, "encrypted")?,
                    asset_ids: flow_assets.remove(&fid).unwrap_or_default(),
                    data_classification: parse_opt_col(r, "data_classification")?,
                    authorization_model: r.try_get("authorization_model")?,
                    entry_point: r.try_get("entry_point")?,
                },
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

        // Threats and their links.
        let target_rows: Vec<(Uuid, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT l.threat_id, l.component_id, l.flow_id FROM tm_threat_targets l
             JOIN tm_threats t ON t.id = l.threat_id
             WHERE t.threat_model_id = $1 ORDER BY l.position",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await?;
        let mut targets: HashMap<Uuid, Vec<ThreatTarget>> = HashMap::new();
        for (t, c, f) in target_rows {
            let target = match (c, f) {
                (Some(c), _) => ThreatTarget {
                    kind: TargetKind::Component,
                    id: c,
                },
                (None, Some(f)) => ThreatTarget {
                    kind: TargetKind::Flow,
                    id: f,
                },
                (None, None) => continue,
            };
            targets.entry(t).or_default().push(target);
        }
        let mut threat_assets = group(
            pairs(
                &mut conn,
                "SELECT l.threat_id, l.asset_id FROM tm_threat_assets l
                 JOIN tm_threats t ON t.id = l.threat_id
                 WHERE t.threat_model_id = $1 ORDER BY l.position",
                id,
            )
            .await?,
        );
        let threat_control_rows = pairs(
            &mut conn,
            "SELECT l.threat_id, l.control_id FROM tm_threat_controls l
             JOIN tm_threats t ON t.id = l.threat_id
             WHERE t.threat_model_id = $1 ORDER BY l.position, l.control_id",
            id,
        )
        .await?;
        let mut control_threats =
            group(threat_control_rows.iter().map(|(t, c)| (*c, *t)).collect());
        let mut threat_controls = group(threat_control_rows);
        let threats =
            sqlx::query("SELECT * FROM tm_threats WHERE threat_model_id = $1 ORDER BY number")
                .bind(id)
                .fetch_all(&mut *conn)
                .await?
                .iter()
                .map(|r| {
                    let tid: Uuid = r.try_get("id")?;
                    let explanation: Option<serde_json::Value> = r.try_get("explanation")?;
                    Ok(Threat {
                        audit: audit(r)?,
                        number: r.try_get("number")?,
                        fields: ThreatFields {
                            title: r.try_get("title")?,
                            category: parse_col(r, "category")?,
                            scenario: r.try_get("scenario")?,
                            attacker_objective: r.try_get("attacker_objective")?,
                            preconditions: r.try_get("preconditions")?,
                            consequences: r.try_get("consequences")?,
                            targets: targets.remove(&tid).unwrap_or_default(),
                            asset_ids: threat_assets.remove(&tid).unwrap_or_default(),
                            control_ids: threat_controls.remove(&tid).unwrap_or_default(),
                            status: parse_col(r, "status")?,
                            dismiss_reason: r.try_get("dismiss_reason")?,
                            inherent: rating(r, "inherent_likelihood", "inherent_impact")?,
                            current: rating(r, "current_likelihood", "current_impact")?,
                            residual: rating(r, "residual_likelihood", "residual_impact")?,
                            threat_source: r.try_get("threat_source")?,
                            detection: r.try_get("detection")?,
                            existing_mitigations: r.try_get("existing_mitigations")?,
                            confidence: parse_opt_col(r, "confidence")?,
                            treatment: parse_col(r, "treatment")?,
                            risk_owner: r.try_get("risk_owner")?,
                            due_date: r.try_get("due_date")?,
                            acceptance_rationale: r.try_get("acceptance_rationale")?,
                            accepted_by: r.try_get("accepted_by")?,
                            acceptance_expires: r.try_get("acceptance_expires")?,
                        },
                        accepted_on: r.try_get("accepted_on")?,
                        source_model: r.try_get("ai_source")?,
                        origin: parse_col(r, "origin")?,
                        explanation: explanation
                            .map(serde_json::from_value)
                            .transpose()
                            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
                    })
                })
                .collect::<Result<Vec<_>, sqlx::Error>>()?;

        let mut control_components = group(
            pairs(
                &mut conn,
                "SELECT l.control_id, l.component_id FROM tm_control_components l
                 JOIN tm_controls c ON c.id = l.control_id
                 WHERE c.threat_model_id = $1 ORDER BY l.position",
                id,
            )
            .await?,
        );
        let controls =
            sqlx::query("SELECT * FROM tm_controls WHERE threat_model_id = $1 ORDER BY number")
                .bind(id)
                .fetch_all(&mut *conn)
                .await?
                .iter()
                .map(|r| {
                    let cid: Uuid = r.try_get("id")?;
                    Ok(Control {
                        audit: audit(r)?,
                        provenance: provenance(r)?,
                        number: r.try_get("number")?,
                        fields: ControlFields {
                            name: r.try_get("name")?,
                            description: r.try_get("description")?,
                            control_type: parse_col(r, "control_type")?,
                            category: parse_col(r, "category")?,
                            status: parse_col(r, "status")?,
                            owner: r.try_get("owner")?,
                            acceptance_criteria: r.try_get("acceptance_criteria")?,
                            review_by: r.try_get("review_by")?,
                            component_ids: control_components.remove(&cid).unwrap_or_default(),
                            threat_ids: control_threats.remove(&cid).unwrap_or_default(),
                        },
                        verified_at: r.try_get("verified_at")?,
                    })
                })
                .collect::<Result<Vec<_>, sqlx::Error>>()?;

        let mut test_threats = group(
            pairs(
                &mut conn,
                "SELECT l.security_test_id, l.threat_id FROM security_test_threats l
                 JOIN security_tests s ON s.id = l.security_test_id
                 WHERE s.threat_model_id = $1 ORDER BY l.position",
                id,
            )
            .await?,
        );
        let mut test_controls = group(
            pairs(
                &mut conn,
                "SELECT l.security_test_id, l.control_id FROM security_test_controls l
                 JOIN security_tests s ON s.id = l.security_test_id
                 WHERE s.threat_model_id = $1 ORDER BY l.position",
                id,
            )
            .await?,
        );
        let mut latest: HashMap<Uuid, SecurityTestResult> = sqlx::query(
            "SELECT DISTINCT ON (r.security_test_id) r.* FROM security_test_results r
             JOIN security_tests s ON s.id = r.security_test_id
             WHERE s.threat_model_id = $1
             ORDER BY r.security_test_id, r.executed_at DESC, r.recorded_at DESC",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(|r| test_result(r).map(|x| (x.security_test_id, x)))
        .collect::<Result<_, sqlx::Error>>()?;
        let tests =
            sqlx::query("SELECT * FROM security_tests WHERE threat_model_id = $1 ORDER BY number")
                .bind(id)
                .fetch_all(&mut *conn)
                .await?
                .iter()
                .map(|r| {
                    let sid: Uuid = r.try_get("id")?;
                    let steps: serde_json::Value = r.try_get("steps")?;
                    let template_id: Option<String> = r.try_get("template_id")?;
                    let source = match (
                        template_id,
                        r.try_get::<Option<Uuid>, _>("source_threat_id")?,
                    ) {
                        (Some(template_id), Some(threat_id)) => Some(TemplateSource {
                            template_id,
                            template_version: r
                                .try_get::<Option<i32>, _>("template_version")?
                                .unwrap_or(1),
                            threat_id,
                            control_id: r.try_get("source_control_id")?,
                        }),
                        _ => None,
                    };
                    Ok(SecurityTest {
                        audit: audit(r)?,
                        number: r.try_get("number")?,
                        fields: SecurityTestFields {
                            title: r.try_get("title")?,
                            objective: r.try_get("objective")?,
                            preconditions: r.try_get("preconditions")?,
                            steps: serde_json::from_value::<Vec<SecurityTestStep>>(steps)
                                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
                            test_type: parse_col(r, "test_type")?,
                            status: parse_col(r, "status")?,
                            threat_ids: test_threats.remove(&sid).unwrap_or_default(),
                            control_ids: test_controls.remove(&sid).unwrap_or_default(),
                        },
                        origin: parse_col(r, "origin")?,
                        source,
                        last_result: latest.remove(&sid),
                        source_model: r.try_get("ai_source")?,
                    })
                })
                .collect::<Result<Vec<_>, sqlx::Error>>()?;

        let evidence = sqlx::query(
            "SELECT * FROM tm_evidence WHERE threat_model_id = $1 ORDER BY evidence_date DESC, created_at DESC, id",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(evidence_row)
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

        let rule_keys: Vec<String> =
            sqlx::query_scalar("SELECT rule_key FROM tm_rule_keys WHERE threat_model_id = $1")
                .bind(id)
                .fetch_all(&mut *conn)
                .await?;

        Ok(ThreatModelAggregate {
            model,
            arch: Architecture {
                zones,
                components,
                assets,
                flows,
            },
            threats,
            controls,
            tests,
            evidence,
            rule_keys: rule_keys.into_iter().collect(),
        })
    }

    // ---- Zones ----

    /// Creates a zone.
    pub async fn create_tm_zone(&self, tm: Uuid, f: &ZoneFields) -> Result<Uuid> {
        let mut tx = self.pool.begin().await?;
        let id = Self::insert_zone(&mut tx, tm, f).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn insert_zone(conn: &mut PgConnection, tm: Uuid, f: &ZoneFields) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tm_zones (id, threat_model_id, name, description, kind, trust_level)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(tm)
        .bind(&f.name)
        .bind(&f.description)
        .bind(f.kind.as_str())
        .bind(i16::from(f.trust_level))
        .execute(&mut *conn)
        .await
        .map_err(|e| unique_violation(e, || format!("a zone named '{}' already exists", f.name)))?;
        Ok(id)
    }

    /// Replaces a zone.
    pub async fn update_tm_zone(
        &self,
        tm: Uuid,
        id: Uuid,
        version: i32,
        f: &ZoneFields,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let done = sqlx::query(
            "UPDATE tm_zones SET name = $4, description = $5, kind = $6, trust_level = $7,
                 version = version + 1, updated_at = now()
             WHERE id = $1 AND threat_model_id = $2 AND version = $3",
        )
        .bind(id)
        .bind(tm)
        .bind(version)
        .bind(&f.name)
        .bind(&f.description)
        .bind(f.kind.as_str())
        .bind(i16::from(f.trust_level))
        .execute(&mut *conn)
        .await
        .map_err(|e| unique_violation(e, || format!("a zone named '{}' already exists", f.name)))?;
        if done.rows_affected() == 0 {
            return Err(missing_or_conflict_in(&mut conn, "tm_zones", "zone", tm, id).await);
        }
        Ok(())
    }

    /// Deletes a zone; its components are left without a zone.
    pub async fn delete_tm_zone(&self, tm: Uuid, id: Uuid) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        delete_in(&mut conn, "tm_zones", "zone", tm, id).await
    }

    // ---- Components ----

    /// Creates an architecture component.
    pub async fn create_tm_component(&self, tm: Uuid, f: &ArchComponentFields) -> Result<Uuid> {
        let mut tx = self.pool.begin().await?;
        let id = Self::insert_component(&mut tx, tm, f).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn insert_component(
        conn: &mut PgConnection,
        tm: Uuid,
        f: &ArchComponentFields,
    ) -> Result<Uuid> {
        check_project_link(conn, tm, f).await?;
        ensure_in(conn, "tm_assets", tm, &f.asset_ids, "assetIds").await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tm_components (id, threat_model_id, name, description, component_type,
                 owner, technology, criticality, internet_facing, zone_id, linked_component_id,
                 linked_feature_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id)
        .bind(tm)
        .bind(&f.name)
        .bind(&f.description)
        .bind(f.component_type.as_str())
        .bind(&f.owner)
        .bind(&f.technology)
        .bind(f.criticality.map(|c| c.as_str()))
        .bind(f.internet_facing)
        .bind(f.zone_id)
        .bind(f.linked_component_id)
        .bind(f.linked_feature_id)
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            unique_violation(e, || {
                format!("a component named '{}' already exists", f.name)
            })
        })?;
        replace_links(
            conn,
            "tm_component_assets",
            "component_id",
            "asset_id",
            id,
            &f.asset_ids,
        )
        .await?;
        Ok(id)
    }

    /// Replaces an architecture component, including its asset links.
    pub async fn update_tm_component(
        &self,
        tm: Uuid,
        id: Uuid,
        version: i32,
        f: &ArchComponentFields,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        check_project_link(&mut tx, tm, f).await?;
        ensure_in(&mut tx, "tm_assets", tm, &f.asset_ids, "assetIds").await?;
        let done = sqlx::query(
            "UPDATE tm_components SET name = $4, description = $5, component_type = $6, owner = $7,
                 technology = $8, criticality = $9, internet_facing = $10, zone_id = $11,
                 linked_component_id = $12, linked_feature_id = $13,
                 version = version + 1, updated_at = now()
             WHERE id = $1 AND threat_model_id = $2 AND version = $3",
        )
        .bind(id)
        .bind(tm)
        .bind(version)
        .bind(&f.name)
        .bind(&f.description)
        .bind(f.component_type.as_str())
        .bind(&f.owner)
        .bind(&f.technology)
        .bind(f.criticality.map(|c| c.as_str()))
        .bind(f.internet_facing)
        .bind(f.zone_id)
        .bind(f.linked_component_id)
        .bind(f.linked_feature_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            unique_violation(e, || {
                format!("a component named '{}' already exists", f.name)
            })
        })?;
        if done.rows_affected() == 0 {
            return Err(
                missing_or_conflict_in(&mut tx, "tm_components", "component", tm, id).await,
            );
        }
        replace_links(
            &mut tx,
            "tm_component_assets",
            "component_id",
            "asset_id",
            id,
            &f.asset_ids,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Deletes a component and the flows that start or end at it.
    pub async fn delete_tm_component(&self, tm: Uuid, id: Uuid) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        delete_in(&mut conn, "tm_components", "component", tm, id).await
    }

    // ---- Assets ----

    /// Creates an asset.
    pub async fn create_tm_asset(&self, tm: Uuid, f: &AssetFields) -> Result<Uuid> {
        let mut tx = self.pool.begin().await?;
        let id = Self::insert_asset(&mut tx, tm, f).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn insert_asset(conn: &mut PgConnection, tm: Uuid, f: &AssetFields) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tm_assets (id, threat_model_id, name, description, category, owner,
                 confidentiality, integrity, availability, personal_data, classification,
                 data_location, retention, encryption_at_rest)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(id)
        .bind(tm)
        .bind(&f.name)
        .bind(&f.description)
        .bind(f.category.as_str())
        .bind(&f.owner)
        .bind(f.confidentiality.map(|c| c.as_str()))
        .bind(f.integrity.map(|c| c.as_str()))
        .bind(f.availability.map(|c| c.as_str()))
        .bind(f.personal_data)
        .bind(f.classification.map(|c| c.as_str()))
        .bind(&f.data_location)
        .bind(&f.retention)
        .bind(f.encryption_at_rest.as_str())
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            unique_violation(e, || format!("an asset named '{}' already exists", f.name))
        })?;
        Ok(id)
    }

    /// Replaces an asset.
    pub async fn update_tm_asset(
        &self,
        tm: Uuid,
        id: Uuid,
        version: i32,
        f: &AssetFields,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let done = sqlx::query(
            "UPDATE tm_assets SET name = $4, description = $5, category = $6, owner = $7,
                 confidentiality = $8, integrity = $9, availability = $10, personal_data = $11,
                 classification = $12, data_location = $13, retention = $14,
                 encryption_at_rest = $15,
                 version = version + 1, updated_at = now()
             WHERE id = $1 AND threat_model_id = $2 AND version = $3",
        )
        .bind(id)
        .bind(tm)
        .bind(version)
        .bind(&f.name)
        .bind(&f.description)
        .bind(f.category.as_str())
        .bind(&f.owner)
        .bind(f.confidentiality.map(|c| c.as_str()))
        .bind(f.integrity.map(|c| c.as_str()))
        .bind(f.availability.map(|c| c.as_str()))
        .bind(f.personal_data)
        .bind(f.classification.map(|c| c.as_str()))
        .bind(&f.data_location)
        .bind(&f.retention)
        .bind(f.encryption_at_rest.as_str())
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            unique_violation(e, || format!("an asset named '{}' already exists", f.name))
        })?;
        if done.rows_affected() == 0 {
            return Err(missing_or_conflict_in(&mut conn, "tm_assets", "asset", tm, id).await);
        }
        Ok(())
    }

    /// Deletes an asset and its links.
    pub async fn delete_tm_asset(&self, tm: Uuid, id: Uuid) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        delete_in(&mut conn, "tm_assets", "asset", tm, id).await
    }

    // ---- Flows ----

    async fn check_flow(conn: &mut PgConnection, tm: Uuid, f: &FlowFields) -> Result<()> {
        ensure_in(conn, "tm_components", tm, &[f.source_id], "sourceId").await?;
        ensure_in(conn, "tm_components", tm, &[f.target_id], "targetId").await?;
        ensure_in(conn, "tm_assets", tm, &f.asset_ids, "assetIds").await
    }

    /// Creates a data flow.
    pub async fn create_tm_flow(&self, tm: Uuid, f: &FlowFields) -> Result<Uuid> {
        let mut tx = self.pool.begin().await?;
        let id = Self::insert_flow(&mut tx, tm, f).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn insert_flow(conn: &mut PgConnection, tm: Uuid, f: &FlowFields) -> Result<Uuid> {
        Self::check_flow(conn, tm, f).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tm_flows (id, threat_model_id, source_id, target_id, relationship, label,
                 protocol, bidirectional, authentication, encrypted, data_classification,
                 authorization_model, entry_point)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(id)
        .bind(tm)
        .bind(f.source_id)
        .bind(f.target_id)
        .bind(f.relationship.as_str())
        .bind(&f.label)
        .bind(&f.protocol)
        .bind(f.bidirectional)
        .bind(&f.authentication)
        .bind(f.encrypted.as_str())
        .bind(f.data_classification.map(|c| c.as_str()))
        .bind(&f.authorization_model)
        .bind(f.entry_point)
        .execute(&mut *conn)
        .await?;
        replace_links(
            conn,
            "tm_flow_assets",
            "flow_id",
            "asset_id",
            id,
            &f.asset_ids,
        )
        .await?;
        Ok(id)
    }

    /// Replaces a data flow, including its asset links.
    pub async fn update_tm_flow(
        &self,
        tm: Uuid,
        id: Uuid,
        version: i32,
        f: &FlowFields,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        Self::check_flow(&mut tx, tm, f).await?;
        let done = sqlx::query(
            "UPDATE tm_flows SET source_id = $4, target_id = $5, relationship = $6, label = $7,
                 protocol = $8, bidirectional = $9, authentication = $10, encrypted = $11,
                 data_classification = $12, authorization_model = $13, entry_point = $14,
                 version = version + 1, updated_at = now()
             WHERE id = $1 AND threat_model_id = $2 AND version = $3",
        )
        .bind(id)
        .bind(tm)
        .bind(version)
        .bind(f.source_id)
        .bind(f.target_id)
        .bind(f.relationship.as_str())
        .bind(&f.label)
        .bind(&f.protocol)
        .bind(f.bidirectional)
        .bind(&f.authentication)
        .bind(f.encrypted.as_str())
        .bind(f.data_classification.map(|c| c.as_str()))
        .bind(&f.authorization_model)
        .bind(f.entry_point)
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() == 0 {
            return Err(missing_or_conflict_in(&mut tx, "tm_flows", "flow", tm, id).await);
        }
        replace_links(
            &mut tx,
            "tm_flow_assets",
            "flow_id",
            "asset_id",
            id,
            &f.asset_ids,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Deletes a data flow.
    pub async fn delete_tm_flow(&self, tm: Uuid, id: Uuid) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        delete_in(&mut conn, "tm_flows", "flow", tm, id).await
    }

    // ---- Threats ----

    async fn check_threat_links(conn: &mut PgConnection, tm: Uuid, f: &ThreatFields) -> Result<()> {
        let comps: Vec<Uuid> = f
            .targets
            .iter()
            .filter(|t| t.kind == TargetKind::Component)
            .map(|t| t.id)
            .collect();
        let flows: Vec<Uuid> = f
            .targets
            .iter()
            .filter(|t| t.kind == TargetKind::Flow)
            .map(|t| t.id)
            .collect();
        ensure_in(conn, "tm_components", tm, &comps, "targets").await?;
        ensure_in(conn, "tm_flows", tm, &flows, "targets").await?;
        ensure_in(conn, "tm_assets", tm, &f.asset_ids, "assetIds").await?;
        ensure_in(conn, "tm_controls", tm, &f.control_ids, "controlIds").await
    }

    async fn write_threat_links(conn: &mut PgConnection, id: Uuid, f: &ThreatFields) -> Result<()> {
        sqlx::query("DELETE FROM tm_threat_targets WHERE threat_id = $1")
            .bind(id)
            .execute(&mut *conn)
            .await?;
        let mut seen = HashSet::new();
        for (pos, t) in f.targets.iter().filter(|t| seen.insert(**t)).enumerate() {
            let (c, fl) = match t.kind {
                TargetKind::Component => (Some(t.id), None),
                TargetKind::Flow => (None, Some(t.id)),
            };
            sqlx::query(
                "INSERT INTO tm_threat_targets (threat_id, component_id, flow_id, position)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(id)
            .bind(c)
            .bind(fl)
            .bind(i32::try_from(pos).unwrap_or(i32::MAX))
            .execute(&mut *conn)
            .await?;
        }
        replace_links(
            conn,
            "tm_threat_assets",
            "threat_id",
            "asset_id",
            id,
            &f.asset_ids,
        )
        .await?;
        replace_links(
            conn,
            "tm_threat_controls",
            "threat_id",
            "control_id",
            id,
            &f.control_ids,
        )
        .await
    }

    async fn insert_threat(
        conn: &mut PgConnection,
        tm: Uuid,
        f: &ThreatFields,
        origin: ThreatOrigin,
        explanation: Option<serde_json::Value>,
        accepted_on: Option<NaiveDate>,
    ) -> Result<Uuid> {
        let number = next_number(conn, tm, "next_threat_no").await?;
        let id = Uuid::new_v4();
        let (il, ii) = rating_parts(f.inherent);
        let (cl, ci) = rating_parts(f.current);
        let (rl, ri) = rating_parts(f.residual);
        sqlx::query(
            "INSERT INTO tm_threats (id, threat_model_id, number, title, category, scenario,
                 attacker_objective, preconditions, consequences, status, dismiss_reason, origin,
                 explanation, inherent_likelihood, inherent_impact, current_likelihood,
                 current_impact, treatment, risk_owner, due_date, acceptance_rationale, accepted_by,
                 residual_likelihood, residual_impact, threat_source, detection,
                 existing_mitigations, confidence, acceptance_expires, accepted_on)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                 $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30)",
        )
        .bind(id)
        .bind(tm)
        .bind(number)
        .bind(&f.title)
        .bind(f.category.as_str())
        .bind(&f.scenario)
        .bind(&f.attacker_objective)
        .bind(&f.preconditions)
        .bind(&f.consequences)
        .bind(f.status.as_str())
        .bind(&f.dismiss_reason)
        .bind(origin.as_str())
        .bind(explanation)
        .bind(il)
        .bind(ii)
        .bind(cl)
        .bind(ci)
        .bind(f.treatment.as_str())
        .bind(&f.risk_owner)
        .bind(f.due_date)
        .bind(&f.acceptance_rationale)
        .bind(&f.accepted_by)
        .bind(rl)
        .bind(ri)
        .bind(&f.threat_source)
        .bind(&f.detection)
        .bind(&f.existing_mitigations)
        .bind(f.confidence.map(|c| c.as_str()))
        .bind(f.acceptance_expires)
        .bind(accepted_on)
        .execute(&mut *conn)
        .await?;
        Self::write_threat_links(conn, id, f).await?;
        Ok(id)
    }

    /// Creates a manual threat (fields already checked by the domain).
    pub async fn create_tm_threat(
        &self,
        tm: Uuid,
        f: &ThreatFields,
        accepted_on: Option<NaiveDate>,
    ) -> Result<Uuid> {
        let mut tx = self.pool.begin().await?;
        Self::check_threat_links(&mut tx, tm, f).await?;
        let id =
            Self::insert_threat(&mut tx, tm, f, ThreatOrigin::Manual, None, accepted_on).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Stores rule candidates and their rule keys; returns their ids.
    pub async fn create_tm_candidates(
        &self,
        tm: Uuid,
        candidates: &[Candidate],
    ) -> Result<Vec<Uuid>> {
        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::with_capacity(candidates.len());
        for c in candidates {
            let fields = ThreatFields {
                title: c.title.clone(),
                category: c.category,
                scenario: Some(c.scenario.clone()),
                attacker_objective: None,
                preconditions: None,
                consequences: None,
                targets: vec![c.target],
                asset_ids: c.asset_ids.clone(),
                control_ids: vec![],
                status: ThreatStatus::Candidate,
                dismiss_reason: None,
                inherent: None,
                current: None,
                residual: None,
                threat_source: None,
                detection: None,
                existing_mitigations: None,
                confidence: None,
                treatment: tm_domain::threat::RiskTreatment::Undecided,
                risk_owner: None,
                due_date: None,
                acceptance_rationale: None,
                accepted_by: None,
                acceptance_expires: None,
            };
            let id = Self::insert_threat(
                &mut tx,
                tm,
                &fields,
                ThreatOrigin::Rule,
                Some(serde_json::to_value(&c.explanation)?),
                None,
            )
            .await?;
            let inserted = sqlx::query(
                "INSERT INTO tm_rule_keys (threat_model_id, rule_key, threat_id) VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING",
            )
            .bind(tm)
            .bind(&c.rule_key)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            if inserted.rows_affected() == 0 {
                // A concurrent run used the key first.
                sqlx::query("DELETE FROM tm_threats WHERE id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                continue;
            }
            ids.push(id);
        }
        tx.commit().await?;
        Ok(ids)
    }

    /// Replaces a threat (fields already checked by the domain).
    pub async fn update_tm_threat(
        &self,
        tm: Uuid,
        id: Uuid,
        version: i32,
        f: &ThreatFields,
        accepted_on: Option<NaiveDate>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        Self::check_threat_links(&mut tx, tm, f).await?;
        let (il, ii) = rating_parts(f.inherent);
        let (cl, ci) = rating_parts(f.current);
        let (rl, ri) = rating_parts(f.residual);
        let done = sqlx::query(
            "UPDATE tm_threats SET title = $4, category = $5, scenario = $6, attacker_objective = $7,
                 preconditions = $8, consequences = $9, status = $10, dismiss_reason = $11,
                 inherent_likelihood = $12, inherent_impact = $13, current_likelihood = $14,
                 current_impact = $15, treatment = $16, risk_owner = $17, due_date = $18,
                 acceptance_rationale = $19, accepted_by = $20,
                 residual_likelihood = $21, residual_impact = $22, threat_source = $23,
                 detection = $24, existing_mitigations = $25, confidence = $26,
                 acceptance_expires = $27, accepted_on = $28,
                 version = version + 1, updated_at = now()
             WHERE id = $1 AND threat_model_id = $2 AND version = $3",
        )
        .bind(id)
        .bind(tm)
        .bind(version)
        .bind(&f.title)
        .bind(f.category.as_str())
        .bind(&f.scenario)
        .bind(&f.attacker_objective)
        .bind(&f.preconditions)
        .bind(&f.consequences)
        .bind(f.status.as_str())
        .bind(&f.dismiss_reason)
        .bind(il)
        .bind(ii)
        .bind(cl)
        .bind(ci)
        .bind(f.treatment.as_str())
        .bind(&f.risk_owner)
        .bind(f.due_date)
        .bind(&f.acceptance_rationale)
        .bind(&f.accepted_by)
        .bind(rl)
        .bind(ri)
        .bind(&f.threat_source)
        .bind(&f.detection)
        .bind(&f.existing_mitigations)
        .bind(f.confidence.map(|c| c.as_str()))
        .bind(f.acceptance_expires)
        .bind(accepted_on)
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() == 0 {
            return Err(missing_or_conflict_in(&mut tx, "tm_threats", "threat", tm, id).await);
        }
        Self::write_threat_links(&mut tx, id, f).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Sets status and dismiss reason of several threats in one transaction
    /// (values already checked by the domain).
    pub async fn set_tm_threat_statuses(
        &self,
        tm: Uuid,
        changes: &[(Uuid, ThreatStatus, Option<String>)],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for (id, status, reason) in changes {
            let done = sqlx::query(
                "UPDATE tm_threats SET status = $3, dismiss_reason = $4,
                     version = version + 1, updated_at = now()
                 WHERE id = $1 AND threat_model_id = $2",
            )
            .bind(id)
            .bind(tm)
            .bind(status.as_str())
            .bind(reason)
            .execute(&mut *tx)
            .await?;
            if done.rows_affected() == 0 {
                return Err(StorageError::NotFound("threat"));
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Deletes a threat; a rule key stays, so the rule does not suggest it again.
    pub async fn delete_tm_threat(&self, tm: Uuid, id: Uuid) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        delete_in(&mut conn, "tm_threats", "threat", tm, id).await
    }

    // ---- Controls ----

    async fn write_control(conn: &mut PgConnection, id: Uuid, f: &ControlFields) -> Result<()> {
        replace_links(
            conn,
            "tm_control_components",
            "control_id",
            "component_id",
            id,
            &f.component_ids,
        )
        .await?;
        sqlx::query("DELETE FROM tm_threat_controls WHERE control_id = $1")
            .bind(id)
            .execute(&mut *conn)
            .await?;
        let mut seen = HashSet::new();
        for t in f.threat_ids.iter().filter(|t| seen.insert(**t)) {
            sqlx::query(
                "INSERT INTO tm_threat_controls (threat_id, control_id, position)
                 SELECT $1, $2, COALESCE(max(position) + 1, 0) FROM tm_threat_controls WHERE threat_id = $1",
            )
            .bind(t)
            .bind(id)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }

    async fn check_control(conn: &mut PgConnection, tm: Uuid, f: &ControlFields) -> Result<()> {
        ensure_in(conn, "tm_components", tm, &f.component_ids, "componentIds").await?;
        ensure_in(conn, "tm_threats", tm, &f.threat_ids, "threatIds").await
    }

    /// Creates a control.
    pub async fn create_tm_control(&self, tm: Uuid, f: &ControlFields) -> Result<Uuid> {
        let mut tx = self.pool.begin().await?;
        let id = Self::insert_control(&mut tx, tm, f).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn insert_control(conn: &mut PgConnection, tm: Uuid, f: &ControlFields) -> Result<Uuid> {
        Self::check_control(conn, tm, f).await?;
        let number = next_number(conn, tm, "next_control_no").await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tm_controls (id, threat_model_id, number, name, description, control_type,
                 category, status, owner, acceptance_criteria, review_by, verified_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                 CASE WHEN $8 = 'verified' THEN now() END)",
        )
        .bind(id)
        .bind(tm)
        .bind(number)
        .bind(&f.name)
        .bind(&f.description)
        .bind(f.control_type.as_str())
        .bind(f.category.as_str())
        .bind(f.status.as_str())
        .bind(&f.owner)
        .bind(&f.acceptance_criteria)
        .bind(f.review_by)
        .execute(&mut *conn)
        .await?;
        Self::write_control(conn, id, f).await?;
        Ok(id)
    }

    /// Replaces a control, including its links.
    pub async fn update_tm_control(
        &self,
        tm: Uuid,
        id: Uuid,
        version: i32,
        f: &ControlFields,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        Self::check_control(&mut tx, tm, f).await?;
        let done = sqlx::query(
            "UPDATE tm_controls SET name = $4, description = $5, control_type = $6, category = $7,
                 status = $8, owner = $9, acceptance_criteria = $10, review_by = $11,
                 verified_at = CASE WHEN $8 = 'verified' THEN COALESCE(verified_at, now()) END,
                 version = version + 1, updated_at = now()
             WHERE id = $1 AND threat_model_id = $2 AND version = $3",
        )
        .bind(id)
        .bind(tm)
        .bind(version)
        .bind(&f.name)
        .bind(&f.description)
        .bind(f.control_type.as_str())
        .bind(f.category.as_str())
        .bind(f.status.as_str())
        .bind(&f.owner)
        .bind(&f.acceptance_criteria)
        .bind(f.review_by)
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() == 0 {
            return Err(missing_or_conflict_in(&mut tx, "tm_controls", "control", tm, id).await);
        }
        Self::write_control(&mut tx, id, f).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Deletes a control and its links.
    pub async fn delete_tm_control(&self, tm: Uuid, id: Uuid) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        delete_in(&mut conn, "tm_controls", "control", tm, id).await
    }

    // ---- Security tests ----

    async fn check_test(conn: &mut PgConnection, tm: Uuid, f: &SecurityTestFields) -> Result<()> {
        if f.threat_ids.is_empty() {
            return Err(StorageError::Invalid(
                "threatIds: a security test needs at least one threat".into(),
            ));
        }
        ensure_in(conn, "tm_threats", tm, &f.threat_ids, "threatIds").await?;
        ensure_in(conn, "tm_controls", tm, &f.control_ids, "controlIds").await
    }

    async fn insert_test(
        conn: &mut PgConnection,
        tm: Uuid,
        f: &SecurityTestFields,
        origin: SecurityTestOrigin,
        source: Option<&TemplateSource>,
    ) -> Result<Uuid> {
        let number = next_number(conn, tm, "next_test_no").await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO security_tests (id, threat_model_id, number, title, objective,
                 preconditions, steps, test_type, status, origin, template_id, template_version,
                 source_threat_id, source_control_id, template_key)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(id)
        .bind(tm)
        .bind(number)
        .bind(&f.title)
        .bind(&f.objective)
        .bind(&f.preconditions)
        .bind(serde_json::to_value(&f.steps)?)
        .bind(f.test_type.as_str())
        .bind(f.status.as_str())
        .bind(origin.as_str())
        .bind(source.map(|s| s.template_id.clone()))
        .bind(source.map(|s| s.template_version))
        .bind(source.map(|s| s.threat_id))
        .bind(source.and_then(|s| s.control_id))
        .bind(source.map(TemplateSource::key))
        .execute(&mut *conn)
        .await?;
        replace_links(
            conn,
            "security_test_threats",
            "security_test_id",
            "threat_id",
            id,
            &f.threat_ids,
        )
        .await?;
        replace_links(
            conn,
            "security_test_controls",
            "security_test_id",
            "control_id",
            id,
            &f.control_ids,
        )
        .await?;
        Ok(id)
    }

    /// Creates a manual security test.
    pub async fn create_security_test(&self, tm: Uuid, f: &SecurityTestFields) -> Result<Uuid> {
        let mut tx = self.pool.begin().await?;
        Self::check_test(&mut tx, tm, f).await?;
        let id = Self::insert_test(&mut tx, tm, f, SecurityTestOrigin::Manual, None).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Stores generated tests; a key taken concurrently is skipped. Returns
    /// the ids created.
    pub async fn create_generated_tests(
        &self,
        tm: Uuid,
        tests: &[NewSecurityTest],
    ) -> Result<Vec<Uuid>> {
        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::with_capacity(tests.len());
        for t in tests {
            sqlx::query("SAVEPOINT gen").execute(&mut *tx).await?;
            match Self::insert_test(
                &mut tx,
                tm,
                &t.fields,
                SecurityTestOrigin::Generated,
                Some(&t.source),
            )
            .await
            {
                Ok(id) => ids.push(id),
                Err(StorageError::Database(sqlx::Error::Database(db)))
                    if db.code().as_deref() == Some("23505") =>
                {
                    sqlx::query("ROLLBACK TO SAVEPOINT gen")
                        .execute(&mut *tx)
                        .await?;
                }
                Err(e) => return Err(e),
            }
        }
        tx.commit().await?;
        Ok(ids)
    }

    /// Replaces a security test including its steps and links.
    pub async fn update_security_test(
        &self,
        tm: Uuid,
        id: Uuid,
        version: i32,
        f: &SecurityTestFields,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        Self::check_test(&mut tx, tm, f).await?;
        let done = sqlx::query(
            "UPDATE security_tests SET title = $4, objective = $5, preconditions = $6, steps = $7,
                 test_type = $8, status = $9, version = version + 1, updated_at = now()
             WHERE id = $1 AND threat_model_id = $2 AND version = $3",
        )
        .bind(id)
        .bind(tm)
        .bind(version)
        .bind(&f.title)
        .bind(&f.objective)
        .bind(&f.preconditions)
        .bind(serde_json::to_value(&f.steps)?)
        .bind(f.test_type.as_str())
        .bind(f.status.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            unique_violation(e, || {
                "a non-obsolete test from the same template already exists".into()
            })
        })?;
        if done.rows_affected() == 0 {
            return Err(
                missing_or_conflict_in(&mut tx, "security_tests", "security test", tm, id).await,
            );
        }
        replace_links(
            &mut tx,
            "security_test_threats",
            "security_test_id",
            "threat_id",
            id,
            &f.threat_ids,
        )
        .await?;
        replace_links(
            &mut tx,
            "security_test_controls",
            "security_test_id",
            "control_id",
            id,
            &f.control_ids,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Sets the status of several security tests.
    pub async fn set_security_test_statuses(
        &self,
        tm: Uuid,
        ids: &[Uuid],
        status: SecurityTestStatus,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        ensure_in(&mut tx, "security_tests", tm, ids, "ids").await?;
        sqlx::query(
            "UPDATE security_tests SET status = $3, version = version + 1, updated_at = now()
             WHERE threat_model_id = $1 AND id = ANY($2) AND status <> $3",
        )
        .bind(tm)
        .bind(ids)
        .bind(status.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            unique_violation(e, || {
                "a non-obsolete test from the same template already exists".into()
            })
        })?;
        tx.commit().await?;
        Ok(())
    }

    /// Deletes a security test and its results.
    pub async fn delete_security_test(&self, tm: Uuid, id: Uuid) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        delete_in(&mut conn, "security_tests", "security test", tm, id).await
    }

    /// Records a result of a security test.
    pub async fn record_security_test_result(
        &self,
        tm: Uuid,
        test_id: Uuid,
        outcome: SecurityTestOutcome,
        note: Option<String>,
        executed_at: DateTime<Utc>,
    ) -> Result<SecurityTestResult> {
        let mut conn = self.pool.acquire().await?;
        ensure_in(&mut conn, "security_tests", tm, &[test_id], "id")
            .await
            .map_err(|_| StorageError::NotFound("security test"))?;
        let row = sqlx::query(
            "INSERT INTO security_test_results (id, security_test_id, outcome, note, executed_at)
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(test_id)
        .bind(outcome.as_str())
        .bind(note)
        .bind(executed_at)
        .fetch_one(&mut *conn)
        .await?;
        Ok(test_result(&row)?)
    }

    /// Result history of a security test, newest first.
    pub async fn security_test_results(
        &self,
        tm: Uuid,
        test_id: Uuid,
    ) -> Result<Vec<SecurityTestResult>> {
        let mut conn = self.pool.acquire().await?;
        ensure_in(&mut conn, "security_tests", tm, &[test_id], "id")
            .await
            .map_err(|_| StorageError::NotFound("security test"))?;
        let rows = sqlx::query(
            "SELECT * FROM security_test_results WHERE security_test_id = $1
             ORDER BY executed_at DESC, recorded_at DESC",
        )
        .bind(test_id)
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows.iter().map(test_result).collect::<Result<_, _>>()?)
    }
}

fn test_result(r: &PgRow) -> Result<SecurityTestResult, sqlx::Error> {
    Ok(SecurityTestResult {
        id: r.try_get("id")?,
        security_test_id: r.try_get("security_test_id")?,
        outcome: parse_col(r, "outcome")?,
        note: r.try_get("note")?,
        executed_at: r.try_get("executed_at")?,
    })
}

fn evidence_row(r: &PgRow) -> Result<Evidence, sqlx::Error> {
    let control: Option<Uuid> = r.try_get("control_id")?;
    let result: Option<Uuid> = r.try_get("result_id")?;
    let (owner_kind, owner_id) = match (control, result) {
        (Some(c), _) => (EvidenceOwnerKind::Control, c),
        (None, Some(x)) => (EvidenceOwnerKind::TestResult, x),
        (None, None) => return Err(sqlx::Error::Decode("evidence without owner".into())),
    };
    Ok(Evidence {
        audit: audit(r)?,
        owner_kind,
        owner_id,
        fields: EvidenceFields {
            kind: parse_col(r, "kind")?,
            title: r.try_get("title")?,
            link: r.try_get("link")?,
            note: r.try_get("note")?,
            evidence_date: r.try_get("evidence_date")?,
            reviewer: r.try_get("reviewer")?,
        },
    })
}

/// What evidence is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceOwner {
    /// A security control.
    Control(Uuid),
    /// A recorded security test result of the given test.
    TestResult {
        /// The test.
        test_id: Uuid,
        /// The result.
        result_id: Uuid,
    },
}

/// A stored baseline without its snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineMeta {
    /// Id.
    pub id: Uuid,
    /// Unique name within the threat model.
    pub name: String,
    /// Note.
    pub note: Option<String>,
    /// Who created it.
    pub created_by: Option<String>,
    /// When.
    pub created_at: DateTime<Utc>,
    /// Version of the snapshot format.
    pub schema_version: i32,
    /// Threats in the snapshot.
    pub threat_count: i32,
    /// Controls in the snapshot.
    pub control_count: i32,
}

/// A new baseline.
#[derive(Debug, Clone)]
pub struct NewBaseline {
    /// Name, unique within the threat model.
    pub name: String,
    /// Note.
    pub note: Option<String>,
    /// Creator.
    pub created_by: Option<String>,
    /// Snapshot format version.
    pub schema_version: i32,
    /// Threats in the snapshot.
    pub threat_count: i32,
    /// Controls in the snapshot.
    pub control_count: i32,
    /// The snapshot.
    pub snapshot: serde_json::Value,
}

/// Most baselines one threat model may hold.
pub const MAX_BASELINES: i64 = 50;

fn baseline_meta(r: &PgRow) -> Result<BaselineMeta, sqlx::Error> {
    Ok(BaselineMeta {
        id: r.try_get("id")?,
        name: r.try_get("name")?,
        note: r.try_get("note")?,
        created_by: r.try_get("created_by")?,
        created_at: r.try_get("created_at")?,
        schema_version: r.try_get("schema_version")?,
        threat_count: r.try_get("threat_count")?,
        control_count: r.try_get("control_count")?,
    })
}

impl Store {
    async fn evidence_owner_column(
        conn: &mut PgConnection,
        tm: Uuid,
        owner: EvidenceOwner,
    ) -> Result<(Option<Uuid>, Option<Uuid>)> {
        match owner {
            EvidenceOwner::Control(id) => {
                ensure_in(conn, "tm_controls", tm, &[id], "id")
                    .await
                    .map_err(|_| StorageError::NotFound("control"))?;
                Ok((Some(id), None))
            }
            EvidenceOwner::TestResult { test_id, result_id } => {
                ensure_in(conn, "security_tests", tm, &[test_id], "id")
                    .await
                    .map_err(|_| StorageError::NotFound("security test"))?;
                let found: Option<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM security_test_results WHERE id = $1 AND security_test_id = $2",
                )
                .bind(result_id)
                .bind(test_id)
                .fetch_optional(&mut *conn)
                .await?;
                found.ok_or(StorageError::NotFound("result"))?;
                Ok((None, Some(result_id)))
            }
        }
    }

    /// Lists the evidence of a control or a result, newest first.
    pub async fn list_evidence(&self, tm: Uuid, owner: EvidenceOwner) -> Result<Vec<Evidence>> {
        let mut conn = self.pool.acquire().await?;
        let (control, result) = Self::evidence_owner_column(&mut conn, tm, owner).await?;
        let rows = sqlx::query(
            "SELECT * FROM tm_evidence
             WHERE threat_model_id = $1 AND control_id IS NOT DISTINCT FROM $2
               AND result_id IS NOT DISTINCT FROM $3
             ORDER BY evidence_date DESC, created_at DESC, id",
        )
        .bind(tm)
        .bind(control)
        .bind(result)
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows.iter().map(evidence_row).collect::<Result<_, _>>()?)
    }

    /// Adds evidence (fields already checked by the domain).
    pub async fn add_evidence(
        &self,
        tm: Uuid,
        owner: EvidenceOwner,
        f: &EvidenceFields,
    ) -> Result<Evidence> {
        let mut tx = self.pool.begin().await?;
        let (control, result) = Self::evidence_owner_column(&mut tx, tm, owner).await?;
        let row = sqlx::query(
            "INSERT INTO tm_evidence (id, threat_model_id, control_id, result_id, kind, title,
                 link, note, evidence_date, reviewer)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(tm)
        .bind(control)
        .bind(result)
        .bind(f.kind.as_str())
        .bind(&f.title)
        .bind(f.link.as_deref().filter(|v| !v.trim().is_empty()))
        .bind(f.note.as_deref().filter(|v| !v.trim().is_empty()))
        .bind(f.evidence_date)
        .bind(&f.reviewer)
        .fetch_one(&mut *tx)
        .await?;
        let e = evidence_row(&row)?;
        tx.commit().await?;
        Ok(e)
    }

    /// Number of evidence records of a control.
    pub async fn control_evidence_count(&self, tm: Uuid, control: Uuid) -> Result<usize> {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tm_evidence WHERE threat_model_id = $1 AND control_id = $2",
        )
        .bind(tm)
        .bind(control)
        .fetch_one(&self.pool)
        .await?;
        Ok(usize::try_from(n).unwrap_or(0))
    }

    /// Deletes evidence. A verified control left without evidence becomes
    /// implemented in the same transaction; returns that control's id.
    pub async fn delete_evidence(&self, tm: Uuid, id: Uuid) -> Result<Option<Uuid>> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "DELETE FROM tm_evidence WHERE id = $1 AND threat_model_id = $2 RETURNING control_id",
        )
        .bind(id)
        .bind(tm)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(StorageError::NotFound("evidence"))?;
        let control: Option<Uuid> = row.try_get("control_id")?;
        let mut downgraded = None;
        if let Some(c) = control {
            let remaining: i64 =
                sqlx::query_scalar("SELECT count(*) FROM tm_evidence WHERE control_id = $1")
                    .bind(c)
                    .fetch_one(&mut *tx)
                    .await?;
            if remaining == 0 {
                let done = sqlx::query(
                    "UPDATE tm_controls SET status = 'implemented', verified_at = NULL,
                         version = version + 1, updated_at = now()
                     WHERE id = $1 AND status = 'verified'",
                )
                .bind(c)
                .execute(&mut *tx)
                .await?;
                if done.rows_affected() > 0 {
                    downgraded = Some(c);
                }
            }
        }
        tx.commit().await?;
        Ok(downgraded)
    }

    /// Creates a baseline; names are unique and at most [`MAX_BASELINES`] exist.
    pub async fn create_baseline(&self, tm: Uuid, b: NewBaseline) -> Result<BaselineMeta> {
        let mut tx = self.pool.begin().await?;
        // Serialise concurrent creates of one threat model so the cap holds.
        sqlx::query("SELECT id FROM threat_models WHERE id = $1 FOR UPDATE")
            .bind(tm)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound("threat model"))?;
        let n: i64 =
            sqlx::query_scalar("SELECT count(*) FROM tm_baselines WHERE threat_model_id = $1")
                .bind(tm)
                .fetch_one(&mut *tx)
                .await?;
        if n >= MAX_BASELINES {
            return Err(StorageError::Invalid(format!(
                "baselines: at most {MAX_BASELINES} per threat model; delete one first"
            )));
        }
        let row = sqlx::query(
            "INSERT INTO tm_baselines (id, threat_model_id, name, note, created_by,
                 schema_version, threat_count, control_count, snapshot)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, name, note, created_by, created_at, schema_version, threat_count,
                 control_count",
        )
        .bind(Uuid::new_v4())
        .bind(tm)
        .bind(&b.name)
        .bind(&b.note)
        .bind(&b.created_by)
        .bind(b.schema_version)
        .bind(b.threat_count)
        .bind(b.control_count)
        .bind(&b.snapshot)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            unique_violation(e, || {
                format!("a baseline named '{}' already exists", b.name)
            })
        })?;
        let meta = baseline_meta(&row)?;
        tx.commit().await?;
        Ok(meta)
    }

    /// Baselines of a threat model, newest first, without snapshots.
    pub async fn list_baselines(&self, tm: Uuid) -> Result<Vec<BaselineMeta>> {
        let rows = sqlx::query(
            "SELECT id, name, note, created_by, created_at, schema_version, threat_count,
                 control_count
             FROM tm_baselines WHERE threat_model_id = $1 ORDER BY created_at DESC, id",
        )
        .bind(tm)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(baseline_meta).collect::<Result<_, _>>()?)
    }

    /// A baseline with its snapshot.
    pub async fn get_baseline(
        &self,
        tm: Uuid,
        id: Uuid,
    ) -> Result<(BaselineMeta, serde_json::Value)> {
        let row = sqlx::query("SELECT * FROM tm_baselines WHERE id = $1 AND threat_model_id = $2")
            .bind(id)
            .bind(tm)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("baseline"))?;
        Ok((baseline_meta(&row)?, row.try_get("snapshot")?))
    }

    /// Deletes a baseline.
    pub async fn delete_baseline(&self, tm: Uuid, id: Uuid) -> Result<()> {
        let done = sqlx::query("DELETE FROM tm_baselines WHERE id = $1 AND threat_model_id = $2")
            .bind(id)
            .bind(tm)
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(StorageError::NotFound("baseline"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AI proposals of a threat model (ADR 0013)
// ---------------------------------------------------------------------------

/// A stored proposal.
#[derive(Debug, Clone, PartialEq)]
pub struct TmProposalRow {
    /// Id.
    pub id: Uuid,
    /// Threat model.
    pub threat_model_id: Uuid,
    /// `tm-threats`, …
    pub kind: String,
    /// `pending`, `accepted` or `rejected`.
    pub status: String,
    /// Kind-specific payload.
    pub payload: serde_json::Value,
    /// Why the model proposed it.
    pub rationale: Option<String>,
    /// Model id of the LLM.
    pub source: Option<String>,
    /// Version of the prompt templates.
    pub prompt_version: Option<String>,
    /// Entities the request was about.
    pub target_ids: Vec<Uuid>,
    /// Entities created or changed by accepting it.
    pub resulting_entity_ids: Vec<Uuid>,
    /// Why it was rejected.
    pub reject_reason: Option<String>,
    /// When it was made.
    pub created_at: DateTime<Utc>,
}

/// A proposal about to be stored.
#[derive(Debug, Clone)]
pub struct NewTmProposal {
    /// Kind.
    pub kind: String,
    /// Payload.
    pub payload: serde_json::Value,
    /// Rationale.
    pub rationale: Option<String>,
    /// Model id.
    pub source: Option<String>,
    /// Prompt version.
    pub prompt_version: Option<String>,
    /// Targets of the request.
    pub target_ids: Vec<Uuid>,
}

/// A component of an accepted architecture proposal; zone and assets by name.
#[derive(Debug, Clone)]
pub struct NewComponent {
    /// Fields, with no zone and no assets yet.
    pub fields: ArchComponentFields,
    /// Zone name.
    pub zone: Option<String>,
    /// Asset names.
    pub assets: Vec<String>,
}

/// A flow of an accepted architecture proposal; ends and assets by name.
#[derive(Debug, Clone)]
pub struct NewFlow {
    /// Fields, with no ends and no assets yet.
    pub fields: FlowFields,
    /// Source component name.
    pub source: String,
    /// Target component name.
    pub target: String,
    /// Asset names.
    pub assets: Vec<String>,
}

/// An accepted architecture proposal.
#[derive(Debug, Clone, Default)]
pub struct NewArchitecture {
    /// Zones.
    pub zones: Vec<ZoneFields>,
    /// Assets.
    pub assets: Vec<AssetFields>,
    /// Components.
    pub components: Vec<NewComponent>,
    /// Flows.
    pub flows: Vec<NewFlow>,
}

/// What accepting a controls proposal does.
#[derive(Debug, Clone)]
pub enum ControlApply {
    /// Create a planned control.
    New(ControlFields),
    /// Link an existing control to threats.
    Link {
        /// The control.
        control_id: Uuid,
        /// Threats to link it to.
        threat_ids: Vec<Uuid>,
    },
}

fn proposal_row(r: &PgRow) -> Result<TmProposalRow, sqlx::Error> {
    Ok(TmProposalRow {
        id: r.try_get("id")?,
        threat_model_id: r.try_get("threat_model_id")?,
        kind: r.try_get("kind")?,
        status: r.try_get("status")?,
        payload: r.try_get("payload")?,
        rationale: r.try_get("rationale")?,
        source: r.try_get("source")?,
        prompt_version: r.try_get("prompt_version")?,
        target_ids: r.try_get("target_ids")?,
        resulting_entity_ids: r.try_get("resulting_entity_ids")?,
        reject_reason: r.try_get("reject_reason")?,
        created_at: r.try_get("created_at")?,
    })
}

fn name_key(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Marks a row as created by AI.
async fn mark_ai(conn: &mut PgConnection, table: &str, id: Uuid, model: &str) -> Result<()> {
    debug_assert!(matches!(
        table,
        "tm_zones"
            | "tm_components"
            | "tm_assets"
            | "tm_flows"
            | "tm_controls"
            | "tm_threats"
            | "security_tests"
    ));
    let origin_set = if matches!(
        table,
        "tm_zones" | "tm_components" | "tm_assets" | "tm_flows" | "tm_controls"
    ) {
        "origin = 'ai', "
    } else {
        ""
    };
    sqlx::query(&format!(
        "UPDATE {table} SET {origin_set}ai_source = $2 WHERE id = $1"
    ))
    .bind(id)
    .bind(model)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn names(conn: &mut PgConnection, table: &str, tm: Uuid) -> Result<HashMap<String, Uuid>> {
    let rows: Vec<(String, Uuid)> = sqlx::query_as(&format!(
        "SELECT name, id FROM {table} WHERE threat_model_id = $1"
    ))
    .bind(tm)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(|(n, i)| (name_key(&n), i)).collect())
}

/// A new name must not match an existing one, ignoring case: references are
/// by name and have to stay unambiguous.
fn fresh(map: &HashMap<String, Uuid>, name: &str, what: &str) -> Result<()> {
    if map.contains_key(&name_key(name)) {
        return Err(StorageError::Conflict(format!(
            "a {what} named \"{name}\" already exists"
        )));
    }
    Ok(())
}

fn resolve(map: &HashMap<String, Uuid>, name: &str, what: &str) -> Result<Uuid> {
    map.get(&name_key(name)).copied().ok_or_else(|| {
        StorageError::Invalid(format!(
            "{what} \"{name}\" does not exist in this threat model"
        ))
    })
}

impl Store {
    /// Stores pending proposals.
    pub async fn create_tm_proposals(
        &self,
        tm: Uuid,
        items: Vec<NewTmProposal>,
    ) -> Result<Vec<TmProposalRow>> {
        let mut tx = self.pool.begin().await?;
        let mut out = Vec::with_capacity(items.len());
        for p in items {
            let row = sqlx::query(
                "INSERT INTO tm_proposals (id, threat_model_id, kind, payload, rationale, source,
                     prompt_version, target_ids)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
            )
            .bind(Uuid::new_v4())
            .bind(tm)
            .bind(&p.kind)
            .bind(&p.payload)
            .bind(&p.rationale)
            .bind(&p.source)
            .bind(&p.prompt_version)
            .bind(&p.target_ids)
            .fetch_one(&mut *tx)
            .await?;
            out.push(proposal_row(&row)?);
        }
        tx.commit().await?;
        Ok(out)
    }

    /// Proposals of a threat model, newest first.
    pub async fn list_tm_proposals(
        &self,
        tm: Uuid,
        status: Option<&str>,
        kind: Option<&str>,
    ) -> Result<Vec<TmProposalRow>> {
        let rows = sqlx::query(
            "SELECT * FROM tm_proposals
             WHERE threat_model_id = $1 AND ($2::text IS NULL OR status = $2)
               AND ($3::text IS NULL OR kind = $3)
             ORDER BY created_at DESC, id",
        )
        .bind(tm)
        .bind(status)
        .bind(kind)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(proposal_row).collect::<Result<_, _>>()?)
    }

    /// One proposal of a threat model.
    pub async fn get_tm_proposal(&self, tm: Uuid, id: Uuid) -> Result<TmProposalRow> {
        let row = sqlx::query("SELECT * FROM tm_proposals WHERE id = $1 AND threat_model_id = $2")
            .bind(id)
            .bind(tm)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::NotFound("proposal"))?;
        Ok(proposal_row(&row)?)
    }

    /// Rejects a pending proposal; nothing is created.
    pub async fn reject_tm_proposal(
        &self,
        tm: Uuid,
        id: Uuid,
        reason: Option<String>,
    ) -> Result<TmProposalRow> {
        let mut tx = self.pool.begin().await?;
        Self::claim_proposal(&mut tx, tm, id).await?;
        let row = sqlx::query(
            "UPDATE tm_proposals SET status = 'rejected', reject_reason = $2 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(reason)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(proposal_row(&row)?)
    }

    /// Locks a proposal of the threat model; it must still be pending.
    async fn claim_proposal(conn: &mut PgConnection, tm: Uuid, id: Uuid) -> Result<TmProposalRow> {
        let row = sqlx::query(
            "SELECT * FROM tm_proposals WHERE id = $1 AND threat_model_id = $2 FOR UPDATE",
        )
        .bind(id)
        .bind(tm)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("proposal"))?;
        let p = proposal_row(&row)?;
        if p.status != "pending" {
            return Err(StorageError::Conflict(format!(
                "the proposal is already {}",
                p.status
            )));
        }
        Ok(p)
    }

    async fn finish_proposal(
        conn: &mut PgConnection,
        id: Uuid,
        payload: &serde_json::Value,
        results: &[Uuid],
    ) -> Result<TmProposalRow> {
        let row = sqlx::query(
            "UPDATE tm_proposals SET status = 'accepted', payload = $2, resulting_entity_ids = $3
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(payload)
        .bind(results)
        .fetch_one(&mut *conn)
        .await?;
        Ok(proposal_row(&row)?)
    }

    /// Accepts an architecture proposal: creates its zones, assets, components and
    /// flows in dependency order, in one transaction. References are by name,
    /// first among the new elements and then among the existing ones.
    pub async fn apply_tm_architecture(
        &self,
        tm: Uuid,
        id: Uuid,
        payload: &serde_json::Value,
        a: NewArchitecture,
        model: &str,
    ) -> Result<TmProposalRow> {
        let mut tx = self.pool.begin().await?;
        Self::claim_proposal(&mut tx, tm, id).await?;
        let mut zones = names(&mut tx, "tm_zones", tm).await?;
        let mut assets = names(&mut tx, "tm_assets", tm).await?;
        let mut comps = names(&mut tx, "tm_components", tm).await?;
        let mut results = Vec::new();
        for z in &a.zones {
            fresh(&zones, &z.name, "zone")?;
            let zid = Self::insert_zone(&mut tx, tm, z).await?;
            mark_ai(&mut tx, "tm_zones", zid, model).await?;
            zones.insert(name_key(&z.name), zid);
            results.push(zid);
        }
        for x in &a.assets {
            fresh(&assets, &x.name, "asset")?;
            let aid = Self::insert_asset(&mut tx, tm, x).await?;
            mark_ai(&mut tx, "tm_assets", aid, model).await?;
            assets.insert(name_key(&x.name), aid);
            results.push(aid);
        }
        for c in a.components {
            fresh(&comps, &c.fields.name, "component")?;
            let mut f = c.fields;
            f.zone_id = c
                .zone
                .as_deref()
                .map(|z| resolve(&zones, z, "zone"))
                .transpose()?;
            f.asset_ids = c
                .assets
                .iter()
                .map(|n| resolve(&assets, n, "asset"))
                .collect::<Result<_>>()?;
            let cid = Self::insert_component(&mut tx, tm, &f).await?;
            mark_ai(&mut tx, "tm_components", cid, model).await?;
            comps.insert(name_key(&f.name), cid);
            results.push(cid);
        }
        for fl in a.flows {
            let mut f = fl.fields;
            f.source_id = resolve(&comps, &fl.source, "component")?;
            f.target_id = resolve(&comps, &fl.target, "component")?;
            f.asset_ids = fl
                .assets
                .iter()
                .map(|n| resolve(&assets, n, "asset"))
                .collect::<Result<_>>()?;
            let fid = Self::insert_flow(&mut tx, tm, &f).await?;
            mark_ai(&mut tx, "tm_flows", fid, model).await?;
            results.push(fid);
        }
        let row = Self::finish_proposal(&mut tx, id, payload, &results).await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Accepts a threat proposal: an open threat with origin AI.
    pub async fn apply_tm_threat(
        &self,
        tm: Uuid,
        id: Uuid,
        payload: &serde_json::Value,
        f: &ThreatFields,
        model: &str,
    ) -> Result<TmProposalRow> {
        let mut tx = self.pool.begin().await?;
        Self::claim_proposal(&mut tx, tm, id).await?;
        Self::check_threat_links(&mut tx, tm, f).await?;
        let tid = Self::insert_threat(&mut tx, tm, f, ThreatOrigin::Ai, None, None).await?;
        mark_ai(&mut tx, "tm_threats", tid, model).await?;
        let row = Self::finish_proposal(&mut tx, id, payload, &[tid]).await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Accepts a rating proposal: sets one rating of one threat, nothing else.
    pub async fn apply_tm_rating(
        &self,
        tm: Uuid,
        id: Uuid,
        payload: &serde_json::Value,
        threat_id: Uuid,
        basis: &str,
        rating: Rating,
    ) -> Result<TmProposalRow> {
        let (l, i) = match basis {
            "inherent" => ("inherent_likelihood", "inherent_impact"),
            "current" => ("current_likelihood", "current_impact"),
            "residual" => ("residual_likelihood", "residual_impact"),
            other => {
                return Err(StorageError::Invalid(format!(
                    "rating: unknown basis {other}"
                )))
            }
        };
        let mut tx = self.pool.begin().await?;
        Self::claim_proposal(&mut tx, tm, id).await?;
        let done = sqlx::query(&format!(
            "UPDATE tm_threats SET {l} = $3, {i} = $4, version = version + 1, updated_at = now()
             WHERE id = $1 AND threat_model_id = $2"
        ))
        .bind(threat_id)
        .bind(tm)
        .bind(i16::from(rating.likelihood))
        .bind(i16::from(rating.impact))
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() == 0 {
            return Err(StorageError::Invalid(
                "threatId: the threat no longer exists in this threat model".into(),
            ));
        }
        let row = Self::finish_proposal(&mut tx, id, payload, &[threat_id]).await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Accepts a control proposal: a planned control, or a link of an existing one.
    pub async fn apply_tm_control(
        &self,
        tm: Uuid,
        id: Uuid,
        payload: &serde_json::Value,
        apply: ControlApply,
        model: &str,
    ) -> Result<TmProposalRow> {
        let mut tx = self.pool.begin().await?;
        Self::claim_proposal(&mut tx, tm, id).await?;
        let control_id = match apply {
            ControlApply::New(f) => {
                let cid = Self::insert_control(&mut tx, tm, &f).await?;
                mark_ai(&mut tx, "tm_controls", cid, model).await?;
                cid
            }
            ControlApply::Link {
                control_id,
                threat_ids,
            } => {
                ensure_in(&mut tx, "tm_controls", tm, &[control_id], "linkControlId").await?;
                ensure_in(&mut tx, "tm_threats", tm, &threat_ids, "threatIds").await?;
                for t in threat_ids.iter().collect::<HashSet<_>>() {
                    sqlx::query(
                        "INSERT INTO tm_threat_controls (threat_id, control_id, position)
                         SELECT $1, $2, COALESCE(max(position) + 1, 0)
                         FROM tm_threat_controls WHERE threat_id = $1
                         ON CONFLICT DO NOTHING",
                    )
                    .bind(t)
                    .bind(control_id)
                    .execute(&mut *tx)
                    .await?;
                }
                control_id
            }
        };
        let row = Self::finish_proposal(&mut tx, id, payload, &[control_id]).await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Accepts a security test proposal: a draft test with origin AI.
    pub async fn apply_tm_test(
        &self,
        tm: Uuid,
        id: Uuid,
        payload: &serde_json::Value,
        f: &SecurityTestFields,
        model: &str,
    ) -> Result<TmProposalRow> {
        let mut tx = self.pool.begin().await?;
        Self::claim_proposal(&mut tx, tm, id).await?;
        Self::check_test(&mut tx, tm, f).await?;
        let sid = Self::insert_test(&mut tx, tm, f, SecurityTestOrigin::Ai, None).await?;
        mark_ai(&mut tx, "security_tests", sid, model).await?;
        let row = Self::finish_proposal(&mut tx, id, payload, &[sid]).await?;
        tx.commit().await?;
        Ok(row)
    }
}
