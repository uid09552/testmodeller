//! Schema invariants after all migrations. Requires Docker.

use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use tm_storage::Store;

/// Every single-column foreign key has an index that starts with its column,
/// so deleting the referenced row never scans the referencing table.
#[tokio::test]
async fn every_foreign_key_is_indexed() {
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
    // Connecting runs the migrations.
    Store::connect(&url, 1).await.unwrap();
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    let missing: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.conrelid::regclass::text, a.attname::text
         FROM pg_constraint c
         JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = c.conkey[1]
         WHERE c.contype = 'f' AND array_length(c.conkey, 1) = 1
           AND NOT EXISTS (
             SELECT 1 FROM pg_index i
             WHERE i.indrelid = c.conrelid AND i.indkey[0] = c.conkey[1])
         ORDER BY 1, 2",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        missing.is_empty(),
        "foreign keys without an index: {missing:?}"
    );
}
