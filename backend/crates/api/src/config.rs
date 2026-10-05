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

    /// AI provider: `anthropic` (or `claude`), `local` (or `ollama`),
    /// `openai-compatible`, or `none`. When set, the AI settings come from the
    /// environment and override what is stored (07-ai-integration.md).
    #[arg(long, env = "TM_AI_PROVIDER")]
    pub ai_provider: Option<String>,

    /// Base URL of the provider's API, e.g. `http://localhost:11434/v1` for
    /// Ollama. Optional for Anthropic, which has a default.
    #[arg(long, env = "TM_AI_BASE_URL")]
    pub ai_base_url: Option<String>,

    /// Model id, e.g. `claude-opus-5-5` or `llama3.1`.
    #[arg(long, env = "TM_AI_MODEL")]
    pub ai_model: Option<String>,

    /// Output token budget per request.
    ///
    /// Taken as text: compose passes an unset variable through as an empty
    /// string, which must mean "not set" rather than fail integer parsing.
    #[arg(long, env = "TM_AI_MAX_TOKENS")]
    pub ai_max_tokens: Option<String>,

    /// API key for the configured LLM provider. Never logged. Not needed for
    /// a local Ollama.
    #[arg(long, env = "TM_AI_API_KEY", hide_env_values = true)]
    pub ai_api_key: Option<String>,

    /// JWKS URL, or an OIDC discovery URL to resolve one from. Required unless
    /// --dev-mode (FR-042).
    #[arg(long, env = "TM_JWKS_URL")]
    pub jwks_url: Option<String>,

    /// Claim holding the org id used as the tenant (FR-043).
    #[arg(long, env = "TM_TENANT_CLAIM", default_value = "tenant")]
    pub tenant_claim: String,

    /// Claim holding the caller's role.
    #[arg(long, env = "TM_ROLE_CLAIM", default_value = "roles")]
    pub role_claim: String,

    /// Expected token issuer; not checked when unset.
    #[arg(long, env = "TM_JWT_ISSUER")]
    pub jwt_issuer: Option<String>,

    /// Expected token audience; not checked when unset.
    #[arg(long, env = "TM_JWT_AUDIENCE")]
    pub jwt_audience: Option<String>,

    /// How long a fetched JWKS is reused, in seconds.
    #[arg(long, env = "TM_JWKS_CACHE_SECS", default_value_t = 300)]
    pub jwks_cache_secs: u64,

    /// Minimum seconds between JWKS refreshes triggered by an unknown key id.
    #[arg(long, env = "TM_JWKS_MIN_REFRESH_SECS", default_value_t = 30)]
    pub jwks_min_refresh_secs: u64,

    /// 32-byte key (base64 or hex) that encrypts secrets stored in the
    /// database, such as the AI provider key. Without it the provider key is
    /// kept in memory only. Generate one with `openssl rand -base64 32`.
    #[arg(long, env = "TM_SECRET_KEY", hide_env_values = true)]
    pub secret_key: Option<String>,
}

impl Config {
    /// AI settings from the environment, or `None` when `TM_AI_PROVIDER` is
    /// unset and the stored settings apply.
    ///
    /// Checked here rather than on first use, so a typo in `.env` stops the
    /// server at startup instead of surfacing as a failed proposal later.
    pub fn ai_settings(&self) -> anyhow::Result<Option<tm_domain::AiSettings>> {
        use tm_domain::AiProvider;
        let Some(raw) = self
            .ai_provider
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
        else {
            return Ok(None);
        };
        // Product names are accepted as well as the contract's protocol names,
        // because "ollama" is what people will write in a .env file.
        let provider = match raw.to_ascii_lowercase().as_str() {
            "claude" => AiProvider::Anthropic,
            "ollama" => AiProvider::Local,
            other => other.parse::<AiProvider>().map_err(|_| {
                anyhow::anyhow!(
                    "TM_AI_PROVIDER={raw} is not one of anthropic, claude, local, ollama, \
                     openai-compatible, none"
                )
            })?,
        };

        let nonempty = |v: &Option<String>| {
            v.as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let mut base_url = nonempty(&self.ai_base_url);
        let model = nonempty(&self.ai_model);

        if let Some(url) = &base_url {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                anyhow::bail!("TM_AI_BASE_URL must be an http(s) URL, got {url}");
            }
        }
        if provider == AiProvider::Local && base_url.is_none() {
            // Ollama's default address, so `TM_AI_PROVIDER=ollama` alone works.
            base_url = Some("http://localhost:11434/v1".to_owned());
        }
        let needs_endpoint = matches!(provider, AiProvider::Local | AiProvider::OpenAiCompatible);
        if needs_endpoint && model.is_none() {
            anyhow::bail!("TM_AI_MODEL is required for TM_AI_PROVIDER={raw}");
        }
        if provider == AiProvider::OpenAiCompatible && base_url.is_none() {
            anyhow::bail!("TM_AI_BASE_URL is required for TM_AI_PROVIDER={raw}");
        }
        let max_tokens = match nonempty(&self.ai_max_tokens) {
            None => None,
            Some(raw) => match raw.parse::<i32>() {
                Ok(t) if t >= 1 => Some(t),
                _ => anyhow::bail!(
                    "TM_AI_MAX_TOKENS must be a whole number of at least 1, got {raw}"
                ),
            },
        };

        Ok(Some(tm_domain::AiSettings {
            provider,
            base_url,
            model,
            max_tokens_per_request: max_tokens,
        }))
    }

