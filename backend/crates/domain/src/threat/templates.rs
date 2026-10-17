//! Security test templates and generation from threats and controls.
//! See docs/specification/09-threat-modeling.md#security-test-generation-template-version-1.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::arch::Architecture;
use super::{
    Control, SecurityTestFields, SecurityTestStatus, SecurityTestStep, SecurityTestType,
    StrideCategory, TargetKind, TemplateSource, Threat, ThreatStatus,
};

/// Version of the templates; stored with every generated test.
pub const TEMPLATE_VERSION: i32 = 1;

/// A test to be created.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSecurityTest {
    /// Fields, status draft.
    pub fields: SecurityTestFields,
    /// Where it came from.
    pub source: TemplateSource,
}

/// Result of generation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Generated {
    /// New tests.
    pub tests: Vec<NewSecurityTest>,
    /// Combinations that already have a non-obsolete test.
    pub skipped: usize,
}

struct Ctx<'a> {
    target: &'a str,
    source: &'a str,
    asset: &'a str,
}

struct Template {
    id: &'static str,
    title: &'static str,
    category: StrideCategory,
    /// `None` matches any target kind.
    kind: Option<TargetKind>,
    test_type: SecurityTestType,
    steps: fn(&Ctx) -> Vec<(String, String)>,
}

const TEMPLATES: &[Template] = &[
    Template {
        id: "AUTH-FLOW",
        title: "Reject unauthenticated calls",
        category: StrideCategory::Spoofing,
        kind: Some(TargetKind::Flow),
        test_type: SecurityTestType::Authentication,
        steps: |c| {
            vec![
                (
                    format!("Call {} from {} without credentials", c.target, c.source),
                    "The request is rejected".into(),
                ),
                (
                    format!(
                        "Call {} from {} with invalid or expired credentials",
                        c.target, c.source
                    ),
                    "The request is rejected".into(),
                ),
                (
                    format!("Call {} from {} with valid credentials", c.target, c.source),
                    "The request is accepted".into(),
                ),
            ]
        },
    },
    Template {
        id: "AUTH-COMPONENT",
        title: "Deny access without valid identity",
        category: StrideCategory::Spoofing,
        kind: Some(TargetKind::Component),
        test_type: SecurityTestType::Authentication,
        steps: |c| {
            vec![
                (
                    format!("Access {} without credentials", c.target),
                    "Access is denied".into(),
                ),
                (
                    format!("Access {} with another identity's credentials", c.target),
                    "Access is denied".into(),
                ),
            ]
        },
    },
    Template {
        id: "INTEGRITY-FLOW",
        title: "Reject modified or replayed messages",
        category: StrideCategory::Tampering,
        kind: Some(TargetKind::Flow),
        test_type: SecurityTestType::TransportSecurity,
        steps: |c| {
            vec![
                (
                    format!(
                        "Modify a message from {} to {} in transit",
                        c.source, c.target
                    ),
                    format!("{} rejects the message", c.target),
                ),
                (
                    format!(
                        "Replay a captured message from {} to {}",
                        c.source, c.target
                    ),
                    format!("{} rejects the message", c.target),
                ),
            ]
        },
    },
    Template {
        id: "INTEGRITY-COMPONENT",
        title: "Prevent unauthorized modification",
        category: StrideCategory::Tampering,
        kind: Some(TargetKind::Component),
        test_type: SecurityTestType::NegativeAuthorization,
        steps: |c| {
            vec![
                (
                    format!(
                        "Modify {} in {} without write permission",
                        c.asset, c.target
                    ),
                    "The change is denied".into(),
                ),
                (
                    format!("Make a permitted change to {} in {}", c.asset, c.target),
                    "The change is recorded with its author".into(),
                ),
            ]
        },
    },
    Template {
        id: "AUDIT",
        title: "Record security-relevant actions",
        category: StrideCategory::Repudiation,
        kind: None,
        test_type: SecurityTestType::LoggingAudit,
        steps: |c| {
            vec![
                (
                    format!("Perform a security-relevant action on {}", c.target),
                    "An audit record with actor, time and action exists".into(),
                ),
                (
                    "As that actor, try to change or delete the audit record".into(),
                    "The change is denied".into(),
                ),
            ]
        },
    },
    Template {
        id: "TRANSPORT-FLOW",
        title: "Encrypt data in transit",
        category: StrideCategory::InformationDisclosure,
        kind: Some(TargetKind::Flow),
        test_type: SecurityTestType::TransportSecurity,
        steps: |c| {
            vec![
                (
                    format!("Observe traffic from {} to {}", c.source, c.target),
                    "The payload is encrypted".into(),
                ),
                (
                    format!(
                        "Connect from {} to {} without encryption",
                        c.source, c.target
                    ),
                    "The connection is refused".into(),
                ),
            ]
        },
    },
    Template {
        id: "ACCESS-COMPONENT",
        title: "Deny access to other users' data",
        category: StrideCategory::InformationDisclosure,
        kind: Some(TargetKind::Component),
        test_type: SecurityTestType::NegativeAuthorization,
        steps: |c| {
            vec![
                (
                    format!(
                        "As user A, request {} belonging to user B from {}",
                        c.asset, c.target
                    ),
                    "Access is denied".into(),
                ),
                (
                    "Inspect the response".into(),
                    "It contains none of user B's data".into(),
                ),
            ]
        },
    },
    Template {
        id: "RATE-LIMIT",
        title: "Throttle excessive requests",
        category: StrideCategory::DenialOfService,
        kind: None,
        test_type: SecurityTestType::Resilience,
        steps: |c| {
            vec![
                (
                    format!(
                        "Send requests to {} above the documented rate limit",
                        c.target
                    ),
                    "Excess requests are throttled".into(),
                ),
                (
                    "Meanwhile send a normal request as another client".into(),
                    "It succeeds".into(),
                ),
            ]
        },
    },
    Template {
        id: "PRIVILEGE",
        title: "Deny privileged operations to ordinary users",
        category: StrideCategory::ElevationOfPrivilege,
        kind: None,
        test_type: SecurityTestType::NegativeAuthorization,
        steps: |c| {
            vec![
                (
                    format!(
                        "As a low-privilege user, call an administrative operation of {}",
                        c.target
                    ),
                    "It is denied".into(),
                ),
                ("Try to change your own role".into(), "It is denied".into()),
            ]
        },
    },
];

