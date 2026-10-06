//! JWT validation, roles and tenant resolution (FR-040 to FR-045).
//!
//! The gateway performs the OIDC handshake and forwards the access token in
//! `Authorization`; this module only validates it. See
//! docs/specification/08-usermanagement.md and
//! docs/adr/0005-jwt-auth-and-tenancy.md.
//!
//! Tokens are never logged: a validation failure reports why it failed, not
//! what the token contained.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{header, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::jwk::{AlgorithmParameters, JwkSet};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::error::ApiError;

/// Tenant used when `--dev-mode` skips validation.
pub const DEV_TENANT: &str = "dev";

/// Signature algorithms accepted from a JWKS.
///
/// The HMAC family and `none` are excluded on purpose: a JWKS distributes
/// public keys, so a symmetric or unsigned token can only be an attack.
const ALLOWED_ALGS: [Algorithm; 5] = [
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::ES256,
    Algorithm::ES384,
];

/// What a caller may do (FR-045).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Read-only.
    User,
    /// Read and write.
    Editor,
}

impl Role {
    /// Parses a role name, case-insensitively. Unknown names are not roles.
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "editor" => Some(Self::Editor),
            "user" => Some(Self::User),
            _ => None,
        }
    }

    /// Whether this role may use `method`.
    ///
    /// Decided by method rather than by route, so a new endpoint is covered
    /// the day it is added.
    pub fn may(self, method: &Method) -> bool {
        match self {
            Self::Editor => true,
            Self::User => matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS),
        }
    }

    /// Name for problem details and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Editor => "Editor",
        }
    }
}

/// The authenticated caller, attached to every request by [`authenticate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Org id from the configured claim; the tenant (FR-043).
    pub tenant: String,
    /// Highest role found in the token.
    pub role: Role,
    /// `sub`, when present. Used for audit, never for authorization.
    pub subject: Option<String>,
}

impl Identity {
    /// The identity `--dev-mode` runs as.
    pub fn dev() -> Self {
        Self {
            tenant: DEV_TENANT.to_owned(),
            role: Role::Editor,
            subject: Some("dev".to_owned()),
        }
    }
}

/// Lets handlers take `identity: Identity` as an argument.
impl<S: Send + Sync> FromRequestParts<S> for Identity {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Identity>().cloned().ok_or_else(|| {
            // Unreachable behind the middleware; a 500 is the honest answer if
            // a route is ever mounted without it.
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "request reached a handler without authentication",
            )
        })
    }
}

/// How tokens are validated.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWKS URL, or an OIDC discovery URL to resolve one from.
    pub jwks_url: String,
    /// Claim path to the org id (FR-043); see [`claim_at`].
    pub tenant_claim: String,
    /// Claim path to the role; see [`claim_at`].
    pub role_claim: String,
    /// Expected `iss`; unchecked when `None`.
    pub issuer: Option<String>,
    /// Expected `aud`; unchecked when `None`.
    pub audience: Option<String>,
    /// How long a fetched key set is reused.
    pub cache: Duration,
    /// Floor between refreshes triggered by an unknown `kid`.
    pub min_refresh: Duration,
}

/// Validates bearer tokens against a cached JWKS.
pub struct Authenticator {
    config: AuthConfig,
    http: reqwest::Client,
    keys: RwLock<CachedKeys>,
}

#[derive(Default)]
struct CachedKeys {
    set: Option<JwkSet>,
    fetched_at: Option<Instant>,
}

impl CachedKeys {
    fn age(&self) -> Option<Duration> {
        self.fetched_at.map(|t| t.elapsed())
    }
}

