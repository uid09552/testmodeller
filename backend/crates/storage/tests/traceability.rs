//! Links and traceability against a real PostgreSQL (FR-025..FR-028). Requires Docker.

use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tm_domain::links::backlog_key;
use tm_domain::{
    AssignmentTarget, ModelGraph, Origin, State, StateKind, TestCaseData, TestCaseStatus,
};
use tm_storage::{
    FeatureFields, ModelMeta, NamedFields, NewAssignment, Store, TraceFilter, TraceGap,
};
use uuid::Uuid;

async fn database() -> (ContainerAsync<Postgres>, String) {
    let container = Postgres::default()
        .with_tag("17-alpine")
        .start()
        .await
        .expect("start PostgreSQL; is Docker running?");
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    (
        container,
        format!("postgres://postgres:postgres@{host}:{port}/postgres"),
    )
}

fn data(name: &str, implementation: Option<&str>, backlog: Option<&str>) -> TestCaseData {
    TestCaseData {
        name: name.into(),
        description: None,
        preconditions: None,
        priority: None,
        status: TestCaseStatus::Draft,
        tags: vec![],
        steps: vec![],
        implementation_url: implementation.map(Into::into),
        backlog_url: backlog.map(Into::into),
    }
}

struct Fixture {
    store: Store,
    project: Uuid,
    component: Uuid,
    feature: Uuid,
    model: Uuid,
    state: Uuid,
}

async fn fixture(store: &Store, tenant: &str) -> Fixture {
    let project = store
        .create_project(
            tenant,
            NamedFields {
                name: "P".into(),
                description: None,
            },
        )
        .await
        .unwrap()
        .audit
        .id;
    let component = store
        .create_component(
            project,
            NamedFields {
                name: "C".into(),
                description: None,
            },
        )
        .await
        .unwrap()
        .audit
        .id;
    let feature = store
        .create_feature(
            component,
            FeatureFields {
                name: "F".into(),
                description: None,
                scenario_description: None,
                tags: None,
            },
        )
        .await
        .unwrap()
        .audit
        .id;
    let state = Uuid::new_v4();
    let graph = ModelGraph {
        variables: vec![],
        states: vec![State {
            id: state,
            name: "S".into(),
            description: None,
            kind: StateKind::Initial,
            position: None,
        }],
        transitions: vec![],
    };
    let model = store
        .create_model(feature, "M", ModelMeta::default(), &graph)
        .await
        .unwrap()
        .summary
        .audit
        .id;
    Fixture {
        store: store.clone(),
        project,
        component,
        feature,
        model,
        state,
    }
}

impl Fixture {
    async fn case(&self, d: TestCaseData, assigned: bool) -> Uuid {
        let assignments = if assigned {
            vec![NewAssignment {
                model_id: self.model,
                target: AssignmentTarget::State(self.state),
                step_order: None,
            }]
        } else {
            vec![]
        };
        self.store
            .create_test_case(self.feature, &d, Origin::Manual, &assignments)
            .await
            .unwrap()
            .audit
            .id
    }
}

/// FR-025: links are separate fields, round trip, and change on update.
#[tokio::test]
async fn links_round_trip_as_fields() {
    let (_db, url) = database().await;
    let store = Store::connect(&url, 2).await.unwrap();
    let f = fixture(&store, "t1").await;
    let id = f
        .case(
            data("a", Some("https://git/x/1"), Some("https://jira/TM-1")),
            false,
        )
        .await;
    let tc = store.get_test_case(id).await.unwrap();
    assert_eq!(
        tc.data.implementation_url.as_deref(),
        Some("https://git/x/1")
    );
    assert_eq!(tc.data.backlog_url.as_deref(), Some("https://jira/TM-1"));
    assert_eq!(tc.data.description, None);

    let updated = store
        .replace_test_case(id, tc.audit.version, &data("a", None, None), None)
        .await
        .unwrap();
    assert_eq!(updated.data.backlog_url, None);
    let trace = store
        .traceability(f.project, TraceFilter::default())
        .await
        .unwrap();
    assert!(trace.items.is_empty());
    assert_eq!(trace.untraced.len(), 1);
}

