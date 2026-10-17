//! STRIDE rules that suggest candidate threats from the architecture.
//! See docs/specification/09-threat-modeling.md#stride-rules-ruleset-version-1.

use std::collections::{BTreeSet, HashSet};

use uuid::Uuid;

use super::arch::{crossing, layer, Architecture, CrossingDirection};
use super::{
    ArchComponent, ArchComponentType, ArchLayer, Asset, AuthState, CiaLevel, Criticality, Flow,
    FlowRelationship, RuleExplanation, StrideCategory, TargetKind, ThreatTarget, TriState,
};

/// Version of the ruleset; stored with every explanation.
pub const RULESET_VERSION: i32 = 1;

/// Reminder added to every explanation.
pub const UNDOCUMENTED_IS_A_QUESTION: &str =
    "An undocumented control is a question to check, not a finding.";

/// A rule match that does not have a threat yet.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// `<ruleId>:<component|flow>:<targetId>`.
    pub rule_key: String,
    /// STRIDE category.
    pub category: StrideCategory,
    /// Title.
    pub title: String,
    /// Scenario description.
    pub scenario: String,
    /// The target.
    pub target: ThreatTarget,
    /// Assets the match involves.
    pub asset_ids: Vec<Uuid>,
    /// Why it was suggested.
    pub explanation: RuleExplanation,
}

/// Result of applying the rules.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Suggestion {
    /// New candidates, in rule order then architecture order.
    pub candidates: Vec<Candidate>,
    /// Matches skipped because their rule key was used before.
    pub skipped: usize,
}

/// Builds a rule key.
pub fn rule_key(rule_id: &str, target: ThreatTarget) -> String {
    format!("{rule_id}:{}:{}", target.kind, target.id)
}

struct Match {
    target: ThreatTarget,
    title: String,
    scenario: String,
    facts: Vec<String>,
    asset_ids: Vec<Uuid>,
}

struct Rule {
    id: &'static str,
    name: &'static str,
    category: StrideCategory,
    missing: &'static str,
    recommendations: &'static [&'static str],
    find: fn(&Architecture) -> Vec<Match>,
}

/// Applies every rule. Matches whose key is in `used_keys` are skipped.
pub fn suggest(arch: &Architecture, used_keys: &HashSet<String>) -> Suggestion {
    let mut out = Suggestion::default();
    for rule in RULES {
        for m in (rule.find)(arch) {
            let key = rule_key(rule.id, m.target);
            if used_keys.contains(&key) {
                out.skipped += 1;
                continue;
            }
            out.candidates.push(Candidate {
                rule_key: key,
                category: rule.category,
                title: m.title,
                scenario: m.scenario,
                target: m.target,
                asset_ids: m.asset_ids,
                explanation: RuleExplanation {
                    rule_id: rule.id.to_owned(),
                    rule_version: RULESET_VERSION,
                    rule_name: rule.name.to_owned(),
                    facts: m.facts,
                    missing: vec![
                        rule.missing.to_owned(),
                        UNDOCUMENTED_IS_A_QUESTION.to_owned(),
                    ],
                    recommendations: rule
                        .recommendations
                        .iter()
                        .map(|r| (*r).to_owned())
                        .collect(),
                },
            });
        }
    }
    out
}

/// Control recommendations of a rule, for "Add as control".
pub fn recommendations(rule_id: &str) -> &'static [&'static str] {
    RULES
        .iter()
        .find(|r| r.id == rule_id)
        .map_or(&[], |r| r.recommendations)
}

