//! HTTP API entry point. See docs/specification/05-api.md.

use anyhow::{bail, Context};
use axum::http::{HeaderValue, Method};
use clap::Parser;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tm_api::config::Config;
use tm_api::{router, AppState};
use tm_storage::Store;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

const DEV_CORS_ORIGIN: &str = "http://localhost:4200";

/// Starts a disposable PostgreSQL container and returns it with its connection URL.
async fn start_dev_database(tag: &str) -> anyhow::Result<(ContainerAsync<Postgres>, String)> {
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

    let mut app =
        router(AppState::new(store, config.ai_api_key.clone())).layer(TraceLayer::new_for_http());
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
        tracing::info!("dev mode: removing PostgreSQL container");
        c.rm().await?;
    }
    Ok(())
}
