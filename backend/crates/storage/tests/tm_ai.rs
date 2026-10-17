//! AI proposals of a threat model against a real PostgreSQL. Requires Docker.

use serde_json::json;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tm_domain::threat::*;
use tm_storage::{
    ControlApply, NamedFields, NewArchitecture, NewComponent, NewFlow, NewTmProposal, StorageError,
    Store,
};
use uuid::Uuid;

async fn store() -> (ContainerAsync<Postgres>, Store) {
    let c = Postgres::default()
        .with_tag("17-alpine")
        .start()
        .await
        .expect("start PostgreSQL; is Docker running?");
    let url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        c.get_host().await.unwrap(),
        c.get_host_port_ipv4(5432).await.unwrap()
    );
    (c, Store::connect(&url, 4).await.unwrap())
}

async fn model(s: &Store) -> Uuid {
    let p = s
        .create_project(
            "t",
            NamedFields {
                name: "P".into(),
                description: None,
            },
        )
        .await
        .unwrap()
        .audit
        .id;
    s.create_threat_model(
        p,
        ThreatModelFields {
            name: "TM".into(),
            ..Default::default()
        },
        Thresholds::default(),
    )
    .await
    .unwrap()
    .audit
    .id
}

fn zone(name: &str, trust: u8) -> ZoneFields {
    ZoneFields {
        name: name.into(),
        description: None,
        kind: ZoneKind::Other,
        trust_level: trust,
    }
}

fn comp(name: &str, zone: Option<Uuid>) -> ArchComponentFields {
    ArchComponentFields {
        name: name.into(),
        description: None,
        component_type: ArchComponentType::Api,
        owner: None,
        technology: None,
        criticality: None,
        internet_facing: false,
        zone_id: zone,
        linked_component_id: None,
        linked_feature_id: None,
        asset_ids: vec![],
    }
}

fn asset(name: &str) -> AssetFields {
    AssetFields {
        name: name.into(),
        description: None,
        category: AssetCategory::BusinessData,
        owner: None,
        confidentiality: None,
        integrity: None,
        availability: None,
        personal_data: false,
        classification: None,
        data_location: None,
        retention: None,
        encryption_at_rest: TriState::Unknown,
    }
}

fn flow() -> FlowFields {
    FlowFields {
        source_id: Uuid::nil(),
        target_id: Uuid::nil(),
        relationship: FlowRelationship::Calls,
        label: None,
        protocol: None,
        bidirectional: false,
        authentication: "unknown".into(),
        encrypted: TriState::Unknown,
        asset_ids: vec![],
        data_classification: None,
        authorization_model: None,
        entry_point: false,
    }
}

fn threat_fields(target: Uuid) -> ThreatFields {
    ThreatFields {
        title: "Spoofed session".into(),
        category: StrideCategory::Spoofing,
        scenario: None,
        attacker_objective: None,
        preconditions: None,
        consequences: None,
        targets: vec![ThreatTarget {
            kind: TargetKind::Component,
            id: target,
        }],
        asset_ids: vec![],
        control_ids: vec![],
        status: ThreatStatus::Open,
        dismiss_reason: None,
        inherent: None,
        current: None,
        residual: None,
        threat_source: None,
        detection: None,
        existing_mitigations: None,
        confidence: None,
        treatment: RiskTreatment::Undecided,
        risk_owner: None,
        due_date: None,
        acceptance_rationale: None,
        accepted_by: None,
        acceptance_expires: None,
    }
}

fn control_fields(threats: Vec<Uuid>) -> ControlFields {
    ControlFields {
        name: "Rate limiting".into(),
        description: None,
        control_type: ControlType::Preventive,
        category: ControlCategory::ApplicationNetwork,
        status: ControlStatus::Planned,
        owner: None,
        acceptance_criteria: None,
        review_by: None,
        component_ids: vec![],
        threat_ids: threats,
    }
}

fn new_proposal(kind: &str) -> NewTmProposal {
    NewTmProposal {
        kind: kind.into(),
        payload: json!({"x": 1}),
        rationale: Some("because".into()),
        source: Some("fake-model".into()),
        prompt_version: Some("v1".into()),
        target_ids: vec![],
    }
}

async fn one(s: &Store, tm: Uuid, kind: &str) -> Uuid {
    s.create_tm_proposals(tm, vec![new_proposal(kind)])
        .await
        .unwrap()[0]
        .id
}