impl Authenticator {
    /// Creates an authenticator. Keys are fetched on first use.
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            keys: RwLock::new(CachedKeys::default()),
        }
    }

    /// Validates a raw `Authorization` header value.
    pub async fn authenticate(&self, header_value: &str) -> Result<Identity, ApiError> {
        let token = header_value
            .strip_prefix("Bearer ")
            .or_else(|| header_value.strip_prefix("bearer "))
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| unauthorized("expected an \"Authorization: Bearer <token>\" header"))?;

        let header = jsonwebtoken::decode_header(token)
            .map_err(|_| unauthorized("the token is not a well-formed JWT"))?;
        if !ALLOWED_ALGS.contains(&header.alg) {
            return Err(unauthorized(format!(
                "token algorithm {:?} is not accepted",
                header.alg
            )));
        }
        let kid = header
            .kid
            .ok_or_else(|| unauthorized("the token has no \"kid\", so no key can be selected"))?;

        let key = self.decoding_key(&kid, header.alg).await?;
        let mut validation = Validation::new(header.alg);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        match &self.config.issuer {
            Some(iss) => validation.set_issuer(&[iss]),
            None => validation.iss = None,
        }
        match &self.config.audience {
            Some(aud) => validation.set_audience(&[aud]),
            None => validation.validate_aud = false,
        }

        let claims = jsonwebtoken::decode::<Value>(token, &key, &validation)
            .map_err(|e| unauthorized(format!("the token was rejected: {}", e.kind_detail())))?
            .claims;

        let tenant = claim_at(&claims, &self.config.tenant_claim)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| {
                // Without a tenant there is no data the caller could be shown.
                forbidden(format!(
                    "the token carries no \"{}\" claim, so it is not scoped to a tenant",
                    self.config.tenant_claim
                ))
            })?
            .to_owned();

        Ok(Identity {
            tenant,
            role: role_from(claim_at(&claims, &self.config.role_claim)),
            subject: claims.get("sub").and_then(Value::as_str).map(str::to_owned),
        })
    }

    /// The key for `kid`, refreshing the key set when it is stale or unknown.
    async fn decoding_key(&self, kid: &str, alg: Algorithm) -> Result<DecodingKey, ApiError> {
        if let Some(key) = self.cached_key(kid, alg).await? {
            return Ok(key);
        }
        self.refresh(kid).await?;
        self.cached_key(kid, alg).await?.ok_or_else(|| {
            unauthorized("the token was signed with a key the provider does not publish")
        })
    }

    async fn cached_key(&self, kid: &str, alg: Algorithm) -> Result<Option<DecodingKey>, ApiError> {
        let keys = self.keys.read().await;
        let fresh = keys.age().is_some_and(|age| age < self.config.cache);
        if !fresh {
            return Ok(None);
        }
        match keys.set.as_ref().and_then(|s| s.find(kid)) {
            Some(jwk) => Ok(Some(to_decoding_key(jwk, alg)?)),
            None => Ok(None),
        }
    }

    /// Fetches the key set, unless a refresh happened too recently.
    async fn refresh(&self, kid: &str) -> Result<(), ApiError> {
        let mut keys = self.keys.write().await;
        // Another task may have refreshed while this one waited for the lock.
        if keys.set.as_ref().is_some_and(|s| {
            s.find(kid).is_some() && keys.age().is_some_and(|a| a < self.config.cache)
        }) {
            return Ok(());
        }
        if keys.age().is_some_and(|age| age < self.config.min_refresh) {
            // A provider rotating keys must not turn into a request amplifier.
            return Err(unauthorized(
                "the signing key is unknown and the key set was just refreshed",
            ));
        }
        let url = self.resolve_jwks_url().await?;
        let set: JwkSet = self
            .http
            .get(&url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| {
                tracing::error!(error = %e, "cannot fetch JWKS");
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "cannot reach the identity provider's key set",
                )
            })?
            .json()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "JWKS is not valid JSON");
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "the identity provider's key set could not be parsed",
                )
            })?;
        tracing::info!(keys = set.keys.len(), "fetched JWKS");
        keys.set = Some(set);
        keys.fetched_at = Some(Instant::now());
        Ok(())
    }

    /// Follows an OIDC discovery document to its `jwks_uri`, if that is what
    /// the configured URL points at.
    async fn resolve_jwks_url(&self) -> Result<String, ApiError> {
        let configured = &self.config.jwks_url;
        if !configured.contains("/.well-known/openid-configuration") {
            return Ok(configured.clone());
        }
        let doc: Value = self
            .http
            .get(configured)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| {
                tracing::error!(error = %e, "cannot fetch OIDC discovery document");
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "cannot reach the identity provider's discovery document",
                )
            })?
            .json()
            .await
            .map_err(|_| {
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "the discovery document could not be parsed",
                )
            })?;
        doc.get("jwks_uri")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "the discovery document has no \"jwks_uri\"",
                )
            })
    }
}

