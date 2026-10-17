//! Migration 0010 (AI assistance) on a database with a populated threat model. Requires Docker.

use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

#[tokio::test]
async fn existing_rows_stay_manual_and_ai_is_off() {
    let db = Postgres::default()
        .with_tag("17-alpine")
        .start()
        .await
        .unwrap();
    let url = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        db.get_host().await.unwrap(),
        db.get_host_port_ipv4(5432).await.unwrap()
    );
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    for sql in [
        include_str!("../migrations/0001_init.sql"),
        include_str!("../migrations/0002_tenancy.sql"),
        include_str!("../migrations/0003_ai_secret.sql"),
        include_str!("../migrations/0004_server_secrets.sql"),
        include_str!("../migrations/0005_traceability_links.sql"),
        include_str!("../migrations/0006_test_results.sql"),
        include_str!("../migrations/0007_model_layout.sql"),
        include_str!("../migrations/0008_threat_model.sql"),
        include_str!("../migrations/0009_tra_assurance.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let (p, tm, c, t, s) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    sqlx::raw_sql(&format!(
        "INSERT INTO projects (id, name, tenant_id) VALUES ('{p}', 'p', 't');
         INSERT INTO threat_models (id, project_id, tenant_id, name, thresholds)
             VALUES ('{tm}', '{p}', 't', 'TM', '[]');
         INSERT INTO tm_components (id, threat_model_id, name, component_type)
             VALUES ('{c}', '{tm}', 'API', 'api');
         INSERT INTO tm_threats (id, threat_model_id, number, title, category, status, origin, treatment)
             VALUES ('{t}', '{tm}', 1, 'Old', 'spoofing', 'open', 'rule', 'undecided');
         INSERT INTO security_tests (id, threat_model_id, number, title, test_type, status, origin)
             VALUES ('{s}', '{tm}', 1, 'T', 'other', 'draft', 'generated');"
    ))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(include_str!("../migrations/0010_tm_ai.sql"))
        .execute(&pool)
        .await
        .unwrap();

    let ai: bool = sqlx::query_scalar("SELECT ai_enabled FROM threat_models WHERE id = $1")
        .bind(tm)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!ai);
    let (origin, src): (String, Option<String>) =
        sqlx::query_as("SELECT origin, ai_source FROM tm_components WHERE id = $1")
            .bind(c)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((origin.as_str(), src), ("manual", None));
    // Old origins survive and the new value is accepted.
    let kept: String = sqlx::query_scalar("SELECT origin FROM tm_threats WHERE id = $1")
        .bind(t)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept, "rule");
    sqlx::query("UPDATE tm_threats SET origin = 'ai' WHERE id = $1")
        .bind(t)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE security_tests SET origin = 'ai' WHERE id = $1")
        .bind(s)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE tm_threats SET origin = 'robot' WHERE id = $1")
            .bind(t)
            .execute(&pool)
            .await
            .is_err()
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM tm_proposals")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}
