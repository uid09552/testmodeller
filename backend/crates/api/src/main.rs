//! HTTP API entry point. See docs/specification/05-api.md.

use anyhow::{bail, Context};
use axum::http::{HeaderValue, Method};
use clap::Parser;
use std::sync::Arc;
#[cfg(feature = "dev-mode")]
use testcontainers::runners::AsyncRunner;
#[cfg(feature = "dev-mode")]
use testcontainers::{ContainerAsync, ImageExt};
#[cfg(feature = "dev-mode")]
use testcontainers_modules::postgres::Postgres;

use tm_api::config::Config;
use tm_api::secrets::{KeySource, SecretStore};
use tm_api::{auth_mode, router, AppState};
use tm_storage::Store;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

const DEV_CORS_ORIGIN: &str = "http://localhost:4200";

/// The dev-mode database, removed again at shutdown.
#[cfg(feature = "dev-mode")]
type DevDatabase = ContainerAsync<Postgres>;
/// Without the `dev-mode` feature there is never one.
#[cfg(not(feature = "dev-mode"))]
type DevDatabase = std::convert::Infallible;

/// Without the `dev-mode` feature, `--dev-mode` cannot start a database.
#[cfg(not(feature = "dev-mode"))]
async fn start_dev_database(_tag: &str) -> anyhow::Result<(DevDatabase, String)> {
    bail!(
        "--dev-mode needs a build with the dev-mode feature (the default); \
         this binary was built with --no-default-features, so set TM_DATABASE_URL"
    )
}

/// Starts a disposable PostgreSQL container and returns it with its connection URL.
#[cfg(feature = "dev-mode")]
async fn start_dev_database(tag: &str) -> anyhow::Result<(DevDatabase, String)> {
    tracing::info!(image = %format!("postgres:{tag}"), "dev mode: starting PostgreSQL container");
    let container = Postgres::default()
        .with_db_name("testmodeller")
        .with_user("testmodeller")
        .with_password("testmodeller")
        .with_tag(tag)
        .start()
        .await
        .context("cannot start PostgreSQL container; is Docker running?")?;
    let host = container.get_host().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://testmodeller:testmodeller@{host}:{port}/testmodeller");
    tracing::info!(%host, %port, "dev mode: PostgreSQL ready (user/password/db: testmodeller)");
    Ok((container, url))
}

#[cfg(feature = "dev-mode")]
async fn remove_dev_database(c: DevDatabase) -> anyhow::Result<()> {
    tracing::info!("dev mode: removing PostgreSQL container");
    c.rm().await?;
    Ok(())
}

#[cfg(not(feature = "dev-mode"))]
async fn remove_dev_database(c: DevDatabase) -> anyhow::Result<()> {
    match c {}
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "cannot listen for ctrl-c");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => tracing::error!(error = %e, "cannot listen for SIGTERM"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn")),
        )
        .init();
    let config = Config::parse();

    let (container, database_url) = if config.dev_mode {
        if config.database_url.is_some() {
            tracing::warn!("--dev-mode set: ignoring TM_DATABASE_URL / --database-url");
        }
        let (c, url) = start_dev_database(&config.dev_postgres_tag).await?;
        (Some(c), url)
    } else {
        match config.database_url.clone() {
            Some(url) => (None, url),
            None => bail!("set TM_DATABASE_URL (or --database-url), or run with --dev-mode"),
        }
    };

    let store = Store::connect(&database_url, config.db_max_connections)
        .await
        .context("cannot connect to PostgreSQL or run migrations")?;
    let interrupted = store.fail_interrupted_jobs().await?;
    if interrupted > 0 {
        tracing::warn!(count = interrupted, "marked interrupted AI jobs as failed");
    }

    spawn_retention(
        store.clone(),
        config.ai_job_retention_days,
        config.ai_proposal_retention_days,
    );

    // Fails fast when neither --dev-mode nor a JWKS URL is set, rather than
    // starting an unauthenticated server (FR-040).
    let auth = config.auth()?;
    if auth.is_none() {
        tracing::warn!("--dev-mode set: JWT validation is skipped and every request is an Editor");
    }

    // The AI provider key is always persisted; what differs is how well it is
    // protected. With TM_SECRET_KEY the sealing key lives outside the database;
    // without it the server generates one and keeps it in the database, which
    // survives restarts but does not survive a dump of the database itself.
    let secrets = match config.secret_key.as_deref().filter(|k| !k.is_empty()) {
        Some(raw) => {
            tracing::info!("secrets are sealed with TM_SECRET_KEY");
            SecretStore::from_config(raw).map_err(|e| anyhow::anyhow!(e))?
        }
        None => {
            let key = store
                .get_or_create_data_key(&SecretStore::generate_key())
                .await
                .context("cannot read or create the server data key")?;
            if config.ai_api_key.as_deref().is_none_or(str::is_empty) {
                tracing::warn!(
                    "TM_SECRET_KEY is not set: a provider key saved through the API is \
                     sealed with a key stored in the database. Prefer TM_AI_API_KEY."
                );
            }
            SecretStore::from_bytes(&key, KeySource::Generated).map_err(|e| anyhow::anyhow!(e))?
        }
    };

    // AI settings from the environment win over whatever is stored, so .env
    // (via compose.yaml) is the one place to configure the provider.
    if let Some(settings) = config.ai_settings()? {
        store
            .put_ai_settings(&settings)
            .await
            .context("cannot apply the AI settings from the environment")?;
        tracing::info!(
            provider = settings.provider.as_str(),
            model = settings.model.as_deref().unwrap_or("default"),
            "AI provider configured from the environment"
        );
    }

    let mut state = AppState::new(store, config.ai_api_key.clone(), Arc::new(secrets));
    if let Some(oauth) = config.mcp_oauth_resource()? {
        state = state.with_oauth(oauth);
    }
    state.ai.load_persisted_key().await;

    let mut app = router(state, auth_mode(auth)).layer(TraceLayer::new_for_http());
    let origin = config
        .cors_origin
        .clone()
        .or_else(|| config.dev_mode.then(|| DEV_CORS_ORIGIN.to_owned()));
    if let Some(origin) = origin {
        let origin: HeaderValue = origin.parse().context("invalid CORS origin")?;
        app = app.layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                ])
                .allow_headers(tower_http::cors::Any)
                .expose_headers(tower_http::cors::Any),
        );
    }

    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("cannot bind {}", config.bind))?;
    tracing::info!(addr = %config.bind, "listening on http://{}/api/v1", config.bind);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    if let Some(c) = container {
        remove_dev_database(c).await?;
    }
    Ok(())
}

/// Deletes old AI history once an hour. Jobs go first so no job outlives the proposals it
/// lists. Logs counts only.
fn spawn_retention(store: Store, job_days: u32, proposal_days: u32) {
    if job_days == 0 && proposal_days == 0 {
        return;
    }
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            tick.tick().await;
            if job_days > 0 {
                match store.purge_finished_jobs(job_days).await {
                    Ok(n) if n > 0 => tracing::info!(count = n, "purged finished AI jobs"),
                    Ok(_) => {}
                    Err(e) => tracing::error!(error = %e, "AI job retention failed"),
                }
            }
            if proposal_days > 0 {
                match store.purge_decided_proposals(proposal_days).await {
                    Ok(n) if n > 0 => tracing::info!(count = n, "purged decided AI proposals"),
                    Ok(_) => {}
                    Err(e) => tracing::error!(error = %e, "AI proposal retention failed"),
                }
            }
        }
    });
}
