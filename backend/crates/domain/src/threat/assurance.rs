//! Evidence, control verification, completeness findings and baseline
//! comparison. See docs/specification/09-threat-modeling.md#assurance-residual-risk-evidence-acceptance-baselines.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::arch::Architecture;
use super::risk::{effective_status, Thresholds};
use super::{
    AuthState, Control, ControlFlag, ControlStatus, EvidenceKind, EvidenceOwnerKind, Threat,
    ThreatModelError, ThreatStatus, TriState,
};
use crate::entities::Audit;

/// Editable fields of an evidence record.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceFields {
    /// Kind.
    pub kind: EvidenceKind,
    /// Title.
    pub title: String,
    /// Link to the evidence.
    pub link: Option<String>,
    /// Free-text note.
    pub note: Option<String>,
    /// Date of the evidence.
    pub evidence_date: NaiveDate,
    /// Who reviewed it.
    pub reviewer: Option<String>,
}

/// Evidence of a control or a security test result. A reference, not a file.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    /// Audit fields.
    pub audit: Audit,
    /// What it belongs to.
    pub owner_kind: EvidenceOwnerKind,
    /// Id of the control or result.
    pub owner_id: Uuid,
    /// Editable fields.
    pub fields: EvidenceFields,
}

fn blank(s: &Option<String>) -> bool {
    s.as_deref().is_none_or(|v| v.trim().is_empty())
}

/// Evidence needs a title and a link or a note.
pub fn check_evidence(f: &EvidenceFields) -> Result<(), ThreatModelError> {
    if f.title.trim().is_empty() {
        return Err(ThreatModelError::new("title", "must not be empty"));
    }
    if blank(&f.link) && blank(&f.note) {
        return Err(ThreatModelError::new("link", "provide a link or a note"));
    }
    Ok(())
}

/// A control may become verified only with evidence.
pub fn check_can_verify(evidence_count: usize) -> Result<(), ThreatModelError> {
    if evidence_count == 0 {
        Err(ThreatModelError::new(
            "status",
            "evidence is required to verify a control",
        ))
    } else {
        Ok(())
    }
}

/// Status of a verified control after its last evidence was removed.
pub fn status_after_evidence_removed(status: ControlStatus, remaining: usize) -> ControlStatus {
    if status == ControlStatus::Verified && remaining == 0 {
        ControlStatus::Implemented
    } else {
        status
    }
}

/// Computed flags of a control.
pub fn control_flags(
    control: &Control,
    evidence_count: usize,
    today: NaiveDate,
) -> Vec<ControlFlag> {
    let mut out = Vec::new();
    if control.fields.status == ControlStatus::Verified {
        if control.fields.review_by.is_some_and(|d| d < today) {
            out.push(ControlFlag::VerificationOverdue);
        }
        if evidence_count == 0 {
            out.push(ControlFlag::VerifiedWithoutEvidence);
        }
    }
    out
}

/// Date recorded as `accepted_on`: today for a new or changed acceptance.
pub fn acceptance_date(
    previous: Option<&Threat>,
    new: &super::ThreatFields,
    today: NaiveDate,
) -> Option<NaiveDate> {
    if new.treatment != super::RiskTreatment::Accept {
        return None;
    }
    match previous {
        Some(p)
            if p.fields.treatment == super::RiskTreatment::Accept
                && p.fields.acceptance_expires == new.acceptance_expires
                && p.fields.accepted_by == new.accepted_by
                && p.accepted_on.is_some() =>
        {
            p.accepted_on
        }
        _ => Some(today),
    }
}

/// Days ahead that count as "expiring".
pub const EXPIRING_DAYS: i64 = 30;

/// Finding on the completeness list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable code.
    pub code: &'static str,
    /// Message.
    pub message: String,
    /// `component`, `flow`, `threat` or `control`.
    pub entity_kind: &'static str,
    /// Entity.
    pub entity_id: Uuid,
}

fn finding(code: &'static str, message: String, kind: &'static str, id: Uuid) -> Finding {
    Finding {
        code,
        message,
        entity_kind: kind,
        entity_id: id,
    }
}