fn template_for(category: StrideCategory, kind: TargetKind) -> Option<&'static Template> {
    TEMPLATES
        .iter()
        .find(|t| t.category == category && t.kind.is_none_or(|k| k == kind))
}

/// Generates draft tests for the given threats (09-threat-modeling.md).
/// Only open and accepted threats are used. `existing_keys` are the template
/// keys of non-obsolete tests.
pub fn generate(
    threats: &[&Threat],
    arch: &Architecture,
    controls: &HashMap<Uuid, &Control>,
    existing_keys: &HashSet<String>,
) -> Generated {
    let mut out = Generated::default();
    let mut seen = existing_keys.clone();
    for threat in threats {
        let f = &threat.fields;
        if !matches!(f.status, ThreatStatus::Open | ThreatStatus::Accepted) {
            continue;
        }
        let Some(target) = f.targets.first() else {
            continue;
        };
        let Some(tpl) = template_for(f.category, target.kind) else {
            continue;
        };
        let (target_name, source_name) = match target.kind {
            TargetKind::Component => (
                arch.component(target.id)
                    .map_or("the component".to_owned(), |c| c.fields.name.clone()),
                String::new(),
            ),
            TargetKind::Flow => arch.flow(target.id).map_or(
                ("the target".to_owned(), "the source".to_owned()),
                |fl| {
                    let n = |id| {
                        arch.component(id)
                            .map_or("?".to_owned(), |c| c.fields.name.clone())
                    };
                    (n(fl.fields.target_id), n(fl.fields.source_id))
                },
            ),
        };
        let asset_name = f
            .asset_ids
            .iter()
            .find_map(|id| arch.asset(*id))
            .map_or("protected data".to_owned(), |a| a.fields.name.clone());
        let ctx = Ctx {
            target: &target_name,
            source: &source_name,
            asset: &asset_name,
        };

        let linked: Vec<&Control> = f
            .control_ids
            .iter()
            .filter_map(|id| controls.get(id).copied())
            .collect();
        let per: Vec<Option<&Control>> = if linked.is_empty() {
            vec![None]
        } else {
            linked.into_iter().map(Some).collect()
        };
        for control in per {
            let source = TemplateSource {
                template_id: tpl.id.to_owned(),
                template_version: TEMPLATE_VERSION,
                threat_id: threat.audit.id,
                control_id: control.map(|c| c.audit.id),
            };
            if !seen.insert(source.key()) {
                out.skipped += 1;
                continue;
            }
            let mut steps: Vec<SecurityTestStep> = (tpl.steps)(&ctx)
                .into_iter()
                .map(|(action, expected)| SecurityTestStep { action, expected })
                .collect();
            let mut title = format!("{}: {}", tpl.title, f.title);
            if let Some(c) = control {
                title.push_str(&format!(" ({})", c.display_id()));
                steps.push(SecurityTestStep {
                    action: format!("Verify control {} {}", c.display_id(), c.fields.name),
                    expected: c
                        .fields
                        .acceptance_criteria
                        .clone()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or_else(|| "The control is in place".to_owned()),
                });
            }
            out.tests.push(NewSecurityTest {
                fields: SecurityTestFields {
                    title,
                    objective: Some(format!(
                        "Verify that threat {} cannot be realised.",
                        threat.display_id()
                    )),
                    preconditions: vec![format!("Threat {} is in scope", threat.display_id())],
                    steps,
                    test_type: tpl.test_type,
                    status: SecurityTestStatus::Draft,
                    threat_ids: vec![threat.audit.id],
                    control_ids: control.map(|c| vec![c.audit.id]).unwrap_or_default(),
                },
                source,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::testutil::*;
    use super::super::*;
    use super::*;

    fn api_arch() -> (Architecture, Uuid, Uuid, Uuid) {
        let mut a = Architecture::default();
        let w = component(&mut a, "Web app", ArchComponentType::WebFrontend, None);
        let api = component(&mut a, "Customer API", ArchComponentType::Api, None);
        let f = flow(&mut a, w, api);
        let rec = asset(&mut a, "Customer records", Some(CiaLevel::High), None);
        (a, api, f, rec)
    }

    #[test]
    fn spoofing_on_a_flow_calls_without_and_with_invalid_credentials() {
        let (a, _, f, _) = api_arch();
        let mut t = threat(
            1,
            threat_fields(ThreatTarget {
                kind: TargetKind::Flow,
                id: f,
            }),
        );
        let c = control(1, "Service authentication", ControlStatus::Planned);
        t.fields.control_ids = vec![c.audit.id];
        let controls = HashMap::from([(c.audit.id, &c)]);
        let g = generate(&[&t], &a, &controls, &HashSet::new());
        assert_eq!(g.tests.len(), 1);
        let test = &g.tests[0];
        assert_eq!(test.fields.status, SecurityTestStatus::Draft);
        assert_eq!(test.fields.test_type, SecurityTestType::Authentication);
        assert_eq!(
            test.fields.steps[0].action,
            "Call Customer API from Web app without credentials"
        );
        assert_eq!(test.fields.steps[0].expected, "The request is rejected");
        assert!(test.fields.steps[1].action.contains("invalid"));
        assert_eq!(
            test.fields.steps.last().unwrap().action,
            "Verify control CTL-1 Service authentication"
        );
        assert_eq!(test.fields.threat_ids, [t.audit.id]);
        assert_eq!(test.fields.control_ids, [c.audit.id]);
        assert_eq!(test.source.template_id, "AUTH-FLOW");
        assert!(test.fields.title.ends_with("(CTL-1)"));
    }

    #[test]
    fn disclosure_on_component_requests_another_users_asset() {
        let (a, api, _, rec) = api_arch();
        let mut fields = threat_fields(ThreatTarget {
            kind: TargetKind::Component,
            id: api,
        });
        fields.category = StrideCategory::InformationDisclosure;
        fields.asset_ids = vec![rec];
        let t = threat(2, fields);
        let g = generate(&[&t], &a, &HashMap::new(), &HashSet::new());
        let test = &g.tests[0];
        assert_eq!(
            test.fields.test_type,
            SecurityTestType::NegativeAuthorization
        );
        assert_eq!(
            test.fields.steps[0].action,
            "As user A, request Customer records belonging to user B from Customer API"
        );
        assert_eq!(test.fields.steps[0].expected, "Access is denied");
        assert!(test.fields.control_ids.is_empty());
    }

    #[test]
    fn one_test_per_control_and_no_duplicates() {
        let (a, api, _, _) = api_arch();
        let mut t = threat(
            1,
            threat_fields(ThreatTarget {
                kind: TargetKind::Component,
                id: api,
            }),
        );
        let cs: Vec<Control> = (1..=3)
            .map(|n| control(n, "C", ControlStatus::Planned))
            .collect();
        t.fields.control_ids = cs.iter().map(|c| c.audit.id).collect();
        let map: HashMap<Uuid, &Control> = cs.iter().map(|c| (c.audit.id, c)).collect();
        let g = generate(&[&t], &a, &map, &HashSet::new());
        assert_eq!(g.tests.len(), 3);
        let keys: HashSet<String> = g.tests.iter().map(|x| x.source.key()).collect();
        let again = generate(&[&t], &a, &map, &keys);
        assert!(again.tests.is_empty());
        assert_eq!(again.skipped, 3);
    }

    #[test]
    fn candidates_and_dismissed_are_excluded() {
        let (a, api, _, _) = api_arch();
        let mut c = threat(
            1,
            threat_fields(ThreatTarget {
                kind: TargetKind::Component,
                id: api,
            }),
        );
        c.fields.status = ThreatStatus::Candidate;
        let mut d = c.clone();
        d.fields.status = ThreatStatus::Dismissed;
        let mut acc = c.clone();
        acc.fields.status = ThreatStatus::Accepted;
        let g = generate(&[&c, &d, &acc], &a, &HashMap::new(), &HashSet::new());
        assert_eq!(g.tests.len(), 1);
    }

    #[test]
    fn every_category_has_a_template_for_each_target_kind() {
        use StrideCategory as S;
        for cat in [
            S::Spoofing,
            S::Tampering,
            S::Repudiation,
            S::InformationDisclosure,
            S::DenialOfService,
            S::ElevationOfPrivilege,
        ] {
            for kind in [TargetKind::Component, TargetKind::Flow] {
                assert!(template_for(cat, kind).is_some(), "{cat} / {kind}");
            }
        }
    }
}
