//! Command line and environment configuration (`TM_` prefix).

use std::net::SocketAddr;

use clap::Parser;

/// TestModeller backend server.
#[derive(Debug, Clone, Parser)]
#[command(name = "testmodeller", version, about)]
pub struct Config {
    /// Start a throwaway PostgreSQL container with testcontainers (requires Docker).
    /// Data is lost when the server stops.
    #[arg(long)]
    pub dev_mode: bool,

    /// PostgreSQL connection URL; required unless --dev-mode is set.
    #[arg(long, env = "TM_DATABASE_URL", hide_env_values = true)]
    pub database_url: Option<String>,

    /// Address to listen on.
    #[arg(long, env = "TM_BIND_ADDR", default_value = "127.0.0.1:8080")]
    pub bind: SocketAddr,

    /// Maximum database connections.
    #[arg(long, env = "TM_DB_MAX_CONNECTIONS", default_value_t = 10)]
    pub db_max_connections: u32,

    /// Allowed CORS origin (e.g. http://localhost:4200). Defaults to that origin in dev mode.
    #[arg(long, env = "TM_CORS_ORIGIN")]
    pub cors_origin: Option<String>,

    /// PostgreSQL image tag used in dev mode.
    #[arg(long, env = "TM_DEV_POSTGRES_TAG", default_value = "16-alpine")]
    pub dev_postgres_tag: String,

    /// API key for the configured LLM provider. Never persisted or logged.
    #[arg(long, env = "TM_AI_API_KEY", hide_env_values = true)]
    pub ai_api_key: Option<String>,
}