/// Completeness findings of a threat model. A list, not a score.
pub fn completeness(
    arch: &Architecture,
    threats: &[Threat],
    controls: &[Control],
    evidence_counts: &HashMap<Uuid, usize>,
    today: NaiveDate,
) -> Vec<Finding> {
    let mut out = Vec::new();
    for c in &arch.components {
        if blank(&c.fields.owner) {
            out.push(finding(
                "COMPONENT_NO_OWNER",
                format!("{} has no owner", c.fields.name),
                "component",
                c.audit.id,
            ));
        }
        if arch.zone_of(c).is_none() {
            out.push(finding(
                "COMPONENT_NO_ZONE",
                format!("{} has no exposure zone", c.fields.name),
                "component",
                c.audit.id,
            ));
        }
    }
    for f in &arch.flows {
        let name = arch.flow_name(f);
        if f.fields.auth_state() == AuthState::Unknown {
            out.push(finding(
                "FLOW_AUTH_UNKNOWN",
                format!("{name}: authentication not documented"),
                "flow",
                f.audit.id,
            ));
        }
        if f.fields.encrypted == TriState::Unknown {
            out.push(finding(
                "FLOW_ENCRYPTION_UNKNOWN",
                format!("{name}: encryption in transit not documented"),
                "flow",
                f.audit.id,
            ));
        }
    }
    for t in threats {
        let status = effective_status(t, today);
        if matches!(status, ThreatStatus::Dismissed | ThreatStatus::Candidate) {
            continue;
        }
        let label = format!("{} {}", t.display_id(), t.fields.title);
        if t.fields.inherent.is_none() {
            out.push(finding(
                "THREAT_NOT_RATED",
                format!("{label} has no inherent rating"),
                "threat",
                t.audit.id,
            ));
        }
        if status == ThreatStatus::Open {
            if t.fields.treatment == super::RiskTreatment::Undecided {
                out.push(finding(
                    "THREAT_NO_TREATMENT",
                    format!("{label} has no treatment"),
                    "threat",
                    t.audit.id,
                ));
            }
            if blank(&t.fields.risk_owner) {
                out.push(finding(
                    "THREAT_NO_OWNER",
                    format!("{label} has no risk owner"),
                    "threat",
                    t.audit.id,
                ));
            }
        }
        if t.fields.status == ThreatStatus::Accepted {
            match t.fields.acceptance_expires {
                Some(d) if d < today => out.push(finding(
                    "ACCEPTANCE_EXPIRED",
                    format!("{label}: acceptance expired on {d}"),
                    "threat",
                    t.audit.id,
                )),
                Some(d) if d <= today + Duration::days(EXPIRING_DAYS) => out.push(finding(
                    "ACCEPTANCE_EXPIRING",
                    format!("{label}: acceptance expires on {d}"),
                    "threat",
                    t.audit.id,
                )),
                None => out.push(finding(
                    "ACCEPTANCE_NO_EXPIRY",
                    format!("{label}: acceptance has no expiry"),
                    "threat",
                    t.audit.id,
                )),
                _ => {}
            }
        }
    }
    for c in controls {
        if c.fields.status != ControlStatus::Verified {
            continue;
        }
        let label = format!("{} {}", c.display_id(), c.fields.name);
        if c.fields.review_by.is_none() {
            out.push(finding(
                "CONTROL_NO_REVIEW_DATE",
                format!("{label} is verified without a review-by date"),
                "control",
                c.audit.id,
            ));
        }
        if evidence_counts.get(&c.audit.id).copied().unwrap_or(0) == 0 {
            out.push(finding(
                "CONTROL_NO_EVIDENCE",
                format!("{label} is verified without evidence"),
                "control",
                c.audit.id,
            ));
        }
    }
    out
}

