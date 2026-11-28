//! Retention of finished AI jobs and decided proposals (ADR 0019). Requires Docker.

use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use tm_domain::JobStatus;
use tm_storage::{NamedFields, Store};
use uuid::Uuid;

#[tokio::test]
async fn purge_keeps_running_jobs_and_pending_proposals() {
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
    let store = Store::connect(&url, 4).await.unwrap();
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    let project = store
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

    // Jobs: an old finished one goes; an old running one and a fresh finished one stay.
    let old_done = store.create_job("t").await.unwrap().id;
    let old_running = store.create_job("t").await.unwrap().id;
    let fresh_done = store.create_job("t").await.unwrap().id;
    store
        .update_job(old_done, JobStatus::Succeeded, &[], None)
        .await
        .unwrap();
    store
        .update_job(old_running, JobStatus::Running, &[], None)
        .await
        .unwrap();
    store
        .update_job(fresh_done, JobStatus::Failed, &[], Some("x"))
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET updated_at = now() - interval '30 days' WHERE id = ANY($1)")
        .bind([old_done, old_running])
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(store.purge_finished_jobs(7).await.unwrap(), 1);
    for (id, kept) in [(old_done, false), (old_running, true), (fresh_done, true)] {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n == 1, kept, "{id}");
    }

    // Proposals: deciding sets `decided_at`; only old decided ones are purged.
    let mut ids = vec![];
    for status in ["pending", "pending", "pending"] {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO proposals (id, project_id, kind, status, payload)
             VALUES ($1, $2, 'test-cases', $3, '{}')",
        )
        .bind(id)
        .bind(project)
        .bind(status)
        .execute(&pool)
        .await
        .unwrap();
        ids.push(id);
    }
    let (old_decided, fresh_decided, pending) = (ids[0], ids[1], ids[2]);
    for id in [old_decided, fresh_decided] {
        sqlx::query("UPDATE proposals SET status = 'rejected' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let stamped: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT decided_at FROM proposals WHERE id = $1")
            .bind(fresh_decided)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(stamped.is_some(), "deciding records decided_at");
    sqlx::query("UPDATE proposals SET decided_at = now() - interval '200 days' WHERE id = $1")
        .bind(old_decided)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE proposals SET created_at = now() - interval '400 days' WHERE id = $1")
        .bind(pending)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(store.purge_decided_proposals(90).await.unwrap(), 1);
    let left: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM proposals ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(!left.contains(&old_decided));
    assert!(left.contains(&fresh_decided) && left.contains(&pending));
}
