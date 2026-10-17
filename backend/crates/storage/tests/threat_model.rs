//! Threat models against a real PostgreSQL (FR-060..FR-076). Requires Docker.

use std::collections::HashSet;
use std::time::Instant;

use chrono::Utc;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tm_domain::threat::rules::suggest;
use tm_domain::threat::templates::generate;
use tm_domain::threat::*;
use tm_storage::{FeatureFields, NamedFields, StorageError, Store, TenantScope};
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
        treatment: RiskTreatment::Undecided,
        risk_owner: None,
        due_date: None,
        acceptance_rationale: None,
        accepted_by: None,
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
    let t1 = s.create_tm_threat(tm, &threat(target)).await.unwrap();
    let t2 = s.create_tm_threat(tm, &threat(target)).await.unwrap();
    s.delete_tm_threat(tm, t2).await.unwrap();
    let t3 = s.create_tm_threat(tm, &threat(target)).await.unwrap();
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
