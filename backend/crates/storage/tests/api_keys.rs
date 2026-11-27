//! API keys against a real PostgreSQL (FR-090). Requires Docker.

use chrono::{Duration, Utc};
use testcontainers::runners::AsyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;
use tm_storage::{NewApiKey, StorageError, Store, MAX_ACTIVE_KEYS};
use uuid::Uuid;

fn hash(n: u8) -> [u8; 32] {
    [n; 32]
}

fn key<'a>(tenant: &'a str, subject: &'a str, hash: &'a [u8]) -> NewApiKey<'a> {
    NewApiKey {
        tenant,
        subject,
        name: "laptop",
        prefix: "tm_abc123",
        hash,
        role: "Editor",
        expires_at: Some(Utc::now() + Duration::days(90)),
    }
}

#[tokio::test]
async fn keys_are_scoped_limited_revocable_and_expire() {
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

    // Create and find by hash; the use is recorded.
    let h1 = hash(1);
    let created = store.create_api_key(key("t1", "alice", &h1)).await.unwrap();
    assert_eq!(created.role, "Editor");
    assert!(created.last_used_at.is_none());
    let used = store.use_api_key(&h1).await.unwrap().expect("active key");
    assert_eq!(used.id, created.id);
    let listed = store.list_api_keys("t1", "alice").await.unwrap();
    assert!(listed[0].last_used_at.is_some());

    // Another user, or the same subject in another tenant, sees nothing and cannot revoke.
    assert!(store.list_api_keys("t1", "bob").await.unwrap().is_empty());
    assert!(store.list_api_keys("t2", "alice").await.unwrap().is_empty());
    assert!(matches!(
        store.revoke_api_key("t1", "bob", created.id).await,
        Err(StorageError::NotFound(_))
    ));

    // Revoked keys stop working and drop out of the list.
    store
        .revoke_api_key("t1", "alice", created.id)
        .await
        .unwrap();
    assert!(store.use_api_key(&h1).await.unwrap().is_none());
    assert!(store.list_api_keys("t1", "alice").await.unwrap().is_empty());
    assert!(store
        .revoke_api_key("t1", "alice", created.id)
        .await
        .is_err());

    // Expired keys stop working.
    let h2 = hash(2);
    let mut expired = key("t1", "alice", &h2);
    expired.expires_at = Some(Utc::now() - Duration::seconds(1));
    store.create_api_key(expired).await.unwrap();
    assert!(store.use_api_key(&h2).await.unwrap().is_none());

    // The active-key limit, counted per owner.
    let hashes: Vec<[u8; 32]> = (10..10 + MAX_ACTIVE_KEYS as u8).map(hash).collect();
    for h in &hashes {
        store.create_api_key(key("t1", "alice", h)).await.unwrap();
    }
    let over = hash(99);
    assert!(matches!(
        store.create_api_key(key("t1", "alice", &over)).await,
        Err(StorageError::Invalid(_))
    ));
    store.create_api_key(key("t1", "bob", &over)).await.unwrap();

    // Unknown ids are not found.
    assert!(store
        .revoke_api_key("t1", "alice", Uuid::new_v4())
        .await
        .is_err());
}
