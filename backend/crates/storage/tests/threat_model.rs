//! Threat models against a real PostgreSQL (FR-060..FR-076). Requires Docker.

use std::collections::HashSet;
use std::time::Instant;

use chrono::Utc;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tm_domain::threat::assurance::EvidenceFields;
use tm_domain::threat::rules::suggest;
use tm_domain::threat::templates::generate;
use tm_domain::threat::*;
use tm_storage::{
    EvidenceOwner, FeatureFields, NamedFields, NewBaseline, StorageError, Store, TenantScope,
    MAX_BASELINES,
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
    let s = Store::connect(&url, 4).await.unwrap();
    (c, s)
}

async fn project(s: &Store, tenant: &str) -> Uuid {
    s.create_project(
        tenant,
        NamedFields {
            name: "P".into(),
            description: None,
        },
    )
    .await
    .unwrap()
    .audit
    .id
}

async fn model(s: &Store, project: Uuid) -> Uuid {
    s.create_threat_model(
        project,
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

fn comp(name: &str, t: ArchComponentType, zone: Option<Uuid>) -> ArchComponentFields {
    ArchComponentFields {
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
    }
}

fn asset(name: &str) -> AssetFields {
    AssetFields {
        name: name.into(),
        description: None,
        category: AssetCategory::BusinessData,
        owner: None,
        confidentiality: Some(CiaLevel::High),
        integrity: None,
        availability: None,
        personal_data: true,
        classification: None,
        data_location: None,
        retention: None,
        encryption_at_rest: TriState::Unknown,
    }
}

fn flow(from: Uuid, to: Uuid) -> FlowFields {
    FlowFields {
        source_id: from,
        target_id: to,
        relationship: FlowRelationship::Calls,
        label: None,
        protocol: Some("HTTPS".into()),
        bidirectional: false,
        authentication: "unknown".into(),
        encrypted: TriState::Unknown,
        asset_ids: vec![],
        data_classification: None,
        authorization_model: None,
        entry_point: false,
    }
}

fn threat(target: ThreatTarget) -> ThreatFields {
    ThreatFields {
        title: "Unauthorized access".into(),
        category: StrideCategory::InformationDisclosure,
        scenario: None,
        attacker_objective: None,
        preconditions: None,
        consequences: None,
        targets: vec![target],
        asset_ids: vec![],
        control_ids: vec![],
        status: ThreatStatus::Open,
        dismiss_reason: None,
        inherent: Some(Rating {
            likelihood: 4,
            impact: 5,
        }),
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

fn control(name: &str) -> ControlFields {
    ControlFields {
        name: name.into(),
        description: None,
        control_type: ControlType::Preventive,
        category: ControlCategory::IdentityAccess,
        status: ControlStatus::Planned,
        owner: None,
        acceptance_criteria: None,
        review_by: None,
        component_ids: vec![],
        threat_ids: vec![],
    }
}

#[tokio::test]
async fn threat_model_lifecycle_tenancy_and_layout() {
    let (_c, s) = store().await;
    let p = project(&s, "t1").await;
    let tm = model(&s, p).await;

    // One per project.
    let again = s
        .create_threat_model(
            p,
            ThreatModelFields {
                name: "x".into(),
                ..Default::default()
            },
            Thresholds::default(),
        )
        .await;
    assert!(matches!(again, Err(StorageError::Conflict(_))));
    assert_eq!(s.threat_model_of_project(p).await.unwrap().audit.id, tm);

    // Tenant lookup.
    assert_eq!(
        s.tenant_of(TenantScope::ThreatModel, tm).await.unwrap(),
        "t1"
    );
    assert!(matches!(
        s.tenant_of(TenantScope::ThreatModel, Uuid::new_v4()).await,
        Err(StorageError::NotFound(_))
    ));

    // Scope fields with optimistic concurrency.
    let fields = ThreatModelFields {
        name: "TM".into(),
        assumptions: Some("TLS everywhere".into()),
        ..Default::default()
    };
    let saved = s
        .update_threat_model(tm, 1, fields.clone(), Thresholds::default())
        .await
        .unwrap();
    assert_eq!(saved.fields.assumptions.as_deref(), Some("TLS everywhere"));
    assert!(matches!(
        s.update_threat_model(tm, 1, fields, Thresholds::default())
            .await,
        Err(StorageError::VersionMismatch)
    ));

    // Layout has its own version and leaves the model version alone.
    let v = s
        .save_threat_model_layout(tm, 0, Some(serde_json::json!({"v": 1})))
        .await
        .unwrap();
    assert_eq!(v, 1);
    assert!(matches!(
        s.save_threat_model_layout(tm, 0, None).await,
        Err(StorageError::VersionMismatch)
    ));
    let m = s.get_threat_model(tm).await.unwrap();
    assert_eq!((m.audit.version, m.layout_version), (2, 1));

    // Deleting the project deletes the threat model.
    s.delete_project(p).await.unwrap();
    assert!(matches!(
        s.get_threat_model(tm).await,
        Err(StorageError::NotFound(_))
    ));
}

#[tokio::test]
async fn architecture_links_and_cascades() {
    let (_c, s) = store().await;
    let p = project(&s, "t").await;
    let tm = model(&s, p).await;
    let z0 = s.create_tm_zone(tm, &zone("Internet", 0)).await.unwrap();
    let z3 = s.create_tm_zone(tm, &zone("Internal", 3)).await.unwrap();
    assert!(matches!(
        s.create_tm_zone(tm, &zone("Internet", 1)).await,
        Err(StorageError::Conflict(_))
    ));

    let rec = s
        .create_tm_asset(tm, &asset("Customer records"))
        .await
        .unwrap();
    let mut api = comp("Customer API", ArchComponentType::Api, Some(z3));
    api.asset_ids = vec![rec];
    let api_id = s.create_tm_component(tm, &api).await.unwrap();
    let web = s
        .create_tm_component(
            tm,
            &comp("Web app", ArchComponentType::WebFrontend, Some(z0)),
        )
        .await
        .unwrap();
    let mut rep = comp("Reporting", ArchComponentType::Service, Some(z3));
    rep.asset_ids = vec![rec];
    s.create_tm_component(tm, &rep).await.unwrap();
    let dup = s
        .create_tm_component(tm, &comp("Web app", ArchComponentType::Api, None))
        .await;
    match dup {
        Err(StorageError::Conflict(m)) => assert!(m.contains("Web app")),
        other => panic!("expected conflict, got {other:?}"),
    }

    // Two flows between the same pair.
    let f1 = s.create_tm_flow(tm, &flow(web, api_id)).await.unwrap();
    let mut amqp = flow(web, api_id);
    amqp.relationship = FlowRelationship::PublishesSubscribes;
    amqp.asset_ids = vec![rec];
    s.create_tm_flow(tm, &amqp).await.unwrap();

    let agg = s.load_threat_model(tm).await.unwrap();
    assert_eq!(agg.arch.flows.len(), 2);
    assert_eq!(agg.arch.components_with_asset(rec).len(), 2);
    assert_eq!(agg.arch.flows_with_asset(rec).len(), 1);
    assert!(tm_domain::threat::arch::crossing(&agg.arch, agg.arch.flow(f1).unwrap()).crosses);

    // Ids of another threat model are rejected.
    let p2 = project(&s, "t").await;
    let tm2 = model(&s, p2).await;
    let other = s
        .create_tm_component(tm2, &comp("X", ArchComponentType::Service, None))
        .await
        .unwrap();
    assert!(matches!(
        s.create_tm_flow(tm, &flow(web, other)).await,
        Err(StorageError::Invalid(_))
    ));
    assert!(matches!(
        s.delete_tm_component(tm2, web).await,
        Err(StorageError::NotFound(_))
    ));

    // Link to a Component of the same project only.
    let own = s
        .create_component(
            p,
            NamedFields {
                name: "Payments".into(),
                description: None,
            },
        )
        .await
        .unwrap();
    let foreign = s
        .create_component(
            p2,
            NamedFields {
                name: "Other".into(),
                description: None,
            },
        )
        .await
        .unwrap();
    let feature = s
        .create_feature(
            own.audit.id,
            FeatureFields {
                name: "Pay".into(),
                description: None,
                scenario_description: None,
                tags: None,
            },
        )
        .await
        .unwrap();
    let mut linked = comp("Payments svc", ArchComponentType::Service, None);
    linked.linked_component_id = Some(own.audit.id);
    linked.linked_feature_id = Some(feature.audit.id);
    let linked_id = s.create_tm_component(tm, &linked).await.unwrap();
    linked.linked_component_id = Some(foreign.audit.id);
    assert!(matches!(
        s.update_tm_component(tm, linked_id, 1, &linked).await,
        Err(StorageError::Invalid(_))
    ));
    // Deleting the Component clears the link.
    s.delete_component(own.audit.id, true).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let c = agg.arch.component(linked_id).unwrap();
    assert_eq!(
        (c.fields.linked_component_id, c.fields.linked_feature_id),
        (None, None)
    );

    // Deleting a zone leaves components without one.
    s.delete_tm_zone(tm, z3).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert!(agg.arch.component(api_id).unwrap().fields.zone_id.is_none());

    // Deleting a component deletes its flows; deleting an asset its links.
    s.delete_tm_component(tm, web).await.unwrap();
    s.delete_tm_asset(tm, rec).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert!(agg.arch.flows.is_empty());
    assert!(agg
        .arch
        .component(api_id)
        .unwrap()
        .fields
        .asset_ids
        .is_empty());

    // Stale version.
    let api = comp("Customer API", ArchComponentType::Api, None);
    assert!(matches!(
        s.update_tm_component(tm, api_id, 99, &api).await,
        Err(StorageError::VersionMismatch)
    ));
}

#[tokio::test]
async fn threats_controls_ids_and_rule_keys() {
    let (_c, s) = store().await;
    let p = project(&s, "t").await;
    let tm = model(&s, p).await;
    let z0 = s.create_tm_zone(tm, &zone("Internet", 0)).await.unwrap();
    let z3 = s.create_tm_zone(tm, &zone("Internal", 3)).await.unwrap();
    let web = s
        .create_tm_component(
            tm,
            &comp("Web app", ArchComponentType::WebFrontend, Some(z0)),
        )
        .await
        .unwrap();
    let api = s
        .create_tm_component(tm, &comp("Customer API", ArchComponentType::Api, Some(z3)))
        .await
        .unwrap();
    s.create_tm_flow(tm, &flow(web, api)).await.unwrap();

    let target = ThreatTarget {
        kind: TargetKind::Component,
        id: api,
    };
    let t1 = s.create_tm_threat(tm, &threat(target), None).await.unwrap();
    let t2 = s.create_tm_threat(tm, &threat(target), None).await.unwrap();
    s.delete_tm_threat(tm, t2).await.unwrap();
    let t3 = s.create_tm_threat(tm, &threat(target), None).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let numbers: Vec<i32> = agg.threats.iter().map(|t| t.number).collect();
    assert_eq!(numbers, [1, 3], "deleted numbers are not reused");
    assert_eq!(
        agg.threats[0].fields.inherent,
        Some(Rating {
            likelihood: 4,
            impact: 5
        })
    );

    // Controls link both ways.
    let mut c = control("Object-level authorization");
    c.threat_ids = vec![t1];
    c.component_ids = vec![api];
    let c1 = s.create_tm_control(tm, &c).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert_eq!(agg.controls[0].number, 1);
    assert_eq!(
        agg.threats
            .iter()
            .find(|t| t.audit.id == t1)
            .unwrap()
            .fields
            .control_ids,
        [c1]
    );
    assert!(agg.controls[0].verified_at.is_none());
    c.status = ControlStatus::Verified;
    s.update_tm_control(tm, c1, 1, &c).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert!(agg.controls[0].verified_at.is_some());
    // Deleting a control keeps the threat and its ratings.
    s.delete_tm_control(tm, c1).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let kept = agg.threats.iter().find(|t| t.audit.id == t1).unwrap();
    assert!(kept.fields.control_ids.is_empty());
    assert_eq!(kept.fields.inherent.unwrap().score(), 20);

    // Rule candidates; dismissed and deleted keys are remembered.
    let first = suggest(&agg.arch, &agg.rule_keys);
    assert_eq!(first.candidates.len(), 1);
    let ids = s.create_tm_candidates(tm, &first.candidates).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let cand = agg.threats.iter().find(|t| t.audit.id == ids[0]).unwrap();
    assert_eq!(
        (cand.fields.status, cand.origin),
        (ThreatStatus::Candidate, ThreatOrigin::Rule)
    );
    assert_eq!(
        cand.explanation.as_ref().unwrap().rule_id,
        "SPOOF-BOUNDARY-AUTH"
    );
    s.set_tm_threat_statuses(tm, &[(ids[0], ThreatStatus::Dismissed, Some("n/a".into()))])
        .await
        .unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert!(suggest(&agg.arch, &agg.rule_keys).candidates.is_empty());
    s.delete_tm_threat(tm, ids[0]).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert!(suggest(&agg.arch, &agg.rule_keys).candidates.is_empty());
    let _ = t3;
}

#[tokio::test]
async fn security_tests_generation_results_and_deleted_threat() {
    let (_c, s) = store().await;
    let p = project(&s, "t").await;
    let tm = model(&s, p).await;
    let api = s
        .create_tm_component(tm, &comp("Customer API", ArchComponentType::Api, None))
        .await
        .unwrap();
    let t1 = s
        .create_tm_threat(
            tm,
            &threat(ThreatTarget {
                kind: TargetKind::Component,
                id: api,
            }),
            None,
        )
        .await
        .unwrap();

    let agg = s.load_threat_model(tm).await.unwrap();
    let threats: Vec<&Threat> = agg.threats.iter().collect();
    let keys: HashSet<String> = HashSet::new();
    let g = generate(&threats, &agg.arch, &Default::default(), &keys);
    assert_eq!(g.tests.len(), 1);
    let ids = s.create_generated_tests(tm, &g.tests).await.unwrap();
    assert_eq!(ids.len(), 1);
    // The same key again is skipped by the database too.
    assert!(s
        .create_generated_tests(tm, &g.tests)
        .await
        .unwrap()
        .is_empty());

    let manual = SecurityTestFields {
        title: "Access another customer's record".into(),
        objective: None,
        preconditions: vec!["Two customers exist".into()],
        steps: vec![SecurityTestStep {
            action: "GET /customers/2 as 1".into(),
            expected: "403".into(),
        }],
        test_type: SecurityTestType::NegativeAuthorization,
        status: SecurityTestStatus::Draft,
        threat_ids: vec![],
        control_ids: vec![],
    };
    assert!(matches!(
        s.create_security_test(tm, &manual).await,
        Err(StorageError::Invalid(_))
    ));
    let manual_id = s
        .create_security_test(
            tm,
            &SecurityTestFields {
                threat_ids: vec![t1],
                ..manual.clone()
            },
        )
        .await
        .unwrap();

    s.set_security_test_statuses(tm, &[ids[0], manual_id], SecurityTestStatus::Ready)
        .await
        .unwrap();
    let older = Utc::now() - chrono::Duration::hours(1);
    s.record_security_test_result(tm, manual_id, SecurityTestOutcome::Passed, None, older)
        .await
        .unwrap();
    s.record_security_test_result(
        tm,
        manual_id,
        SecurityTestOutcome::Failed,
        Some("IDOR".into()),
        Utc::now(),
    )
    .await
    .unwrap();
    assert_eq!(
        s.security_test_results(tm, manual_id).await.unwrap().len(),
        2
    );

    let agg = s.load_threat_model(tm).await.unwrap();
    let gen = agg.tests.iter().find(|t| t.audit.id == ids[0]).unwrap();
    assert_eq!((gen.number, gen.origin), (1, SecurityTestOrigin::Generated));
    assert_eq!(gen.source.as_ref().unwrap().template_id, "ACCESS-COMPONENT");
    let m = agg.tests.iter().find(|t| t.audit.id == manual_id).unwrap();
    assert_eq!(m.fields.status, SecurityTestStatus::Ready);
    assert_eq!(
        m.last_result.as_ref().unwrap().outcome,
        SecurityTestOutcome::Failed
    );
    assert_eq!(m.fields.steps[0].expected, "403");

    // Deleting the threat leaves the tests without a threat.
    s.delete_tm_threat(tm, t1).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert_eq!(agg.tests.len(), 2);
    assert!(agg.tests.iter().all(|t| t.fields.threat_ids.is_empty()));
    assert_eq!(agg.tests[0].source.as_ref().unwrap().threat_id, t1);
}

#[tokio::test]
async fn aggregate_of_300_entities_loads_quickly() {
    let (_c, s) = store().await;
    let p = project(&s, "t").await;
    let tm = model(&s, p).await;
    let z = s.create_tm_zone(tm, &zone("Internal", 3)).await.unwrap();
    let mut comps = Vec::new();
    for i in 0..100 {
        comps.push(
            s.create_tm_component(
                tm,
                &comp(&format!("C{i}"), ArchComponentType::Service, Some(z)),
            )
            .await
            .unwrap(),
        );
    }
    for w in comps.windows(2) {
        s.create_tm_flow(tm, &flow(w[0], w[1])).await.unwrap();
    }
    for c in comps.iter().take(100) {
        s.create_tm_threat(
            tm,
            &threat(ThreatTarget {
                kind: TargetKind::Component,
                id: *c,
            }),
            None,
        )
        .await
        .unwrap();
    }
    s.load_threat_model(tm).await.unwrap();
    let start = Instant::now();
    let agg = s.load_threat_model(tm).await.unwrap();
    let elapsed = start.elapsed();
    assert_eq!(
        agg.arch.components.len() + agg.arch.flows.len() + agg.threats.len(),
        299
    );
    assert!(elapsed.as_millis() < 200, "aggregate load took {elapsed:?}");
}

fn evidence(title: &str) -> EvidenceFields {
    EvidenceFields {
        kind: EvidenceKind::TestReport,
        title: title.into(),
        link: Some("https://example.org/report".into()),
        note: None,
        evidence_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        reviewer: Some("R. Viewer".into()),
    }
}

#[tokio::test]
async fn assurance_fields_round_trip() {
    let (_c, s) = store().await;
    let p = project(&s, "t").await;
    let tm = model(&s, p).await;
    let api = s
        .create_tm_component(tm, &comp("API", ArchComponentType::Api, None))
        .await
        .unwrap();
    let target = ThreatTarget {
        kind: TargetKind::Component,
        id: api,
    };
    let mut f = threat(target);
    f.current = Some(Rating {
        likelihood: 3,
        impact: 3,
    });
    f.residual = Some(Rating {
        likelihood: 2,
        impact: 2,
    });
    f.threat_source = Some("malicious tenant admin".into());
    f.detection = Some("audit log alert".into());
    f.existing_mitigations = Some("WAF".into());
    f.confidence = Some(Confidence::Medium);
    f.treatment = RiskTreatment::Accept;
    f.status = ThreatStatus::Accepted;
    f.acceptance_rationale = Some("low".into());
    f.accepted_by = Some("J. Doe".into());
    let expires = chrono::NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
    let on = chrono::NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();
    f.acceptance_expires = Some(expires);
    let t = s.create_tm_threat(tm, &f, Some(on)).await.unwrap();

    let agg = s.load_threat_model(tm).await.unwrap();
    let got = agg.threats.iter().find(|x| x.audit.id == t).unwrap();
    assert_eq!(got.fields.residual, f.residual);
    assert_eq!(got.fields.confidence, Some(Confidence::Medium));
    assert_eq!(
        got.fields.threat_source.as_deref(),
        Some("malicious tenant admin")
    );
    assert_eq!(got.fields.acceptance_expires, Some(expires));
    assert_eq!(got.accepted_on, Some(on));

    // Asset and flow detail fields.
    let mut a = asset("Records");
    a.classification = Some(Classification::Restricted);
    a.data_location = Some("eu-west-1".into());
    a.encryption_at_rest = TriState::Yes;
    let aid = s.create_tm_asset(tm, &a).await.unwrap();
    let other = s
        .create_tm_component(tm, &comp("DB", ArchComponentType::RelationalDatabase, None))
        .await
        .unwrap();
    let mut fl = flow(api, other);
    fl.entry_point = true;
    fl.data_classification = Some(Classification::Confidential);
    fl.authorization_model = Some("RBAC".into());
    s.create_tm_flow(tm, &fl).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    let ga = agg.arch.asset(aid).unwrap();
    assert_eq!(ga.fields.classification, Some(Classification::Restricted));
    assert_eq!(ga.fields.encryption_at_rest, TriState::Yes);
    assert_eq!(ga.fields.data_location.as_deref(), Some("eu-west-1"));
    let gf = &agg.arch.flows[0];
    assert!(gf.fields.entry_point);
    assert_eq!(gf.fields.authorization_model.as_deref(), Some("RBAC"));

    // Review-by date on controls.
    let mut c = control("MFA");
    c.review_by = Some(expires);
    let cid = s.create_tm_control(tm, &c).await.unwrap();
    let agg = s.load_threat_model(tm).await.unwrap();
    assert_eq!(
        agg.controls
            .iter()
            .find(|x| x.audit.id == cid)
            .unwrap()
            .fields
            .review_by,
        Some(expires)
    );
}

#[tokio::test]
async fn evidence_belongs_to_one_owner_and_last_removal_downgrades_the_control() {
    let (_c, s) = store().await;
    let p = project(&s, "t").await;
    let tm = model(&s, p).await;
    let other_p = project(&s, "t").await;
    let other_tm = model(&s, other_p).await;
    let cid = s.create_tm_control(tm, &control("MFA")).await.unwrap();

    // Another threat model's control is not found.
    assert!(matches!(
        s.add_evidence(other_tm, EvidenceOwner::Control(cid), &evidence("x"))
            .await,
        Err(StorageError::NotFound(_))
    ));

    let e1 = s
        .add_evidence(tm, EvidenceOwner::Control(cid), &evidence("Pen test"))
        .await
        .unwrap();
    let e2 = s
        .add_evidence(tm, EvidenceOwner::Control(cid), &evidence("Config review"))
        .await
        .unwrap();
    assert_eq!(s.control_evidence_count(tm, cid).await.unwrap(), 2);
    assert_eq!(
        s.list_evidence(tm, EvidenceOwner::Control(cid))
            .await
            .unwrap()
            .len(),
        2
    );

    // Verify the control, then remove evidence.
    let mut f = control("MFA");
    f.status = ControlStatus::Verified;
    s.update_tm_control(tm, cid, 1, &f).await.unwrap();
    assert_eq!(s.delete_evidence(tm, e1.audit.id).await.unwrap(), None);
    assert_eq!(s.delete_evidence(tm, e2.audit.id).await.unwrap(), Some(cid));
    let agg = s.load_threat_model(tm).await.unwrap();
    let c = agg.controls.iter().find(|x| x.audit.id == cid).unwrap();
    assert_eq!(c.fields.status, ControlStatus::Implemented);
    assert!(c.verified_at.is_none());
    assert!(matches!(
        s.delete_evidence(tm, e2.audit.id).await,
        Err(StorageError::NotFound(_))
    ));
}

#[tokio::test]
async fn evidence_of_a_result_goes_with_the_test_and_the_control() {
    let (_c, s) = store().await;
    let p = project(&s, "t").await;
    let tm = model(&s, p).await;
    let api = s
        .create_tm_component(tm, &comp("API", ArchComponentType::Api, None))
        .await
        .unwrap();
    let t = s
        .create_tm_threat(
            tm,
            &threat(ThreatTarget {
                kind: TargetKind::Component,
                id: api,
            }),
            None,
        )
        .await
        .unwrap();
    let test = s
        .create_security_test(
            tm,
            &SecurityTestFields {
                title: "x".into(),
                objective: None,
                preconditions: vec![],
                steps: vec![],
                test_type: SecurityTestType::Other,
                status: SecurityTestStatus::Ready,
                threat_ids: vec![t],
                control_ids: vec![],
            },
        )
        .await
        .unwrap();
    let r = s
        .record_security_test_result(tm, test, SecurityTestOutcome::Passed, None, Utc::now())
        .await
        .unwrap();
    let owner = EvidenceOwner::TestResult {
        test_id: test,
        result_id: r.id,
    };
    s.add_evidence(tm, owner, &evidence("Run log"))
        .await
        .unwrap();
    assert_eq!(s.list_evidence(tm, owner).await.unwrap().len(), 1);
    assert_eq!(s.load_threat_model(tm).await.unwrap().evidence.len(), 1);
    // A result of another test is not found.
    let bad = EvidenceOwner::TestResult {
        test_id: Uuid::new_v4(),
        result_id: r.id,
    };
    assert!(s.add_evidence(tm, bad, &evidence("x")).await.is_err());
    s.delete_security_test(tm, test).await.unwrap();
    assert!(s.load_threat_model(tm).await.unwrap().evidence.is_empty());
}

fn new_baseline(name: &str, snapshot: serde_json::Value) -> NewBaseline {
    NewBaseline {
        name: name.into(),
        note: None,
        created_by: Some("tester".into()),
        schema_version: 1,
        threat_count: 2,
        control_count: 1,
        snapshot,
    }
}

#[tokio::test]
async fn baselines_are_unique_capped_immutable_and_cascade() {
    let (_c, s) = store().await;
    let p = project(&s, "t").await;
    let tm = model(&s, p).await;
    let snap = serde_json::json!({"threats": [{"title": "a"}]});
    let b = s
        .create_baseline(tm, new_baseline("Release 2.3", snap.clone()))
        .await
        .unwrap();
    assert!(matches!(
        s.create_baseline(tm, new_baseline("Release 2.3", snap.clone()))
            .await,
        Err(StorageError::Conflict(_))
    ));
    let (meta, got) = s.get_baseline(tm, b.id).await.unwrap();
    assert_eq!(meta.threat_count, 2);
    assert_eq!(got, snap);
    assert_eq!(s.list_baselines(tm).await.unwrap().len(), 1);

    // Live edits do not change the stored snapshot.
    s.update_threat_model(
        tm,
        1,
        ThreatModelFields {
            name: "renamed".into(),
            ..Default::default()
        },
        Thresholds::default(),
    )
    .await
    .unwrap();
    assert_eq!(s.get_baseline(tm, b.id).await.unwrap().1, snap);

    // Another threat model cannot read or delete it.
    let p2 = project(&s, "t").await;
    let tm2 = model(&s, p2).await;
    assert!(matches!(
        s.get_baseline(tm2, b.id).await,
        Err(StorageError::NotFound(_))
    ));
    assert!(matches!(
        s.delete_baseline(tm2, b.id).await,
        Err(StorageError::NotFound(_))
    ));

    // The cap.
    for i in 1..MAX_BASELINES {
        s.create_baseline(tm, new_baseline(&format!("b{i}"), snap.clone()))
            .await
            .unwrap();
    }
    assert!(matches!(
        s.create_baseline(tm, new_baseline("one too many", snap.clone()))
            .await,
        Err(StorageError::Invalid(_))
    ));
    s.delete_baseline(tm, b.id).await.unwrap();
    s.create_baseline(tm, new_baseline("fits again", snap))
        .await
        .unwrap();

    // Deleting the threat model deletes its baselines.
    s.delete_threat_model(tm).await.unwrap();
    assert!(s.list_baselines(tm).await.unwrap().is_empty());
}
