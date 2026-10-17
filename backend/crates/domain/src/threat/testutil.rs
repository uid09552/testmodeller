//! Builders for threat-model unit tests.

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use super::arch::Architecture;
use super::*;
use crate::entities::Audit;

/// Audit fields with a fresh id.
pub fn audit() -> Audit {
    let t = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    Audit {
        id: Uuid::new_v4(),
        version: 1,
        created_at: t,
        updated_at: t,
    }
}

pub fn zone(a: &mut Architecture, name: &str, trust: u8) -> Uuid {
    let z = Zone {
        audit: audit(),
        fields: ZoneFields {
            name: name.into(),
            description: None,
            kind: ZoneKind::Other,
            trust_level: trust,
        },
    };
    let id = z.audit.id;
    a.zones.push(z);
    id
}

pub fn component(
    a: &mut Architecture,
    name: &str,
    t: ArchComponentType,
    zone: Option<Uuid>,
) -> Uuid {
    let c = ArchComponent {
        audit: audit(),
        fields: ArchComponentFields {
            name: name.into(),
            description: None,
            component_type: t,
            owner: None,
            technology: None,
            criticality: None,
            internet_facing: false,
            zone_id: zone,
            linked_component_id: None,
            linked_feature_id: None,
            asset_ids: vec![],
        },
    };
    let id = c.audit.id;
    a.components.push(c);
    id
}

pub fn asset(
    a: &mut Architecture,
    name: &str,
    conf: Option<CiaLevel>,
    integ: Option<CiaLevel>,
) -> Uuid {
    let x = Asset {
        audit: audit(),
        fields: AssetFields {
            name: name.into(),
            description: None,
            category: AssetCategory::BusinessData,
            owner: None,
            confidentiality: conf,
            integrity: integ,
            availability: None,
            personal_data: false,
        },
    };
    let id = x.audit.id;
    a.assets.push(x);
    id
}

pub fn flow(a: &mut Architecture, from: Uuid, to: Uuid) -> Uuid {
    flow_rel(a, from, to, FlowRelationship::Calls)
}

pub fn flow_rel(a: &mut Architecture, from: Uuid, to: Uuid, rel: FlowRelationship) -> Uuid {
    let f = Flow {
        audit: audit(),
        fields: FlowFields {
            source_id: from,
            target_id: to,
            relationship: rel,
            label: None,
            protocol: None,
            bidirectional: false,
            authentication: "unknown".into(),
            encrypted: TriState::Unknown,
            asset_ids: vec![],
        },
    };
    let id = f.audit.id;
    a.flows.push(f);
    id
}

pub fn threat_fields(target: ThreatTarget) -> ThreatFields {
    ThreatFields {
        title: "T".into(),
        category: StrideCategory::Spoofing,
        scenario: None,
        attacker_objective: None,
        preconditions: None,
        consequences: None,
        targets: vec![target],
        asset_ids: vec![],
        control_ids: vec![],
        status: ThreatStatus::Open,
        dismiss_reason: None,
        inherent: None,
        current: None,
        treatment: RiskTreatment::Undecided,
        risk_owner: None,
        due_date: None,
        acceptance_rationale: None,
        accepted_by: None,
    }
}

pub fn threat(number: i32, fields: ThreatFields) -> Threat {
    Threat {
        audit: audit(),
        number,
        fields,
        origin: ThreatOrigin::Manual,
        explanation: None,
    }
}

pub fn control(number: i32, name: &str, status: ControlStatus) -> Control {
    Control {
        audit: audit(),
        number,
        fields: ControlFields {
            name: name.into(),
            description: None,
            control_type: ControlType::Preventive,
            category: ControlCategory::IdentityAccess,
            status,
            owner: None,
            acceptance_criteria: None,
            component_ids: vec![],
            threat_ids: vec![],
        },
        verified_at: None,
    }
}
