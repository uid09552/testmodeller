//! Migration 0007 (model layout) on a database with models. Requires Docker.

use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

#[tokio::test]
async fn existing_models_get_no_layout_and_keep_their_data() {
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
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let (p, c, f, m) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    sqlx::raw_sql(&format!(
        "INSERT INTO projects (id, name, tenant_id) VALUES ('{p}', 'p', 't');
         INSERT INTO components (id, project_id, name) VALUES ('{c}', '{p}', 'c');
         INSERT INTO features (id, component_id, name) VALUES ('{f}', '{c}', 'f');
         INSERT INTO models (id, feature_id, name) VALUES ('{m}', '{f}', 'Login');"
    ))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(include_str!("../migrations/0007_model_layout.sql"))
        .execute(&pool)
        .await
        .unwrap();

    let (name, layout): (String, Option<serde_json::Value>) =
        sqlx::query_as("SELECT name, layout FROM models WHERE id = $1")
            .bind(m)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(name, "Login");
    assert_eq!(layout, None);
}
