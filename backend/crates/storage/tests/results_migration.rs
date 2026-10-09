//! Migration 0006 (test results) on an existing database. Requires Docker.

use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use tm_storage::Store;
use uuid::Uuid;

#[tokio::test]
async fn applies_to_a_database_with_data_and_enforces_one_result_per_run() {
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
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let (p, c, f, t) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    sqlx::raw_sql(&format!(
        "INSERT INTO projects (id, name, tenant_id) VALUES ('{p}', 'p', 't');
         INSERT INTO components (id, project_id, name) VALUES ('{c}', '{p}', 'c');
         INSERT INTO features (id, component_id, name) VALUES ('{f}', '{c}', 'f');
         INSERT INTO test_cases (id, feature_id, name, origin) VALUES ('{t}', '{f}', 'tc', 'manual');"
    ))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(include_str!("../migrations/0006_test_results.sql"))
        .execute(&pool)
        .await
        .unwrap();

    let name: String = sqlx::query_scalar("SELECT name FROM test_cases WHERE id = $1")
        .bind(t)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "tc");
    let insert = |run: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query(
                "INSERT INTO test_results (id, tenant_id, test_case_id, run_id, status, executed_at, source)
                 VALUES ($1, 't', $2, $3, 'passed', now(), 'junit')",
            )
            .bind(Uuid::new_v4())
            .bind(t)
            .bind(run)
            .execute(&pool)
            .await
        }
    };
    insert("r1").await.unwrap();
    insert("r2").await.unwrap();
    assert!(
        insert("r1").await.is_err(),
        "one result per test case and run"
    );

    // Deleting the test case deletes its results.
    sqlx::query("DELETE FROM test_cases WHERE id = $1")
        .bind(t)
        .execute(&pool)
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM test_results")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[tokio::test]
async fn applies_to_a_fresh_database() {
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
    Store::connect(&url, 1).await.unwrap();
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    let exists: bool = sqlx::query_scalar("SELECT to_regclass('public.test_results') IS NOT NULL")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(exists);
}