const RULES: &[Rule] = &[
    Rule {
        id: "SPOOF-BOUNDARY-AUTH",
        name: "Unauthenticated flow into a more trusted zone",
        category: StrideCategory::Spoofing,
        missing: "Document the authentication method to confirm or dismiss.",
        recommendations: &[
            "Service authentication",
            "Authorization at the receiving component",
        ],
        find: spoof_boundary_auth,
    },
    Rule {
        id: "INFO-BOUNDARY-ENCRYPTION",
        name: "Sensitive data crosses a trust boundary without documented encryption",
        category: StrideCategory::InformationDisclosure,
        missing: "Document encryption in transit to confirm or dismiss.",
        recommendations: &[
            "Encryption in transit (TLS)",
            "Message-level encryption for sensitive fields",
        ],
        find: info_boundary_encryption,
    },
    Rule {
        id: "INFO-PUBLIC-SENSITIVE",
        name: "Internet-facing component handles confidential data",
        category: StrideCategory::InformationDisclosure,
        missing: "Document the authorization policy to confirm or dismiss.",
        recommendations: &[
            "Object-level authorization",
            "Tenant isolation",
            "Access logging and alerting",
        ],
        find: info_public_sensitive,
    },
    Rule {
        id: "TAMPER-DATASTORE",
        name: "Data store holds high-integrity data",
        category: StrideCategory::Tampering,
        missing: "Document write access and integrity protection to confirm or dismiss.",
        recommendations: &[
            "Least-privilege write access",
            "Integrity checks",
            "Protected backups",
        ],
        find: tamper_datastore,
    },
    Rule {
        id: "SPOOF-IDP",
        name: "Shared identity provider",
        category: StrideCategory::Spoofing,
        missing: "Document how the identity provider is protected to confirm or dismiss.",
        recommendations: &[
            "MFA for administrators",
            "Signing-key rotation",
            "Monitoring of authentication anomalies",
        ],
        find: spoof_idp,
    },
    Rule {
        id: "TAMPER-CICD",
        name: "Pipeline deploys to components",
        category: StrideCategory::Tampering,
        missing: "Document build integrity and deployment approvals to confirm or dismiss.",
        recommendations: &[
            "Signed build artifacts",
            "Protected branches and deployment approvals",
            "Secret scanning",
        ],
        find: tamper_cicd,
    },
    Rule {
        id: "DOS-PUBLIC",
        name: "Internet-facing component availability",
        category: StrideCategory::DenialOfService,
        missing: "Document rate limits and capacity to confirm or dismiss.",
        recommendations: &["Rate limiting and quotas", "CDN or WAF in front"],
        find: dos_public,
    },
    Rule {
        id: "REPUD-AUDIT",
        name: "Critical component without audit trail",
        category: StrideCategory::Repudiation,
        missing: "Document audit logging to confirm or dismiss.",
        recommendations: &["Audit logging of security-relevant actions"],
        find: repud_audit,
    },
];

fn comp_target(c: &ArchComponent) -> ThreatTarget {
    ThreatTarget {
        kind: TargetKind::Component,
        id: c.audit.id,
    }
}

fn flow_target(f: &Flow) -> ThreatTarget {
    ThreatTarget {
        kind: TargetKind::Flow,
        id: f.audit.id,
    }
}

fn assets_where<'a>(
    arch: &'a Architecture,
    ids: &[Uuid],
    pred: impl Fn(&Asset) -> bool,
) -> Vec<&'a Asset> {
    ids.iter()
        .filter_map(|id| arch.asset(*id))
        .filter(|a| pred(a))
        .collect()
}

fn conf_high(a: &Asset) -> bool {
    a.fields.confidentiality == Some(CiaLevel::High)
}

fn integ_high(a: &Asset) -> bool {
    a.fields.integrity == Some(CiaLevel::High)
}

fn crossing_fact(arch: &Architecture, f: &Flow) -> String {
    let side = |id| {
        arch.component(id)
            .map_or_else(|| "?".to_owned(), |c| arch.describe_component(c))
    };
    format!(
        "crosses {} → {}",
        side(f.fields.source_id),
        side(f.fields.target_id)
    )
}

fn asset_facts(assets: &[&Asset], attr: &str) -> Vec<String> {
    assets
        .iter()
        .map(|a| format!("asset {}: {attr} high", a.fields.name))
        .collect()
}

fn spoof_boundary_auth(arch: &Architecture) -> Vec<Match> {
    arch.flows
        .iter()
        .filter(|f| {
            crossing(arch, f).direction == Some(CrossingDirection::IntoMoreTrusted)
                && f.fields.auth_state() != AuthState::Documented
        })
        .map(|f| {
            let name = arch.flow_name(f);
            Match {
                target: flow_target(f),
                title: format!("Spoofed caller on {name}"),
                scenario: format!(
                    "A caller from a less trusted zone impersonates the source of {name}, because no authentication of the flow is documented."
                ),
                facts: vec![
                    crossing_fact(arch, f),
                    format!("authentication: {}", auth_text(f)),
                ],
                asset_ids: f.fields.asset_ids.clone(),
            }
        })
        .collect()
}

fn auth_text(f: &Flow) -> &'static str {
    match f.fields.auth_state() {
        AuthState::None => "none",
        AuthState::Unknown => "unknown",
        AuthState::Documented => "documented",
    }
}

