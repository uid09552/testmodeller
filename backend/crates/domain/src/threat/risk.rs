//! Risk ratings, levels, thresholds and threat flags.
//! See docs/specification/09-threat-modeling.md#risk-score-and-level.

use std::collections::HashMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    Control, ControlStatus, SecurityTest, SecurityTestOutcome, Threat, ThreatFlag,
    ThreatModelError, ThreatStatus,
};
use crate::entities::string_enum;

string_enum!(
    /// Risk level a score maps to.
    RiskLevel { Low => "low", Moderate => "moderate", High => "high", Critical => "critical" }
);

const LEVEL_ORDER: [RiskLevel; 4] = [
    RiskLevel::Low,
    RiskLevel::Moderate,
    RiskLevel::High,
    RiskLevel::Critical,
];

/// Likelihood (1 rare … 5 almost certain) and impact (1 negligible … 5 severe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rating {
    /// 1–5.
    pub likelihood: u8,
    /// 1–5.
    pub impact: u8,
}

impl Rating {
    /// Validated rating; `field` names the input in the error.
    pub fn new(likelihood: u8, impact: u8, field: &'static str) -> Result<Self, ThreatModelError> {
        if !(1..=5).contains(&likelihood) || !(1..=5).contains(&impact) {
            return Err(ThreatModelError::new(
                field,
                "likelihood and impact must be between 1 and 5",
            ));
        }
        Ok(Self { likelihood, impact })
    }

    /// Likelihood × impact, 1–25.
    pub fn score(self) -> u8 {
        self.likelihood * self.impact
    }
}

/// Score range of one level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Threshold {
    /// Level.
    pub level: RiskLevel,
    /// Lowest score, inclusive.
    pub min: u8,
    /// Highest score, inclusive.
    pub max: u8,
}

/// Score-to-level mapping of a threat model. Always valid once constructed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Threshold>", into = "Vec<Threshold>")]
pub struct Thresholds(Vec<Threshold>);

impl Default for Thresholds {
    fn default() -> Self {
        Self(vec![
            Threshold {
                level: RiskLevel::Low,
                min: 1,
                max: 4,
            },
            Threshold {
                level: RiskLevel::Moderate,
                min: 5,
                max: 9,
            },
            Threshold {
                level: RiskLevel::High,
                min: 10,
                max: 16,
            },
            Threshold {
                level: RiskLevel::Critical,
                min: 17,
                max: 25,
            },
        ])
    }
}

impl Thresholds {
    /// Validates: each level once, in ascending order, covering 1–25 without
    /// gap or overlap.
    pub fn new(items: Vec<Threshold>) -> Result<Self, ThreatModelError> {
        let err = |m: String| Err(ThreatModelError::new("thresholds", m));
        if items.len() != LEVEL_ORDER.len()
            || items.iter().zip(LEVEL_ORDER).any(|(t, l)| t.level != l)
        {
            return err("must list low, moderate, high and critical once, in that order".into());
        }
        let mut next = 1u8;
        for t in &items {
            if t.min > t.max {
                return err(format!("{}: min is greater than max", t.level));
            }
            if t.min != next {
                let msg = if t.min > next {
                    if next == t.min - 1 {
                        format!("score {next} has no level")
                    } else {
                        format!("scores {next}–{} have no level", t.min - 1)
                    }
                } else {
                    format!("{} overlaps the level below", t.level)
                };
                return err(msg);
            }
            next = t.max + 1;
        }
        if next != 26 {
            return err("the highest level must end at 25".into());
        }
        Ok(Self(items))
    }

    /// The thresholds, low to critical.
    pub fn items(&self) -> &[Threshold] {
        &self.0
    }

    /// Level of a score 1–25.
    pub fn level(&self, score: u8) -> RiskLevel {
        self.0
            .iter()
            .find(|t| (t.min..=t.max).contains(&score))
            .map_or(RiskLevel::Critical, |t| t.level)
    }
}

impl TryFrom<Vec<Threshold>> for Thresholds {
    type Error = ThreatModelError;