fn to_decoding_key(jwk: &jsonwebtoken::jwk::Jwk, alg: Algorithm) -> Result<DecodingKey, ApiError> {
    // A key must match the family the token claims, or an RSA key could be
    // offered for an EC token and vice versa.
    let ok = matches!(
        (&jwk.algorithm, alg),
        (
            AlgorithmParameters::RSA(_),
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512
        ) | (
            AlgorithmParameters::EllipticCurve(_),
            Algorithm::ES256 | Algorithm::ES384
        )
    );
    if !ok {
        return Err(unauthorized(
            "the signing key does not match the token's algorithm",
        ));
    }
    DecodingKey::from_jwk(jwk).map_err(|e| {
        tracing::error!(error = %e, "cannot build a decoding key from the JWKS entry");
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "the identity provider published a key this server cannot use",
        )
    })
}

/// The claim at `path`, which names a top-level claim or a claim nested in
/// objects, separated by dots: `tenant`, `org.id`, `edge.siemens.cloud.tenant`.
///
/// Claim names may themselves contain dots (namespaced claims such as
/// `edge.siemens.cloud` or `https://example.com/roles`), so at each level the
/// longest run of segments that names an existing key wins. That keeps a plain
/// claim name working unchanged and needs no escaping in configuration.
fn claim_at<'a>(claims: &'a Value, path: &str) -> Option<&'a Value> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut current = claims;
    let mut start = 0;
    while start < segments.len() {
        let object = current.as_object()?;
        let (end, value) = (start + 1..=segments.len()).rev().find_map(|end| {
            object
                .get(&segments[start..end].join("."))
                .map(|v| (end, v))
        })?;
        current = value;
        start = end;
    }
    Some(current)
}

/// Highest role in the claim. Accepts a string, a space-separated string or an
/// array of strings, which is what the common providers emit.
fn role_from(claim: Option<&Value>) -> Role {
    let mut best = Role::User;
    let mut consider = |raw: &str| {
        for part in raw.split_whitespace() {
            if Role::parse(part) == Some(Role::Editor) {
                best = Role::Editor;
            }
        }
    };
    match claim {
        Some(Value::String(s)) => consider(s),
        Some(Value::Array(items)) => {
            for item in items {
                if let Some(s) = item.as_str() {
                    consider(s);
                }
            }
        }
        _ => {}
    }
    best
}

fn unauthorized(detail: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, detail)
}

fn forbidden(detail: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, detail)
}

/// Why a token was rejected, without echoing the token.
trait KindDetail {
    fn kind_detail(&self) -> &'static str;
}

impl KindDetail for jsonwebtoken::errors::Error {
    fn kind_detail(&self) -> &'static str {
        use jsonwebtoken::errors::ErrorKind as K;
        match self.kind() {
            K::ExpiredSignature => "it has expired",
            K::ImmatureSignature => "it is not valid yet",
            K::InvalidSignature => "the signature does not verify",
            K::InvalidIssuer => "the issuer is not accepted",
            K::InvalidAudience => "the audience is not accepted",
            K::InvalidAlgorithm => "the algorithm does not match the key",
            _ => "it is not a valid token",
        }
    }
}