/// FR-026, FR-027: grouping, several test cases per item, gaps, filters.
#[tokio::test]
async fn groups_by_backlog_item_and_finds_gaps() {
    let (_db, url) = database().await;
    let store = Store::connect(&url, 2).await.unwrap();
    let f = fixture(&store, "t1").await;
    // Same item spelled three ways.
    let a = f
        .case(
            data(
                "a",
                Some("https://git/1"),
                Some("https://Jira.example/TM-1"),
            ),
            true,
        )
        .await;
    f.case(data("b", None, Some("https://jira.example/TM-1/")), true)
        .await;
    f.case(
        data(
            "c",
            Some("https://git/2"),
            Some("https://jira.example/TM-1#c"),
        ),
        true,
    )
    .await;
    // Item whose only test case covers no model element.
    f.case(
        data(
            "d",
            Some("https://git/3"),
            Some("https://jira.example/TM-2"),
        ),
        false,
    )
    .await;
    // Untraced.
    f.case(data("e", Some("https://git/4"), None), true).await;

    let all = store
        .traceability(f.project, TraceFilter::default())
        .await
        .unwrap();
    assert_eq!(all.items.len(), 2);
    let tm1 = &all.items[0];
    assert_eq!(tm1.backlog_url, "https://jira.example/TM-1");
    assert_eq!(tm1.test_cases.len(), 3);
    assert_eq!(tm1.test_cases[0].id, a);
    assert_eq!(tm1.test_cases[0].component_id, f.component);
    assert_eq!(tm1.test_cases[0].feature_id, f.feature);
    assert_eq!(tm1.test_cases[0].elements.len(), 1);
    assert_eq!(tm1.test_cases[0].elements[0].model_id, f.model);
    assert_eq!(
        tm1.test_cases[0].elements[0].target,
        AssignmentTarget::State(f.state)
    );
    assert_eq!(all.untraced.len(), 1);
    assert_eq!(all.summary.backlog_items, 2);
    assert_eq!(all.summary.test_cases, 5);
    assert_eq!(all.summary.untraced, 1);
    assert_eq!(all.summary.unimplemented, 1);
    assert_eq!(all.summary.no_elements, 1);

    let gap = |g| TraceFilter {
        gap: Some(g),
        ..Default::default()
    };
    let unimplemented = store
        .traceability(f.project, gap(TraceGap::Unimplemented))
        .await
        .unwrap();
    assert_eq!(unimplemented.items.len(), 1);
    assert_eq!(
        unimplemented.items[0].backlog_url,
        "https://jira.example/TM-1"
    );
    assert!(unimplemented.untraced.is_empty());

    let none = store
        .traceability(f.project, gap(TraceGap::NoElements))
        .await
        .unwrap();
    assert_eq!(none.items.len(), 1);
    assert_eq!(none.items[0].backlog_url, "https://jira.example/TM-2");

    let untraced = store
        .traceability(f.project, gap(TraceGap::Untraced))
        .await
        .unwrap();
    assert!(untraced.items.is_empty());
    assert_eq!(untraced.untraced.len(), 1);
    // The summary is not narrowed by the gap.
    assert_eq!(untraced.summary, all.summary);

    let other_component = store
        .traceability(
            f.project,
            TraceFilter {
                component_id: Some(Uuid::new_v4()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(other_component.items.is_empty() && other_component.untraced.is_empty());
}

/// Tenant and project scope: another project with the same backlog URL does not appear.
#[tokio::test]
async fn other_projects_and_tenants_are_excluded() {
    let (_db, url) = database().await;
    let store = Store::connect(&url, 2).await.unwrap();
    let mine = fixture(&store, "t1").await;
    let theirs = fixture(&store, "t2").await;
    mine.case(data("mine", None, Some("https://jira/TM-1")), true)
        .await;
    theirs
        .case(data("theirs", None, Some("https://jira/TM-1")), true)
        .await;
    let trace = store
        .traceability(mine.project, TraceFilter::default())
        .await
        .unwrap();
    assert_eq!(trace.items.len(), 1);
    assert_eq!(trace.items[0].test_cases.len(), 1);
    assert_eq!(trace.items[0].test_cases[0].name, "mine");
    // A component of the other tenant yields nothing, not their data.
    let cross = store
        .traceability(
            mine.project,
            TraceFilter {
                component_id: Some(theirs.component),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(cross.items.is_empty() && cross.untraced.is_empty());
}

/// FR-025: the migration moves legacy description lines into the fields, idempotently,
/// computes the same key as `backlog_key`, and leaves other text alone.
#[tokio::test]
async fn migration_moves_legacy_description_lines() {
    let (_db, url) = database().await;
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    for sql in [
        include_str!("../migrations/0001_init.sql"),
        include_str!("../migrations/0002_tenancy.sql"),
        include_str!("../migrations/0003_ai_secret.sql"),
        include_str!("../migrations/0004_server_secrets.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let project = Uuid::new_v4();
    let component = Uuid::new_v4();
    let feature = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name, tenant_id) VALUES ($1, 'p', 't')")
        .bind(project)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO components (id, project_id, name) VALUES ($1, $2, 'c')")
        .bind(component)
        .bind(project)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO features (id, component_id, name) VALUES ($1, $2, 'f')")
        .bind(feature)
        .bind(component)
        .execute(&pool)
        .await
        .unwrap();
    let legacy = [
        (
            "both",
            Some("Implementation: https://git/x/1\nBacklog: https://Jira.example/TM-1/#c"),
        ),
        (
            "with text",
            Some("Backlog: https://jira/TM-2?id=3\nkeep me\n"),
        ),
        (
            "crlf",
            Some("Implementation: https://git/y\r\nBacklog: https://jira/TM-3\r"),
        ),
        (
            "invalid",
            Some("Backlog: not a url\nImplementation: ftp://x"),
        ),
        ("plain", Some("just text")),
        ("none", None),
    ];
    let mut ids = Vec::new();
    for (name, description) in legacy {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO test_cases (id, feature_id, name, description, origin)
             VALUES ($1, $2, $3, $4, 'manual')",
        )
        .bind(id)
        .bind(feature)
        .bind(name)
        .bind(description)
        .execute(&pool)
        .await
        .unwrap();
        ids.push(id);
    }

    let up = include_str!("../migrations/0005_traceability_links.sql");
    sqlx::raw_sql(up).execute(&pool).await.unwrap();

    let read = |id: Uuid| {
        let pool = pool.clone();
        async move {
            let row = sqlx::query(
                "SELECT description, implementation_url, backlog_url, backlog_key
                 FROM test_cases WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
            (
                row.get::<Option<String>, _>(0),
                row.get::<Option<String>, _>(1),
                row.get::<Option<String>, _>(2),
                row.get::<Option<String>, _>(3),
            )
        }
    };
    let s = |v: &str| Some(v.to_owned());

    let both = read(ids[0]).await;
    assert_eq!(both.0, None);
    assert_eq!(both.1, s("https://git/x/1"));
    assert_eq!(both.2, s("https://Jira.example/TM-1/#c"));
    assert_eq!(both.3, s("https://jira.example/TM-1"));

    let text = read(ids[1]).await;
    assert_eq!(text.0, s("keep me"));
    assert_eq!(text.1, None);
    assert_eq!(text.2, s("https://jira/TM-2?id=3"));
    assert_eq!(text.3, s("https://jira/TM-2?id=3"));

    let crlf = read(ids[2]).await;
    assert_eq!(crlf.0, None);
    assert_eq!(
        (crlf.1, crlf.2),
        (s("https://git/y"), s("https://jira/TM-3"))
    );

    // Invalid values are not lost: the lines stay in the description.
    let invalid = read(ids[3]).await;
    assert_eq!(invalid.0, s("Backlog: not a url\nImplementation: ftp://x"));
    assert_eq!((invalid.1, invalid.2, invalid.3), (None, None, None));

    assert_eq!(read(ids[4]).await.0, s("just text"));
    assert_eq!(read(ids[5]).await, (None, None, None, None));

    // The SQL key agrees with the Rust one.
    for id in &ids {
        let (_, _, link, key) = read(*id).await;
        assert_eq!(key, link.as_deref().and_then(backlog_key));
    }

    // Idempotent: running the up script again changes nothing.
    let before = read(ids[0]).await;
    sqlx::raw_sql(up).execute(&pool).await.unwrap();
    assert_eq!(before, read(ids[0]).await);
    assert_eq!(read(ids[1]).await.0, s("keep me"));

    // The down script puts the lines back. sqlx keeps its bookkeeping in this
    // table; the raw runs above skipped it.
    sqlx::raw_sql("CREATE TABLE _sqlx_migrations (version bigint)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations-down/0005_traceability_links.down.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let restored: Option<String> =
        sqlx::query_scalar("SELECT description FROM test_cases WHERE id = $1")
            .bind(ids[1])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        restored.as_deref(),
        Some("Backlog: https://jira/TM-2?id=3\nkeep me")
    );
}

/// Opening an already migrated database with the store applies 0005 on a fresh database too.
#[tokio::test]
async fn fresh_database_has_the_columns() {
    let (_db, url) = database().await;
    Store::connect(&url, 1).await.unwrap();
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns WHERE table_name = 'test_cases'
           AND column_name IN ('implementation_url', 'backlog_url', 'backlog_key')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 3);
}