/// What the comparison needs to know about a threat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatDigest {
    /// Id.
    pub id: Uuid,
    /// `THR-n`.
    pub display_id: String,
    /// Title.
    pub title: String,
    /// Effective status.
    pub status: String,
    /// Inherent level, if rated.
    pub inherent: Option<String>,
    /// Current level, if rated.
    pub current: Option<String>,
    /// Residual level, if rated.
    pub residual: Option<String>,
    /// Treatment.
    pub treatment: String,
}

/// What the comparison needs to know about a control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlDigest {
    /// Id.
    pub id: Uuid,
    /// `CTL-n`.
    pub display_id: String,
    /// Status.
    pub status: String,
}

/// Digest of a threat under given thresholds.
pub fn threat_digest(t: &Threat, thresholds: &Thresholds, today: NaiveDate) -> ThreatDigest {
    let level = |r: Option<super::Rating>| r.map(|r| thresholds.level(r.score()).to_string());
    ThreatDigest {
        id: t.audit.id,
        display_id: t.display_id(),
        title: t.fields.title.clone(),
        status: effective_status(t, today).to_string(),
        inherent: level(t.fields.inherent),
        current: level(t.fields.current),
        residual: level(t.fields.residual),
        treatment: t.fields.treatment.to_string(),
    }
}

/// One changed field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChange {
    /// Field name.
    pub field: &'static str,
    /// Value in the baseline.
    pub before: Option<String>,
    /// Value now.
    pub after: Option<String>,
}

/// A threat in the comparison with its changes (empty for added and removed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparedThreat {
    /// Id.
    pub id: Uuid,
    /// `THR-n`.
    pub display_id: String,
    /// Title.
    pub title: String,
    /// Changed fields.
    pub changes: Vec<FieldChange>,
}

/// A control whose status changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlChange {
    /// Id.
    pub id: Uuid,
    /// `CTL-n`.
    pub display_id: String,
    /// Status in the baseline.
    pub before: String,
    /// Status now.
    pub after: String,
}

/// Result of comparing a baseline with the live model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Comparison {
    /// In the live model only.
    pub added: Vec<ComparedThreat>,
    /// In the baseline only.
    pub removed: Vec<ComparedThreat>,
    /// In both with differences.
    pub changed: Vec<ComparedThreat>,
    /// Controls whose status changed.
    pub controls_changed: Vec<ControlChange>,
    /// Threats per current level (and `not-assessed`) in the baseline.
    pub levels_baseline: std::collections::BTreeMap<String, usize>,
    /// The same for the live model.
    pub levels_live: std::collections::BTreeMap<String, usize>,
}

fn levels(threats: &[ThreatDigest]) -> std::collections::BTreeMap<String, usize> {
    let mut m = std::collections::BTreeMap::new();
    for t in threats {
        let key = t.current.clone().unwrap_or_else(|| "not-assessed".into());
        *m.entry(key).or_insert(0) += 1;
    }
    m
}

/// Compares a baseline with the live model by entity id.
pub fn diff(
    base: &[ThreatDigest],
    live: &[ThreatDigest],
    base_controls: &[ControlDigest],
    live_controls: &[ControlDigest],
) -> Comparison {
    let base_by: HashMap<Uuid, &ThreatDigest> = base.iter().map(|t| (t.id, t)).collect();
    let live_ids: HashSet<Uuid> = live.iter().map(|t| t.id).collect();
    let plain = |t: &ThreatDigest| ComparedThreat {
        id: t.id,
        display_id: t.display_id.clone(),
        title: t.title.clone(),
        changes: vec![],
    };
    let mut c = Comparison {
        levels_baseline: levels(base),
        levels_live: levels(live),
        ..Comparison::default()
    };
    for l in live {
        match base_by.get(&l.id) {
            None => c.added.push(plain(l)),
            Some(b) => {
                let mut changes = Vec::new();
                let mut cmp = |field: &'static str, x: Option<&String>, y: Option<&String>| {
                    if x != y {
                        changes.push(FieldChange {
                            field,
                            before: x.cloned(),
                            after: y.cloned(),
                        });
                    }
                };
                cmp("status", Some(&b.status), Some(&l.status));
                cmp("inherentLevel", b.inherent.as_ref(), l.inherent.as_ref());
                cmp("currentLevel", b.current.as_ref(), l.current.as_ref());
                cmp("residualLevel", b.residual.as_ref(), l.residual.as_ref());
                cmp("treatment", Some(&b.treatment), Some(&l.treatment));
                if !changes.is_empty() {
                    let mut p = plain(l);
                    p.changes = changes;
                    c.changed.push(p);
                }
            }
        }
    }
    for b in base {
        if !live_ids.contains(&b.id) {
            c.removed.push(plain(b));
        }
    }
    let live_by: HashMap<Uuid, &ControlDigest> = live_controls.iter().map(|x| (x.id, x)).collect();
    for b in base_controls {
        if let Some(l) = live_by.get(&b.id) {
            if l.status != b.status {
                c.controls_changed.push(ControlChange {
                    id: b.id,
                    display_id: b.display_id.clone(),
                    before: b.status.clone(),
                    after: l.status.clone(),
                });
            }
        }
    }
    c
}