#[tokio::test]
async fn ai_is_off_by_default_and_the_flag_is_saved_with_the_threat_model() {
    let (_c, s) = store().await;
    let tm = model(&s).await;
    assert!(!s.get_threat_model(tm).await.unwrap().fields.ai_enabled);
    let m = s
        .update_threat_model(
            tm,
            1,
            ThreatModelFields {
                name: "TM".into(),
                ai_enabled: true,
                ..Default::default()
            },
            Thresholds::default(),
        )
        .await
        .unwrap();
    assert!(m.fields.ai_enabled);
    assert!(matches!(
        s.update_threat_model(tm, 1, ThreatModelFields::default(), Thresholds::default())
            .await,
        Err(StorageError::VersionMismatch)
    ));
}

#[tokio::test]
async fn proposals_are_listed_filtered_scoped_and_rejected_once() {
    let (_c, s) = store().await;
    let tm = model(&s).await;
    let other = model(&s).await;
    let a = one(&s, tm, "tm-threats").await;
    let b = one(&s, tm, "tm-ratings").await;
    let foreign = one(&s, other, "tm-threats").await;

    assert_eq!(s.list_tm_proposals(tm, None, None).await.unwrap().len(), 2);
    assert_eq!(
        s.list_tm_proposals(tm, None, Some("tm-ratings"))
            .await
            .unwrap()[0]
            .id,
        b
    );
    assert!(matches!(
        s.get_tm_proposal(tm, foreign).await,
        Err(StorageError::NotFound(_))
    ));

    let r = s
        .reject_tm_proposal(tm, a, Some("noise".into()))
        .await
        .unwrap();
    assert_eq!(
        (r.status.as_str(), r.reject_reason.as_deref()),
        ("rejected", Some("noise"))
    );
    assert!(matches!(
        s.reject_tm_proposal(tm, a, None).await,
        Err(StorageError::Conflict(_))
    ));
    assert_eq!(
        s.list_tm_proposals(tm, Some("pending"), None)
            .await
            .unwrap()
            .len(),
        1
    );
    // Nothing was created by rejecting.
    assert!(s.load_threat_model(tm).await.unwrap().threats.is_empty());

    s.delete_threat_model(tm).await.unwrap();
    assert!(s
        .list_tm_proposals(tm, None, None)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn an_architecture_proposal_creates_everything_with_provenance_and_resolves_names() {
    let (_c, s) = store().await;
    let tm = model(&s).await;
    let existing_zone = s.create_tm_zone(tm, &zone("Internal", 3)).await.unwrap();
    let existing_api = s
        .create_tm_component(tm, &comp("API", Some(existing_zone)))
        .await
        .unwrap();
    let existing_asset = s.create_tm_asset(tm, &asset("Records")).await.unwrap();
    let p = one(&s, tm, "tm-architecture").await;

    let arch = NewArchitecture {
        zones: vec![zone("DMZ", 1)],
        assets: vec![asset("Orders")],
        components: vec![NewComponent {
            fields: comp("Gateway", None),
            zone: Some("dmz".into()),
            assets: vec!["Orders".into(), "records".into()],
        }],
        flows: vec![NewFlow {
            fields: flow(),
            source: "Gateway".into(),
            target: "api".into(),
            assets: vec!["Orders".into()],
        }],
    };
    let row = s
        .apply_tm_architecture(tm, p, &json!({"edited": true}), arch, "fake-model")
        .await
        .unwrap();
    assert_eq!(row.status, "accepted");
    assert_eq!(row.resulting_entity_ids.len(), 4);
    assert_eq!(
        row.payload,
        json!({"edited": true}),
        "the payload that was applied is kept"
    );

    let agg = s.load_threat_model(tm).await.unwrap();
    let gw = agg
        .arch
        .components
        .iter()
        .find(|c| c.fields.name == "Gateway")
        .unwrap();
    assert_eq!(gw.provenance.origin, AiOrigin::Ai);
    assert_eq!(gw.provenance.source.as_deref(), Some("fake-model"));
    assert_eq!(gw.fields.asset_ids.len(), 2);
    assert!(gw.fields.asset_ids.contains(&existing_asset));
    let dmz = agg
        .arch
        .zones
        .iter()
        .find(|z| z.fields.name == "DMZ")
        .unwrap();
    assert_eq!(gw.fields.zone_id, Some(dmz.audit.id));
    assert_eq!(dmz.provenance.origin, AiOrigin::Ai);
    let f = &agg.arch.flows[0];
    assert_eq!(
        (f.fields.source_id, f.fields.target_id),
        (gw.audit.id, existing_api)
    );
    assert_eq!(f.provenance.origin, AiOrigin::Ai);
    // What existed before is untouched and manual.
    let api = agg.arch.component(existing_api).unwrap();
    assert_eq!(api.provenance.origin, AiOrigin::Manual);
    assert!(api.provenance.source.is_none());

    // Accepted once.
    assert!(matches!(
        s.apply_tm_architecture(tm, p, &json!({}), NewArchitecture::default(), "m")
            .await,
        Err(StorageError::Conflict(_))
    ));
}

#[tokio::test]
async fn a_failing_architecture_proposal_creates_nothing_and_stays_pending() {
    let (_c, s) = store().await;
    let tm = model(&s).await;
    let p = one(&s, tm, "tm-architecture").await;
    let arch = NewArchitecture {
        zones: vec![zone("DMZ", 1)],
        assets: vec![],
        components: vec![NewComponent {
            fields: comp("Gateway", None),
            zone: Some("Moon".into()),
            assets: vec![],
        }],
        flows: vec![],
    };
    let e = s
        .apply_tm_architecture(tm, p, &json!({}), arch, "m")
        .await
        .unwrap_err();
    assert!(matches!(e, StorageError::Invalid(m) if m.contains("zone \"Moon\"")));
    let agg = s.load_threat_model(tm).await.unwrap();
    assert!(
        agg.arch.zones.is_empty() && agg.arch.components.is_empty(),
        "rolled back"
    );
    assert_eq!(s.get_tm_proposal(tm, p).await.unwrap().status, "pending");

    // A name that already exists is refused the same way.
    s.create_tm_zone(tm, &zone("DMZ", 1)).await.unwrap();
    let dup = NewArchitecture {
        zones: vec![zone("dmz", 1)],
        ..Default::default()
    };
    assert!(s
        .apply_tm_architecture(tm, p, &json!({}), dup, "m")
        .await
        .is_err());
    assert_eq!(s.get_tm_proposal(tm, p).await.unwrap().status, "pending");
}

#[tokio::test]
async fn a_threat_proposal_becomes_an_open_ai_threat_unless_its_target_is_gone() {
    let (_c, s) = store().await;
    let tm = model(&s).await;
    let api = s.create_tm_component(tm, &comp("API", None)).await.unwrap();
    let p = one(&s, tm, "tm-threats").await;
    let row = s
        .apply_tm_threat(tm, p, &json!({}), &threat_fields(api), "fake-model")
        .await
        .unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let t = &agg.threats[0];
    assert_eq!(row.resulting_entity_ids, vec![t.audit.id]);
    assert_eq!(t.origin, ThreatOrigin::Ai);
    assert_eq!(t.source_model.as_deref(), Some("fake-model"));
    assert_eq!(t.fields.status, ThreatStatus::Open);
    assert_eq!(t.display_id(), "THR-1");

    let gone = s.create_tm_component(tm, &comp("Old", None)).await.unwrap();
    let p2 = one(&s, tm, "tm-threats").await;
    s.delete_tm_component(tm, gone).await.unwrap();
    let e = s
        .apply_tm_threat(tm, p2, &json!({}), &threat_fields(gone), "m")
        .await
        .unwrap_err();
    assert!(matches!(e, StorageError::Invalid(_)));
    assert_eq!(s.get_tm_proposal(tm, p2).await.unwrap().status, "pending");
    assert_eq!(s.load_threat_model(tm).await.unwrap().threats.len(), 1);
}

#[tokio::test]
async fn a_rating_proposal_changes_one_rating_and_nothing_else() {
    let (_c, s) = store().await;
    let tm = model(&s).await;
    let api = s.create_tm_component(tm, &comp("API", None)).await.unwrap();
    let mut f = threat_fields(api);
    f.inherent = Some(Rating {
        likelihood: 4,
        impact: 5,
    });
    f.treatment = RiskTreatment::Mitigate;
    f.risk_owner = Some("team".into());
    let t = s.create_tm_threat(tm, &f, None).await.unwrap();
    let p = one(&s, tm, "tm-ratings").await;
    s.apply_tm_rating(
        tm,
        p,
        &json!({}),
        t,
        "current",
        Rating {
            likelihood: 3,
            impact: 4,
        },
    )
    .await
    .unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let got = &agg.threats[0];
    assert_eq!(
        got.fields.current,
        Some(Rating {
            likelihood: 3,
            impact: 4
        })
    );
    assert_eq!(got.fields.inherent, f.inherent);
    assert_eq!(got.fields.residual, None);
    assert_eq!(got.fields.treatment, RiskTreatment::Mitigate);
    assert_eq!(got.fields.risk_owner.as_deref(), Some("team"));
    assert_eq!(got.fields.status, ThreatStatus::Open);
    assert_eq!(
        got.origin,
        ThreatOrigin::Manual,
        "the threat itself is not AI-created"
    );

    let p2 = one(&s, tm, "tm-ratings").await;
    assert!(s
        .apply_tm_rating(
            tm,
            p2,
            &json!({}),
            Uuid::new_v4(),
            "current",
            Rating {
                likelihood: 1,
                impact: 1
            }
        )
        .await
        .is_err());
    assert!(s
        .apply_tm_rating(
            tm,
            p2,
            &json!({}),
            t,
            "bogus",
            Rating {
                likelihood: 1,
                impact: 1
            }
        )
        .await
        .is_err());
    assert_eq!(s.get_tm_proposal(tm, p2).await.unwrap().status, "pending");
}

#[tokio::test]
async fn control_proposals_create_a_planned_control_or_link_an_existing_one() {
    let (_c, s) = store().await;
    let tm = model(&s).await;
    let api = s.create_tm_component(tm, &comp("API", None)).await.unwrap();
    let t1 = s
        .create_tm_threat(tm, &threat_fields(api), None)
        .await
        .unwrap();
    let t2 = s
        .create_tm_threat(tm, &threat_fields(api), None)
        .await
        .unwrap();

    let p = one(&s, tm, "tm-controls").await;
    s.apply_tm_control(
        tm,
        p,
        &json!({}),
        ControlApply::New(control_fields(vec![t1])),
        "fake-model",
    )
    .await
    .unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let c = &agg.controls[0];
    assert_eq!(c.fields.status, ControlStatus::Planned);
    assert_eq!(c.provenance.origin, AiOrigin::Ai);
    assert_eq!(c.fields.threat_ids, vec![t1]);

    let p2 = one(&s, tm, "tm-controls").await;
    s.apply_tm_control(
        tm,
        p2,
        &json!({}),
        ControlApply::Link {
            control_id: c.audit.id,
            threat_ids: vec![t1, t2],
        },
        "fake-model",
    )
    .await
    .unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert_eq!(agg.controls.len(), 1, "linked, not duplicated");
    let mut linked = agg.controls[0].fields.threat_ids.clone();
    linked.sort();
    let mut want = vec![t1, t2];
    want.sort();
    assert_eq!(linked, want);
    assert_eq!(
        agg.controls[0].provenance.origin,
        AiOrigin::Ai,
        "link keeps the control as it was"
    );

    let p3 = one(&s, tm, "tm-controls").await;
    assert!(s
        .apply_tm_control(
            tm,
            p3,
            &json!({}),
            ControlApply::Link {
                control_id: Uuid::new_v4(),
                threat_ids: vec![t1]
            },
            "m"
        )
        .await
        .is_err());
}

#[tokio::test]
async fn a_security_test_proposal_becomes_a_draft_ai_test() {
    let (_c, s) = store().await;
    let tm = model(&s).await;
    let api = s.create_tm_component(tm, &comp("API", None)).await.unwrap();
    let t = s
        .create_tm_threat(tm, &threat_fields(api), None)
        .await
        .unwrap();
    let p = one(&s, tm, "tm-security-tests").await;
    let f = SecurityTestFields {
        title: "IDOR".into(),
        objective: None,
        preconditions: vec![],
        steps: vec![SecurityTestStep {
            action: "call with another id".into(),
            expected: "403".into(),
        }],
        test_type: SecurityTestType::NegativeAuthorization,
        status: SecurityTestStatus::Draft,
        threat_ids: vec![t],
        control_ids: vec![],
    };
    s.apply_tm_test(tm, p, &json!({}), &f, "fake-model")
        .await
        .unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let test = &agg.tests[0];
    assert_eq!(test.origin, SecurityTestOrigin::Ai);
    assert_eq!(test.fields.status, SecurityTestStatus::Draft);
    assert_eq!(test.source_model.as_deref(), Some("fake-model"));
    assert!(test.source.is_none(), "not a template test");
}