fn info_boundary_encryption(arch: &Architecture) -> Vec<Match> {
    arch.flows
        .iter()
        .filter_map(|f| {
            if !crossing(arch, f).crosses || f.fields.encrypted == TriState::Yes {
                return None;
            }
            let sensitive = assets_where(arch, &f.fields.asset_ids, conf_high);
            if sensitive.is_empty() {
                return None;
            }
            let name = arch.flow_name(f);
            let mut facts = vec![
                crossing_fact(arch, f),
                format!("encrypted in transit: {}", f.fields.encrypted),
            ];
            facts.extend(asset_facts(&sensitive, "confidentiality"));
            Some(Match {
                target: flow_target(f),
                title: format!("Interception of confidential data on {name}"),
                scenario: format!(
                    "An attacker on the network path of {name} reads confidential data in transit."
                ),
                facts,
                asset_ids: sensitive.iter().map(|a| a.audit.id).collect(),
            })
        })
        .collect()
}

fn info_public_sensitive(arch: &Architecture) -> Vec<Match> {
    arch.components
        .iter()
        .filter(|c| c.fields.internet_facing)
        .filter_map(|c| {
            let sensitive = assets_where(arch, &c.fields.asset_ids, conf_high);
            if sensitive.is_empty() {
                return None;
            }
            let mut facts = vec![format!("{} is internet-facing", c.fields.name)];
            facts.extend(asset_facts(&sensitive, "confidentiality"));
            Some(Match {
                target: comp_target(c),
                title: format!("Unauthorized data access via {}", c.fields.name),
                scenario: format!(
                    "A user reaches {} from the internet and retrieves data they are not entitled to, e.g. by manipulating a record identifier.",
                    c.fields.name
                ),
                facts,
                asset_ids: sensitive.iter().map(|a| a.audit.id).collect(),
            })
        })
        .collect()
}

fn tamper_datastore(arch: &Architecture) -> Vec<Match> {
    arch.components
        .iter()
        .filter(|c| layer(c.fields.component_type) == ArchLayer::Data)
        .filter_map(|c| {
            let assets = assets_where(arch, &c.fields.asset_ids, integ_high);
            if assets.is_empty() {
                return None;
            }
            let mut facts = vec![format!("{} is a data store ({})", c.fields.name, c.fields.component_type)];
            facts.extend(asset_facts(&assets, "integrity"));
            Some(Match {
                target: comp_target(c),
                title: format!("Unauthorized modification of data in {}", c.fields.name),
                scenario: format!(
                    "Someone with access to {} alters data whose integrity matters, without the change being prevented or detected.",
                    c.fields.name
                ),
                facts,
                asset_ids: assets.iter().map(|a| a.audit.id).collect(),
            })
        })
        .collect()
}

fn spoof_idp(arch: &Architecture) -> Vec<Match> {
    arch.components
        .iter()
        .filter(|c| c.fields.component_type == ArchComponentType::IdentityProvider)
        .filter_map(|idp| {
            let dependants: BTreeSet<String> = arch
                .flows
                .iter()
                .filter(|f| {
                    f.fields.target_id == idp.audit.id
                        && f.fields.relationship == FlowRelationship::AuthenticatesThrough
                })
                .filter_map(|f| arch.component(f.fields.source_id))
                .map(|c| c.fields.name.clone())
                .collect();
            if dependants.len() < 2 {
                return None;
            }
            let list = dependants.into_iter().collect::<Vec<_>>().join(", ");
            Some(Match {
                target: comp_target(idp),
                title: format!("Identity provider compromise affects dependants of {}", idp.fields.name),
                scenario: format!(
                    "An attacker who compromises {} can impersonate users of every component that authenticates through it: {list}.",
                    idp.fields.name
                ),
                facts: vec![
                    format!("{} is an identity provider", idp.fields.name),
                    format!("dependants: {list}"),
                ],
                asset_ids: idp.fields.asset_ids.clone(),
            })
        })
        .collect()
}

fn tamper_cicd(arch: &Architecture) -> Vec<Match> {
    arch.components
        .iter()
        .filter(|c| c.fields.component_type == ArchComponentType::CicdPipeline)
        .filter_map(|p| {
            let targets: BTreeSet<String> = arch
                .flows
                .iter()
                .filter(|f| {
                    f.fields.source_id == p.audit.id && f.fields.relationship == FlowRelationship::Deploys
                })
                .filter_map(|f| arch.component(f.fields.target_id))
                .map(|c| c.fields.name.clone())
                .collect();
            if targets.is_empty() {
                return None;
            }
            let list = targets.into_iter().collect::<Vec<_>>().join(", ");
            Some(Match {
                target: comp_target(p),
                title: format!("Build or deployment integrity of {}", p.fields.name),
                scenario: format!(
                    "An attacker who can change what {} builds or deploys ships malicious code to {list}.",
                    p.fields.name
                ),
                facts: vec![
                    format!("{} is a CI/CD pipeline", p.fields.name),
                    format!("deploys to: {list}"),
                ],
                asset_ids: p.fields.asset_ids.clone(),
            })
        })
        .collect()
}

