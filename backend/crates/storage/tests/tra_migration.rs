//! Migration 0009 (TRA assurance) on a database with a populated threat model. Requires Docker.

use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

#[tokio::test]
async fn existing_threats_and_controls_keep_their_data_and_gain_empty_assurance_fields() {
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
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let (p, tm, c, t, ctl) = (
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
         INSERT INTO tm_threats (id, threat_model_id, number, title, category, status, origin,
             treatment, acceptance_rationale, accepted_by)
             VALUES ('{t}', '{tm}', 1, 'Old threat', 'spoofing', 'accepted', 'manual',
             'accept', 'fine', 'J. Doe');
         INSERT INTO tm_controls (id, threat_model_id, number, name, control_type, category,
             status, verified_at)
             VALUES ('{ctl}', '{tm}', 1, 'MFA', 'preventive', 'identity-access', 'verified', now());"
    ))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(include_str!("../migrations/0009_tra_assurance.sql"))
        .execute(&pool)
        .await
        .unwrap();

    // A legacy acceptance has no expiry; a verified control has no evidence.
    let (title, expires, residual): (String, Option<chrono::NaiveDate>, Option<i16>) =
        sqlx::query_as(
            "SELECT title, acceptance_expires, residual_likelihood FROM tm_threats WHERE id = $1",
        )
        .bind(t)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        (title.as_str(), expires, residual),
        ("Old threat", None, None)
    );
    let (status, evidence): (String, i64) = sqlx::query_as(
        "SELECT status, (SELECT count(*) FROM tm_evidence WHERE control_id = $1) FROM tm_controls WHERE id = $1",
    )
    .bind(ctl)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((status.as_str(), evidence), ("verified", 0));
}