/// Timestamp helper for callers that stamp snapshots.
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::super::testutil::*;
    use super::super::*;
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ev(link: Option<&str>, note: Option<&str>) -> EvidenceFields {
        EvidenceFields {
            kind: EvidenceKind::TestReport,
            title: "Pen test".into(),
            link: link.map(Into::into),
            note: note.map(Into::into),
            evidence_date: day(2026, 10, 1),
            reviewer: None,
        }
    }

    #[test]
    fn evidence_needs_link_or_note() {
        assert!(check_evidence(&ev(Some("https://x"), None)).is_ok());
        assert!(check_evidence(&ev(None, Some("see wiki"))).is_ok());
        assert!(check_evidence(&ev(None, None)).is_err());
        assert!(check_evidence(&ev(Some("  "), Some(""))).is_err());
    }

    #[test]
    fn verify_needs_evidence_and_last_removal_downgrades() {
        assert!(check_can_verify(0).is_err());
        assert!(check_can_verify(1).is_ok());
        assert_eq!(
            status_after_evidence_removed(ControlStatus::Verified, 0),
            ControlStatus::Implemented
        );
        assert_eq!(
            status_after_evidence_removed(ControlStatus::Verified, 1),
            ControlStatus::Verified
        );
        assert_eq!(
            status_after_evidence_removed(ControlStatus::Planned, 0),
            ControlStatus::Planned
        );
    }

    #[test]
    fn overdue_and_unevidenced_control_flags() {
        let today = day(2026, 10, 17);
        let mut c = control(1, "MFA", ControlStatus::Verified);
        c.fields.review_by = Some(day(2026, 10, 16));
        assert_eq!(
            control_flags(&c, 1, today),
            vec![ControlFlag::VerificationOverdue]
        );
        c.fields.review_by = None;
        assert!(control_flags(&c, 1, today).is_empty());
        assert_eq!(
            control_flags(&c, 0, today),
            vec![ControlFlag::VerifiedWithoutEvidence]
        );
        c.fields.status = ControlStatus::Implemented;
        assert!(control_flags(&c, 0, today).is_empty());
    }

    #[test]
    fn acceptance_date_is_kept_unless_the_acceptance_changes() {
        let today = day(2026, 10, 17);
        let target = ThreatTarget {
            kind: TargetKind::Component,
            id: Uuid::nil(),
        };
        let mut f = threat_fields(target);
        f.treatment = RiskTreatment::Accept;
        f.accepted_by = Some("A".into());
        f.acceptance_expires = Some(day(2027, 1, 1));
        let mut prev = threat(1, f.clone());
        prev.accepted_on = Some(day(2026, 9, 1));
        assert_eq!(
            acceptance_date(Some(&prev), &f, today),
            Some(day(2026, 9, 1))
        );
        let mut renewed = f.clone();
        renewed.acceptance_expires = Some(day(2027, 6, 1));
        assert_eq!(acceptance_date(Some(&prev), &renewed, today), Some(today));
        let mut not = f.clone();
        not.treatment = RiskTreatment::Mitigate;
        assert_eq!(acceptance_date(Some(&prev), &not, today), None);
        assert_eq!(acceptance_date(None, &f, today), Some(today));
    }

    #[test]
    fn completeness_lists_findings_per_entity() {
        let today = day(2026, 10, 17);
        let mut a = Architecture::default();
        let z = zone(&mut a, "Internal", 3);
        let c1 = component(&mut a, "Api", ArchComponentType::Api, Some(z));
        let c2 = component(&mut a, "Db", ArchComponentType::RelationalDatabase, None);
        flow(&mut a, c1, c2);
        let target = ThreatTarget {
            kind: TargetKind::Component,
            id: c1,
        };
        let mut open = threat(1, threat_fields(target));
        open.fields.inherent = None;
        let mut accepted_f = threat_fields(target);
        accepted_f.status = ThreatStatus::Accepted;
        accepted_f.treatment = RiskTreatment::Accept;
        accepted_f.inherent = Rating::new(2, 2, "inherent").ok();
        accepted_f.risk_owner = Some("x".into());
        accepted_f.acceptance_expires = Some(day(2026, 11, 1));
        let accepted = threat(2, accepted_f);
        let ctl = control(1, "WAF", ControlStatus::Verified);
        let codes: Vec<_> = completeness(&a, &[open, accepted], &[ctl], &HashMap::new(), today)
            .iter()
            .map(|f| f.code)
            .collect();
        for expected in [
            "COMPONENT_NO_OWNER",
            "COMPONENT_NO_ZONE",
            "FLOW_AUTH_UNKNOWN",
            "FLOW_ENCRYPTION_UNKNOWN",
            "THREAT_NOT_RATED",
            "THREAT_NO_TREATMENT",
            "THREAT_NO_OWNER",
            "ACCEPTANCE_EXPIRING",
            "CONTROL_NO_REVIEW_DATE",
            "CONTROL_NO_EVIDENCE",
        ] {
            assert!(codes.contains(&expected), "missing {expected}: {codes:?}");
        }
    }

    #[test]
    fn clean_model_has_no_findings() {
        let a = Architecture::default();
        assert!(completeness(&a, &[], &[], &HashMap::new(), day(2026, 1, 1)).is_empty());
    }

    fn digest(n: i32, title: &str, current: Option<&str>) -> ThreatDigest {
        ThreatDigest {
            id: Uuid::from_u128(n as u128),
            display_id: format!("THR-{n}"),
            title: title.into(),
            status: "open".into(),
            inherent: Some("high".into()),
            current: current.map(Into::into),
            residual: None,
            treatment: "mitigate".into(),
        }
    }

    #[test]
    fn diff_reports_added_removed_changed_and_controls() {
        let base = vec![digest(1, "a", Some("high")), digest(2, "b", Some("low"))];
        let live = vec![digest(1, "a", Some("moderate")), digest(3, "c", None)];
        let bc = vec![ControlDigest {
            id: Uuid::from_u128(9),
            display_id: "CTL-1".into(),
            status: "planned".into(),
        }];
        let lc = vec![ControlDigest {
            status: "verified".into(),
            ..bc[0].clone()
        }];
        let d = diff(&base, &live, &bc, &lc);
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.added[0].display_id, "THR-3");
        assert_eq!(d.removed[0].display_id, "THR-2");
        assert_eq!(d.changed.len(), 1);
        assert_eq!(d.changed[0].changes[0].field, "currentLevel");
        assert_eq!(d.changed[0].changes[0].before.as_deref(), Some("high"));
        assert_eq!(d.changed[0].changes[0].after.as_deref(), Some("moderate"));
        assert_eq!(d.controls_changed[0].after, "verified");
        assert_eq!(d.levels_baseline["high"], 1);
        assert_eq!(d.levels_live["not-assessed"], 1);
    }

    #[test]
    fn identical_models_have_an_empty_diff() {
        let t = vec![digest(1, "a", Some("high"))];
        let d = diff(&t, &t, &[], &[]);
        assert!(d.added.is_empty() && d.removed.is_empty() && d.changed.is_empty());
    }
}