    fn try_from(v: Vec<Threshold>) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

impl From<Thresholds> for Vec<Threshold> {
    fn from(t: Thresholds) -> Self {
        t.0
    }
}

/// An accepted threat whose acceptance has lapsed.
pub fn acceptance_expired(threat: &Threat, today: NaiveDate) -> bool {
    threat.fields.status == ThreatStatus::Accepted
        && threat.fields.acceptance_expires.is_some_and(|d| d < today)
}

/// Status as it reads today: a lapsed acceptance is open again.
pub fn effective_status(threat: &Threat, today: NaiveDate) -> ThreatStatus {
    if acceptance_expired(threat, today) {
        ThreatStatus::Open
    } else {
        threat.fields.status
    }
}

/// Facts behind the residual flags: the linked controls that are not verified.
pub fn unsupported_residual_facts(
    threat: &Threat,
    controls: &HashMap<Uuid, &Control>,
) -> Vec<String> {
    threat
        .fields
        .control_ids
        .iter()
        .filter_map(|id| controls.get(id))
        .filter(|c| c.fields.status != ControlStatus::Verified)
        .map(|c| format!("{} is {}", c.display_id(), c.fields.status))
        .collect()
}

/// Computes the flags of a threat (09-threat-modeling.md#threat-flags).
/// `tests` are the security tests linking this threat.
pub fn threat_flags(
    threat: &Threat,
    controls: &HashMap<Uuid, &Control>,
    tests: &[&SecurityTest],
    today: NaiveDate,
) -> Vec<ThreatFlag> {
    let f = &threat.fields;
    let expired = acceptance_expired(threat, today);
    let open = f.status == ThreatStatus::Open || expired;
    let linked: Vec<&Control> = f
        .control_ids
        .iter()
        .filter_map(|id| controls.get(id).copied())
        .collect();
    let mut out = Vec::new();
    if open && linked.is_empty() {
        out.push(ThreatFlag::NoControl);
    }
    if open
        && !linked.is_empty()
        && linked
            .iter()
            .all(|c| c.fields.status == ControlStatus::Planned)
    {
        out.push(ThreatFlag::PlannedControlsOnly);
    }
    if let (Some(i), Some(c)) = (f.inherent, f.current) {
        if c.score() < i.score()
            && !linked
                .iter()
                .any(|c| c.fields.status != ControlStatus::Planned)
        {
            out.push(ThreatFlag::UnsupportedReduction);
        }
    }
    if let (Some(c), Some(r)) = (f.current, f.residual) {
        if r.score() > c.score() {
            out.push(ThreatFlag::ResidualAboveCurrent);
        } else if r.score() < c.score()
            && !linked
                .iter()
                .any(|c| c.fields.status == ControlStatus::Verified)
        {
            out.push(ThreatFlag::UnsupportedResidual);
        }
    }
    if expired {
        out.push(ThreatFlag::AcceptanceExpired);
    }
    if open && f.due_date.is_some_and(|d| d < today) {
        out.push(ThreatFlag::Overdue);
    }
    if tests.iter().any(|t| {
        t.last_result
            .as_ref()
            .is_some_and(|r| r.outcome == SecurityTestOutcome::Failed)
    }) {
        out.push(ThreatFlag::FailingTest);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::testutil::*;
    use super::super::*;
    use super::*;
    use proptest::prelude::*;

    fn th(level: RiskLevel, min: u8, max: u8) -> Threshold {
        Threshold { level, min, max }
    }

    #[test]
    fn score_and_default_levels() {
        let r = Rating::new(4, 5, "inherent").unwrap();
        assert_eq!(r.score(), 20);
        let t = Thresholds::default();
        assert_eq!(
            t.level(Rating::new(3, 4, "x").unwrap().score()),
            RiskLevel::High
        );
        assert_eq!(t.level(4), RiskLevel::Low);
        assert_eq!(t.level(9), RiskLevel::Moderate);
        assert_eq!(t.level(17), RiskLevel::Critical);
    }

    #[test]
    fn out_of_range_rating_is_rejected() {
        let e = Rating::new(6, 1, "inherent").unwrap_err();
        assert_eq!(e.field, "inherent");
        assert!(Rating::new(0, 3, "current").is_err());
    }

    #[test]
    fn custom_thresholds_reclassify() {
        let t = Thresholds::new(vec![
            th(RiskLevel::Low, 1, 4),
            th(RiskLevel::Moderate, 5, 9),
            th(RiskLevel::High, 10, 14),
            th(RiskLevel::Critical, 15, 25),
        ])
        .unwrap();
        assert_eq!(t.level(15), RiskLevel::Critical);
    }

    #[test]
    fn gap_overlap_and_order_are_rejected() {
        let gap = Thresholds::new(vec![
            th(RiskLevel::Low, 1, 4),
            th(RiskLevel::Moderate, 5, 8),
            th(RiskLevel::High, 10, 16),
            th(RiskLevel::Critical, 17, 25),
        ])
        .unwrap_err();
        assert_eq!(gap.message, "score 9 has no level");
        assert!(Thresholds::new(vec![
            th(RiskLevel::Low, 1, 5),
            th(RiskLevel::Moderate, 5, 9),
            th(RiskLevel::High, 10, 16),
            th(RiskLevel::Critical, 17, 25),
        ])
        .is_err());
        assert!(Thresholds::new(vec![
            th(RiskLevel::Moderate, 1, 4),
            th(RiskLevel::Low, 5, 9),
            th(RiskLevel::High, 10, 16),
            th(RiskLevel::Critical, 17, 25),
        ])
        .is_err());
        assert!(Thresholds::new(vec![
            th(RiskLevel::Low, 1, 4),
            th(RiskLevel::Moderate, 5, 9),
            th(RiskLevel::High, 10, 16),
            th(RiskLevel::Critical, 17, 24),
        ])
        .is_err());
    }

    #[test]
    fn thresholds_serde_validates() {
        let json = serde_json::to_string(&Thresholds::default()).unwrap();
        assert!(json.starts_with("[{\"level\":\"low\",\"min\":1,\"max\":4}"));
        assert!(serde_json::from_str::<Thresholds>("[]").is_err());
    }

    proptest! {
        #[test]
        fn valid_thresholds_map_every_score_once(a in 1u8..23, b in 1u8..23, c in 1u8..23) {
            let mut cuts = [a, b, c];
            cuts.sort_unstable();
            prop_assume!(cuts[0] < cuts[1] && cuts[1] < cuts[2] && cuts[2] < 25);
            let t = Thresholds::new(vec![
                th(RiskLevel::Low, 1, cuts[0]),
                th(RiskLevel::Moderate, cuts[0] + 1, cuts[1]),
                th(RiskLevel::High, cuts[1] + 1, cuts[2]),
                th(RiskLevel::Critical, cuts[2] + 1, 25),
            ]).unwrap();
            for s in 1..=25u8 {
                let n = t.items().iter().filter(|x| (x.min..=x.max).contains(&s)).count();
                prop_assert_eq!(n, 1);
                let _ = t.level(s);
            }
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 17).unwrap()
    }

    #[test]
    fn flags_no_control_and_planned_only() {
        let t = threat(
            1,
            threat_fields(ThreatTarget {
                kind: TargetKind::Component,
                id: Uuid::nil(),
            }),
        );
        assert_eq!(
            threat_flags(&t, &HashMap::new(), &[], today()),
            [ThreatFlag::NoControl]
        );
        let c = control(1, "Authz", ControlStatus::Planned);
        let mut t2 = t.clone();
        t2.fields.control_ids = vec![c.audit.id];
        let map = HashMap::from([(c.audit.id, &c)]);
        assert_eq!(
            threat_flags(&t2, &map, &[], today()),
            [ThreatFlag::PlannedControlsOnly]
        );
    }

    #[test]
    fn linking_a_verified_control_does_not_change_ratings_but_clears_reduction_flag() {
        let mut t = threat(
            1,
            threat_fields(ThreatTarget {
                kind: TargetKind::Component,
                id: Uuid::nil(),
            }),
        );
        t.fields.inherent = Some(Rating::new(4, 5, "i").unwrap());
        t.fields.current = Some(Rating::new(2, 3, "c").unwrap());
        let planned = control(1, "P", ControlStatus::Planned);
        t.fields.control_ids = vec![planned.audit.id];
        let map = HashMap::from([(planned.audit.id, &planned)]);
        assert!(threat_flags(&t, &map, &[], today()).contains(&ThreatFlag::UnsupportedReduction));
        let verified = control(2, "V", ControlStatus::Verified);
        t.fields.control_ids.push(verified.audit.id);
        let map = HashMap::from([(planned.audit.id, &planned), (verified.audit.id, &verified)]);
        assert!(!threat_flags(&t, &map, &[], today()).contains(&ThreatFlag::UnsupportedReduction));
        assert_eq!(t.fields.current.unwrap().score(), 6);
    }

    #[test]
    fn overdue_and_failing_test() {
        let mut t = threat(
            1,
            threat_fields(ThreatTarget {
                kind: TargetKind::Component,
                id: Uuid::nil(),
            }),
        );
        t.fields.treatment = RiskTreatment::Mitigate;
        t.fields.due_date = NaiveDate::from_ymd_opt(2026, 10, 1);
        let mut test = SecurityTest {
            audit: audit(),
            number: 1,
            fields: SecurityTestFields {
                title: "x".into(),
                objective: None,
                preconditions: vec![],
                steps: vec![],
                test_type: SecurityTestType::Other,
                status: SecurityTestStatus::Ready,
                threat_ids: vec![t.audit.id],
                control_ids: vec![],
            },
            origin: SecurityTestOrigin::Manual,
            source: None,
            last_result: None,
            source_model: None,
        };
        test.last_result = Some(SecurityTestResult {
            id: Uuid::new_v4(),
            security_test_id: test.audit.id,
            outcome: SecurityTestOutcome::Failed,
            note: None,
            executed_at: test.audit.created_at,
        });
        let flags = threat_flags(&t, &HashMap::new(), &[&test], today());
        assert!(flags.contains(&ThreatFlag::Overdue));
        assert!(flags.contains(&ThreatFlag::FailingTest));
    }

    fn threat_with(current: Option<(u8, u8)>, residual: Option<(u8, u8)>) -> Threat {
        let target = ThreatTarget {
            kind: TargetKind::Component,
            id: uuid::Uuid::nil(),
        };
        let mut f = threat_fields(target);
        f.current = current.map(|(l, i)| Rating::new(l, i, "current").unwrap());
        f.residual = residual.map(|(l, i)| Rating::new(l, i, "residual").unwrap());
        threat(1, f)
    }

    #[test]
    fn residual_score_and_level() {
        let t = threat_with(None, Some((2, 3)));
        let r = t.fields.residual.unwrap();
        assert_eq!(r.score(), 6);
        assert_eq!(Thresholds::default().level(r.score()), RiskLevel::Moderate);
        assert!(Rating::new(2, 0, "residual").is_err());
    }

    #[test]
    fn residual_flags() {
        let above = threat_with(Some((2, 3)), Some((3, 3)));
        assert!(threat_flags(&above, &HashMap::new(), &[], today())
            .contains(&ThreatFlag::ResidualAboveCurrent));

        let mut t = threat_with(Some((3, 4)), Some((2, 2)));
        let mut c = control(1, "WAF", ControlStatus::Implemented);
        let id = c.audit.id;
        c.fields.threat_ids = vec![t.audit.id];
        t.fields.control_ids = vec![id];
        let mut map = HashMap::new();
        map.insert(id, &c);
        let flags = threat_flags(&t, &map, &[], today());
        assert!(flags.contains(&ThreatFlag::UnsupportedResidual));
        assert_eq!(
            unsupported_residual_facts(&t, &map),
            vec!["CTL-1 is implemented".to_owned()]
        );

        let v = control(1, "WAF", ControlStatus::Verified);
        let mut map = HashMap::new();
        map.insert(id, &v);
        assert!(!threat_flags(&t, &map, &[], today()).contains(&ThreatFlag::UnsupportedResidual));
    }

    #[test]
    fn linking_a_verified_control_leaves_the_residual_unchanged() {
        let mut t = threat_with(Some((3, 3)), Some((3, 3)));
        let v = control(1, "x", ControlStatus::Verified);
        t.fields.control_ids = vec![v.audit.id];
        assert_eq!(t.fields.residual, Rating::new(3, 3, "r").ok());
    }

    #[test]
    fn lapsed_acceptance_reads_open_and_is_flagged() {
        let mut t = threat_with(None, None);
        t.fields.status = ThreatStatus::Accepted;
        t.fields.treatment = crate::threat::RiskTreatment::Accept;
        t.fields.acceptance_expires = Some(NaiveDate::from_ymd_opt(2026, 10, 16).unwrap());
        assert_eq!(effective_status(&t, today()), ThreatStatus::Open);
        let flags = threat_flags(&t, &HashMap::new(), &[], today());
        assert!(flags.contains(&ThreatFlag::AcceptanceExpired));
        assert!(flags.contains(&ThreatFlag::NoControl));
        t.fields.acceptance_expires = Some(NaiveDate::from_ymd_opt(2026, 10, 18).unwrap());
        assert_eq!(effective_status(&t, today()), ThreatStatus::Accepted);
        assert!(threat_flags(&t, &HashMap::new(), &[], today()).is_empty());
    }
}