/// How the authenticator was configured for this process.
#[derive(Clone)]
pub enum AuthMode {
    /// `--dev-mode`: validation is skipped (FR-040).
    Dev,
    /// Tokens are validated against a JWKS.
    Jwt(Arc<Authenticator>),
}

/// Authenticates the request and enforces the role rule (FR-041, FR-045).
///
/// `GET /health` is left open so a load balancer does not need a token.
pub async fn authenticate(
    axum::extract::State(mode): axum::extract::State<AuthMode>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let identity = match &mode {
        AuthMode::Dev => Identity::dev(),
        AuthMode::Jwt(auth) => {
            let raw = request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| unauthorized("no \"Authorization\" header"))?;
            auth.authenticate(raw).await?
        }
    };

    if !identity.role.may(request.method()) {
        return Err(forbidden(format!(
            "the {} role may not {} this resource",
            identity.role.as_str(),
            request.method()
        )));
    }

    request.extensions_mut().insert(identity);
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn editor_may_write_and_user_may_not() {
        // FR-045
        assert!(Role::Editor.may(&Method::POST));
        assert!(Role::Editor.may(&Method::DELETE));
        assert!(Role::User.may(&Method::GET));
        assert!(Role::User.may(&Method::HEAD));
        assert!(!Role::User.may(&Method::POST));
        assert!(!Role::User.may(&Method::PUT));
        assert!(!Role::User.may(&Method::PATCH));
        assert!(!Role::User.may(&Method::DELETE));
    }

    #[test]
    fn role_claim_accepts_string_list_and_array() {
        assert_eq!(role_from(Some(&json!("Editor"))), Role::Editor);
        assert_eq!(role_from(Some(&json!("user editor"))), Role::Editor);
        assert_eq!(role_from(Some(&json!(["user", "EDITOR"]))), Role::Editor);
        assert_eq!(role_from(Some(&json!(["user"]))), Role::User);
    }

    #[test]
    fn unknown_or_missing_role_is_read_only() {
        assert_eq!(role_from(None), Role::User);
        assert_eq!(role_from(Some(&json!("admin"))), Role::User);
        assert_eq!(role_from(Some(&json!(42))), Role::User);
    }

    #[test]
    fn a_plain_claim_name_is_a_top_level_claim() {
        let claims = json!({ "tenant": "acme", "org": { "tenant": "nested" } });
        assert_eq!(claim_at(&claims, "tenant"), Some(&json!("acme")));
    }

    #[test]
    fn a_dotted_path_reaches_into_nested_objects() {
        let claims = json!({ "org": { "id": "acme" } });
        assert_eq!(claim_at(&claims, "org.id"), Some(&json!("acme")));
    }

    #[test]
    fn a_namespaced_claim_name_containing_dots_is_matched_whole() {
        let claims = json!({
            "edge.siemens.cloud": { "tenant": "xyz", "roles": ["editor"] }
        });
        assert_eq!(
            claim_at(&claims, "edge.siemens.cloud.tenant"),
            Some(&json!("xyz"))
        );
        assert_eq!(
            role_from(claim_at(&claims, "edge.siemens.cloud.roles")),
            Role::Editor
        );
        assert_eq!(
            claim_at(
                &json!({ "https://example.com/roles": "editor" }),
                "https://example.com/roles"
            ),
            Some(&json!("editor"))
        );
    }

    #[test]
    fn a_path_that_does_not_resolve_is_missing() {
        let claims = json!({ "org": { "id": "acme" }, "flat": "x" });
        assert_eq!(claim_at(&claims, "org.name"), None);
        assert_eq!(claim_at(&claims, "flat.deeper"), None);
        assert_eq!(claim_at(&claims, "edge.siemens.cloud.tenant"), None);
    }

    #[test]
    fn dev_identity_is_editor_in_the_dev_tenant() {
        // FR-040
        let id = Identity::dev();
        assert_eq!(id.tenant, DEV_TENANT);
        assert_eq!(id.role, Role::Editor);
    }
}