fn dos_public(arch: &Architecture) -> Vec<Match> {
    arch.components
        .iter()
        .filter(|c| c.fields.internet_facing)
        .map(|c| Match {
            target: comp_target(c),
            title: format!("Denial of service against {}", c.fields.name),
            scenario: format!(
                "An attacker floods {} from the internet until legitimate users cannot use it.",
                c.fields.name
            ),
            facts: vec![format!("{} is internet-facing", c.fields.name)],
            asset_ids: vec![],
        })
        .collect()
}

fn repud_audit(arch: &Architecture) -> Vec<Match> {
    arch.components
        .iter()
        .filter(|c| matches!(c.fields.criticality, Some(Criticality::High | Criticality::Critical)))
        .filter(|c| {
            !arch.flows.iter().any(|f| {
                f.fields.source_id == c.audit.id
                    && f.fields.relationship == FlowRelationship::SendsData
                    && arch
                        .component(f.fields.target_id)
                        .is_some_and(|t| t.fields.component_type == ArchComponentType::AuditService)
            })
        })
        .map(|c| Match {
            target: comp_target(c),
            title: format!("Actions on {} cannot be traced", c.fields.name),
            scenario: format!(
                "A user performs a security-relevant action on {} and later denies it, because no audit trail is documented.",
                c.fields.name
            ),
            facts: vec![
                format!("{} has criticality {}", c.fields.name, c.fields.criticality.map_or("?", |x| x.as_str())),
                "no sends-data flow to an audit service".to_owned(),
            ],
            asset_ids: vec![],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::testutil::*;
    use super::*;

    fn ids(s: &Suggestion) -> Vec<&str> {
        s.candidates
            .iter()
            .map(|c| c.explanation.rule_id.as_str())
            .collect()
    }

    fn none() -> HashSet<String> {
        HashSet::new()
    }

    /// Web app (Internet 0) → Customer API (Internal 3).
    fn boundary() -> (Architecture, Uuid) {
        let mut a = Architecture::default();
        let z0 = zone(&mut a, "Internet", 0);
        let z3 = zone(&mut a, "Internal", 3);
        let w = component(&mut a, "Web app", ArchComponentType::WebFrontend, Some(z0));
        let api = component(&mut a, "Customer API", ArchComponentType::Api, Some(z3));
        let f = flow(&mut a, w, api);
        (a, f)
    }

    #[test]
    fn unauthenticated_boundary_crossing_is_spoofing_on_the_flow() {
        let (a, f) = boundary();
        let s = suggest(&a, &none());
        assert_eq!(ids(&s), ["SPOOF-BOUNDARY-AUTH"]);
        let c = &s.candidates[0];
        assert_eq!(c.category, StrideCategory::Spoofing);
        assert_eq!(
            c.target,
            ThreatTarget {
                kind: TargetKind::Flow,
                id: f
            }
        );
        assert_eq!(
            c.explanation.rule_name,
            "Unauthenticated flow into a more trusted zone"
        );
        assert_eq!(
            c.explanation.facts,
            [
                "crosses Web app (Internet, trust 0) → Customer API (Internal, trust 3)",
                "authentication: unknown"
            ]
        );
        assert_eq!(
            c.explanation.missing[0],
            "Document the authentication method to confirm or dismiss."
        );
        assert_eq!(c.explanation.missing[1], UNDOCUMENTED_IS_A_QUESTION);
        assert_eq!(c.explanation.rule_version, RULESET_VERSION);
    }

    #[test]
    fn documented_auth_or_outbound_crossing_is_not_spoofing() {
        let (mut a, _) = boundary();
        a.flows[0].fields.authentication = "OAuth2 bearer".into();
        assert!(suggest(&a, &none()).candidates.is_empty());
        let (mut a, _) = boundary();
        let f = &mut a.flows[0].fields;
        std::mem::swap(&mut f.source_id, &mut f.target_id);
        assert!(!ids(&suggest(&a, &none())).contains(&"SPOOF-BOUNDARY-AUTH"));
    }

    #[test]
    fn unencrypted_crossing_with_sensitive_data_is_disclosure() {
        let (mut a, f) = boundary();
        a.flows[0].fields.authentication = "mTLS".into();
        let rec = asset(&mut a, "Customer records", Some(CiaLevel::High), None);
        a.flows[0].fields.asset_ids = vec![rec];
        let s = suggest(&a, &none());
        assert_eq!(ids(&s), ["INFO-BOUNDARY-ENCRYPTION"]);
        assert_eq!(s.candidates[0].target.id, f);
        assert_eq!(s.candidates[0].asset_ids, [rec]);
        a.flows[0].fields.encrypted = TriState::Yes;
        assert!(suggest(&a, &none()).candidates.is_empty());
    }

    #[test]
    fn internet_facing_with_sensitive_asset_is_unauthorized_access_and_dos() {
        let mut a = Architecture::default();
        let c = component(&mut a, "Customer API", ArchComponentType::Api, None);
        a.components[0].fields.internet_facing = true;
        let rec = asset(&mut a, "Customer records", Some(CiaLevel::High), None);
        a.components[0].fields.asset_ids = vec![rec];
        let s = suggest(&a, &none());
        assert_eq!(ids(&s), ["INFO-PUBLIC-SENSITIVE", "DOS-PUBLIC"]);
        assert_eq!(
            s.candidates[0].title,
            "Unauthorized data access via Customer API"
        );
        assert!(s.candidates.iter().all(|x| x.target.id == c));
    }

    #[test]
    fn data_store_with_high_integrity_asset_is_tampering() {
        let mut a = Architecture::default();
        component(
            &mut a,
            "Orders DB",
            ArchComponentType::RelationalDatabase,
            None,
        );
        let o = asset(&mut a, "Orders", None, Some(CiaLevel::High));
        a.components[0].fields.asset_ids = vec![o];
        assert_eq!(ids(&suggest(&a, &none())), ["TAMPER-DATASTORE"]);
        a.components[0].fields.component_type = ArchComponentType::Service;
        assert!(suggest(&a, &none()).candidates.is_empty());
    }

    #[test]
    fn shared_identity_provider_names_dependants() {
        let mut a = Architecture::default();
        let idp = component(&mut a, "IdP", ArchComponentType::IdentityProvider, None);
        let w = component(&mut a, "Web app", ArchComponentType::WebFrontend, None);
        let adm = component(&mut a, "Admin portal", ArchComponentType::WebFrontend, None);
        flow_rel(&mut a, w, idp, FlowRelationship::AuthenticatesThrough);
        assert!(suggest(&a, &none()).candidates.is_empty());
        flow_rel(&mut a, adm, idp, FlowRelationship::AuthenticatesThrough);
        let s = suggest(&a, &none());
        assert_eq!(ids(&s), ["SPOOF-IDP"]);
        assert!(s.candidates[0]
            .explanation
            .facts
            .contains(&"dependants: Admin portal, Web app".to_owned()));
    }

    #[test]
    fn pipeline_with_deploy_flow_is_tampering() {
        let mut a = Architecture::default();
        let p = component(&mut a, "Pipeline", ArchComponentType::CicdPipeline, None);
        let s = component(&mut a, "Order service", ArchComponentType::Service, None);
        flow_rel(&mut a, p, s, FlowRelationship::Calls);
        assert!(suggest(&a, &none()).candidates.is_empty());
        flow_rel(&mut a, p, s, FlowRelationship::Deploys);
        assert_eq!(ids(&suggest(&a, &none())), ["TAMPER-CICD"]);
    }

    #[test]
    fn critical_component_without_audit_flow_is_repudiation() {
        let mut a = Architecture::default();
        let c = component(&mut a, "Payments", ArchComponentType::Service, None);
        a.components[0].fields.criticality = Some(Criticality::High);
        assert_eq!(ids(&suggest(&a, &none())), ["REPUD-AUDIT"]);
        let au = component(&mut a, "Audit", ArchComponentType::AuditService, None);
        flow_rel(&mut a, c, au, FlowRelationship::SendsData);
        assert!(suggest(&a, &none()).candidates.is_empty());
    }

    #[test]
    fn used_keys_are_skipped_including_dismissed_and_deleted() {
        let (a, _) = boundary();
        let first = suggest(&a, &none());
        let used: HashSet<String> = first
            .candidates
            .iter()
            .map(|c| c.rule_key.clone())
            .collect();
        let second = suggest(&a, &used);
        assert!(second.candidates.is_empty());
        assert_eq!(second.skipped, 1);
    }

    #[test]
    fn recommendations_by_rule() {
        assert_eq!(recommendations("SPOOF-BOUNDARY-AUTH").len(), 2);
        assert!(recommendations("NOPE").is_empty());
    }
}