    /// Token validation settings, or `None` in dev mode (FR-040).
    pub fn auth(&self) -> anyhow::Result<Option<crate::auth::AuthConfig>> {
        use std::time::Duration;
        if self.dev_mode {
            return Ok(None);
        }
        let jwks_url = self
            .jwks_url
            .clone()
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!("set TM_JWKS_URL (or --jwks-url), or run with --dev-mode")
            })?;
        Ok(Some(crate::auth::AuthConfig {
            jwks_url,
            tenant_claim: self.tenant_claim.clone(),
            role_claim: self.role_claim.clone(),
            // Compose passes unset variables as empty strings; an empty issuer
            // would otherwise be enforced and reject every token.
            issuer: self.jwt_issuer.clone().filter(|v| !v.trim().is_empty()),
            audience: self.jwt_audience.clone().filter(|v| !v.trim().is_empty()),
            cache: Duration::from_secs(self.jwks_cache_secs),
            min_refresh: Duration::from_secs(self.jwks_min_refresh_secs),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tm_domain::AiProvider;

    fn config(args: &[&str]) -> Config {
        let mut full = vec!["testmodeller"];
        full.extend_from_slice(args);
        Config::try_parse_from(full).expect("arguments should parse")
    }

    #[test]
    fn no_provider_means_the_stored_settings_apply() {
        assert!(config(&[]).ai_settings().unwrap().is_none());
    }

    #[test]
    fn ollama_alone_gets_the_default_local_endpoint() {
        let s = config(&["--ai-provider", "ollama", "--ai-model", "llama3.1"])
            .ai_settings()
            .unwrap()
            .unwrap();
        assert_eq!(s.provider, AiProvider::Local);
        assert_eq!(s.base_url.as_deref(), Some("http://localhost:11434/v1"));
        assert_eq!(s.model.as_deref(), Some("llama3.1"));
    }

    #[test]
    fn claude_is_an_alias_for_anthropic_and_needs_no_endpoint() {
        let s = config(&["--ai-provider", "Claude", "--ai-model", "claude-opus-5-5"])
            .ai_settings()
            .unwrap()
            .unwrap();
        assert_eq!(s.provider, AiProvider::Anthropic);
        assert_eq!(s.base_url, None);
    }

    #[test]
    fn the_contract_names_are_accepted_too() {
        let s = config(&[
            "--ai-provider",
            "openai-compatible",
            "--ai-base-url",
            "https://llm.example.com/v1",
            "--ai-model",
            "gpt-x",
        ])
        .ai_settings()
        .unwrap()
        .unwrap();
        assert_eq!(s.provider, AiProvider::OpenAiCompatible);
    }

    #[test]
    fn an_unknown_provider_stops_startup() {
        let err = config(&["--ai-provider", "olama"])
            .ai_settings()
            .unwrap_err();
        assert!(err.to_string().contains("olama"));
    }

    #[test]
    fn ollama_without_a_model_stops_startup() {
        assert!(config(&["--ai-provider", "ollama"]).ai_settings().is_err());
    }

    #[test]
    fn openai_compatible_without_an_endpoint_stops_startup() {
        assert!(
            config(&["--ai-provider", "openai-compatible", "--ai-model", "m"])
                .ai_settings()
                .is_err()
        );
    }

    #[test]
    fn a_base_url_must_be_http() {
        assert!(config(&[
            "--ai-provider",
            "ollama",
            "--ai-model",
            "m",
            "--ai-base-url",
            "localhost:11434",
        ])
        .ai_settings()
        .is_err());
    }

    #[test]
    fn an_empty_token_budget_means_unset() {
        // What compose passes for `${TM_AI_MAX_TOKENS:-}` when .env leaves it out.
        let s = config(&[
            "--ai-provider",
            "claude",
            "--ai-model",
            "m",
            "--ai-max-tokens",
            "",
        ])
        .ai_settings()
        .unwrap()
        .unwrap();
        assert_eq!(s.max_tokens_per_request, None);
    }

    #[test]
    fn empty_values_from_compose_do_not_count_as_set() {
        let s = config(&[
            "--ai-provider",
            "ollama",
            "--ai-model",
            "llama3.1",
            "--ai-base-url",
            "",
        ])
        .ai_settings()
        .unwrap()
        .unwrap();
        assert_eq!(s.base_url.as_deref(), Some("http://localhost:11434/v1"));
    }

    #[test]
    fn an_empty_jwt_issuer_is_not_enforced() {
        // Otherwise every token would fail with "issuer not accepted".
        let auth = config(&[
            "--jwks-url",
            "https://idp.example.com/jwks",
            "--jwt-issuer",
            "",
            "--jwt-audience",
            "",
        ])
        .auth()
        .unwrap()
        .unwrap();
        assert_eq!(auth.issuer, None);
        assert_eq!(auth.audience, None);
    }

    #[test]
    fn an_empty_jwks_url_is_missing_not_a_url() {
        assert!(config(&["--jwks-url", ""]).auth().is_err());
    }

    #[test]
    fn a_non_positive_token_budget_stops_startup() {
        assert!(config(&[
            "--ai-provider",
            "claude",
            "--ai-model",
            "m",
            "--ai-max-tokens",
            "0",
        ])
        .ai_settings()
        .is_err());
    }
}
