//! A batteries-included [`Transport`] over `reqwest`, with optional `OAuth2`.

use std::sync::Arc;

use http::{HeaderMap, HeaderValue, StatusCode, header};
use serde::Deserialize;
use tokio::sync::Mutex;

use super::error::{Error, Result};
use super::transport::{TmfRequest, TmfResponse, Transport, TransportError};

/// How the transport authenticates against the API.
///
/// The [`Debug`] implementation is hand-written and **redacts every secret**.
/// A transport is the kind of value that ends up in a `tracing` span, a
/// `dbg!` during a bad afternoon, or a panic message shipped to an error
/// tracker — and a derived `Debug` would put a live bearer token in all three.
/// What it prints is the scheme, which is the part worth seeing.
#[derive(Clone, Default)]
#[non_exhaustive]
pub enum Auth {
    /// Send no credentials.
    #[default]
    None,
    /// Send a fixed bearer token.
    Bearer(String),
    /// Send HTTP basic credentials.
    Basic {
        /// Username.
        username: String,
        /// Password.
        password: String,
    },
    /// Fetch and refresh a token via the `OAuth2` client-credentials grant.
    ///
    /// This is how TM Forum APIs are usually deployed behind an API gateway.
    ClientCredentials(Box<ClientCredentials>),
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Bearer(_) => f.write_str("Bearer(<redacted>)"),
            Self::Basic { username, .. } => f
                .debug_struct("Basic")
                .field("username", username)
                .field("password", &"<redacted>")
                .finish(),
            Self::ClientCredentials(config) => {
                f.debug_tuple("ClientCredentials").field(config).finish()
            }
        }
    }
}

/// `OAuth2` client-credentials configuration.
///
/// [`Debug`] redacts `client_secret`; see [`Auth`] for why.
#[derive(Clone)]
#[non_exhaustive]
pub struct ClientCredentials {
    /// The token endpoint URL.
    pub token_url: String,
    /// The client identifier.
    pub client_id: String,
    /// The client secret.
    pub client_secret: String,
    /// Scopes to request, if any.
    pub scopes: Vec<String>,
}

impl std::fmt::Debug for ClientCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientCredentials")
            .field("token_url", &self.token_url)
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .field("scopes", &self.scopes)
            .finish()
    }
}

impl ClientCredentials {
    /// Configures a client-credentials grant.
    pub fn new(
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            token_url: token_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            scopes: Vec::new(),
        }
    }

    /// Requests the given scopes.
    #[must_use]
    pub fn with_scopes<I: IntoIterator<Item = S>, S: Into<String>>(mut self, scopes: I) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Clone)]
struct CachedToken {
    value: String,
    /// When to fetch a new one — not when the token expires.
    ///
    /// Held pre-adjusted so the refresh margin is decided once, against the
    /// lifetime the server actually granted; see [`refresh_at`].
    refresh_at: std::time::Instant,
}

/// When a token granted `lifetime` from now should be replaced.
///
/// Refreshing early stops a token expiring in flight. Subtracting a fixed
/// margin does not survive a short lifetime, though: an authorization server
/// that grants 10-second tokens against a 30-second margin yields a cache entry
/// that is stale the moment it is written, so *every* API call fetches a token
/// first — turning a cache into a second request per request, and a load
/// problem for the authorization server.
///
/// So the margin is the smaller of 30 seconds and half the lifetime. A
/// long-lived token gets the full margin; a short-lived one is still used for
/// half its life.
fn refresh_at(lifetime: std::time::Duration) -> std::time::Instant {
    const MARGIN: std::time::Duration = std::time::Duration::from_secs(30);
    std::time::Instant::now() + lifetime.saturating_sub(MARGIN.min(lifetime / 2))
}

/// A [`Transport`] backed by [`reqwest`].
///
/// ```no_run
/// use rutmf::api::{Auth, ReqwestTransport};
///
/// let transport = ReqwestTransport::builder()
///     .auth(Auth::Bearer("token".into()))
///     .build()
///     .unwrap();
/// ```
///
/// Its [`Debug`] output carries no credentials — neither the configured
/// [`Auth`], nor a cached `OAuth2` token, nor the values of default headers,
/// which is where an `Authorization` or an API key would otherwise sit.
#[derive(Clone)]
pub struct ReqwestTransport {
    client: reqwest::Client,
    auth: Auth,
    token: Arc<Mutex<Option<CachedToken>>>,
    default_headers: HeaderMap,
}

impl std::fmt::Debug for ReqwestTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Header *names* are useful when debugging; their values are not worth
        // the risk that one of them is a credential.
        let header_names: Vec<&str> = self
            .default_headers
            .keys()
            .map(http::HeaderName::as_str)
            .collect();
        f.debug_struct("ReqwestTransport")
            .field("auth", &self.auth)
            .field("default_headers", &header_names)
            .finish_non_exhaustive()
    }
}

impl ReqwestTransport {
    /// Creates a transport with default settings and no authentication.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the underlying HTTP client cannot be
    /// constructed, which generally means a broken TLS configuration.
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// Starts configuring a transport.
    #[must_use]
    pub fn builder() -> ReqwestTransportBuilder {
        ReqwestTransportBuilder::default()
    }

    /// Resolves the `Authorization` header value for the next request.
    async fn authorization(&self) -> Result<Option<HeaderValue>> {
        let raw = match &self.auth {
            Auth::None => return Ok(None),
            Auth::Bearer(token) => format!("Bearer {token}"),
            Auth::Basic { username, password } => {
                format!("Basic {}", base64_encode(&format!("{username}:{password}")))
            }
            Auth::ClientCredentials(config) => {
                format!("Bearer {}", self.client_credentials_token(config).await?)
            }
        };
        HeaderValue::from_str(&raw)
            .map(Some)
            .map_err(|e| Error::Transport(TransportError::new(e)))
    }

    /// Returns a cached token, fetching a fresh one when it is near expiry.
    async fn client_credentials_token(&self, config: &ClientCredentials) -> Result<String> {
        // The lock is deliberately held across the token request: it gives
        // single-flight refresh, so a burst of calls that all find the token
        // expired makes one request to the authorization server rather than
        // one each.
        let mut cached = self.token.lock().await;
        if let Some(token) = cached.as_ref()
            && std::time::Instant::now() < token.refresh_at
        {
            return Ok(token.value.clone());
        }

        let mut form = vec![
            ("grant_type", "client_credentials".to_owned()),
            ("client_id", config.client_id.clone()),
            ("client_secret", config.client_secret.clone()),
        ];
        if !config.scopes.is_empty() {
            form.push(("scope", config.scopes.join(" ")));
        }

        let response = self
            .client
            .post(&config.token_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Transport(TransportError::new(e)))?;

        let status = StatusCode::from_u16(response.status().as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Status { status, body });
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| Error::Transport(TransportError::new(e)))?;

        // Default to a conservative lifetime when the server omits expires_in.
        let lifetime = std::time::Duration::from_secs(token.expires_in.unwrap_or(300));
        *cached = Some(CachedToken {
            value: token.access_token.clone(),
            refresh_at: refresh_at(lifetime),
        });

        Ok(token.access_token)
    }
}

#[async_trait::async_trait]
impl Transport for ReqwestTransport {
    async fn execute(&self, request: TmfRequest) -> Result<TmfResponse> {
        let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|e| Error::Transport(TransportError::new(e)))?;

        let mut builder = self.client.request(method, &request.url);

        if !request.query.is_empty() {
            builder = builder.query(&request.query.into_iter().collect::<Vec<_>>());
        }
        for (name, value) in &self.default_headers {
            builder = builder.header(name, value);
        }
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(value) = self.authorization().await? {
            builder = builder.header(header::AUTHORIZATION, value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| Error::Transport(TransportError::new(e)))?;

        let status = StatusCode::from_u16(response.status().as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let headers = response.headers().clone();
        let body = response
            .bytes()
            .await
            .map_err(|e| Error::Transport(TransportError::new(e)))?;

        Ok(TmfResponse::new(status, headers, body))
    }
}

/// Builder for [`ReqwestTransport`].
#[derive(Debug, Default)]
pub struct ReqwestTransportBuilder {
    auth: Auth,
    default_headers: HeaderMap,
    timeout: Option<std::time::Duration>,
    user_agent: Option<String>,
    client: Option<reqwest::Client>,
}

impl ReqwestTransportBuilder {
    /// Sets the authentication scheme.
    #[must_use]
    pub fn auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }

    /// Adds a header sent with every request.
    #[must_use]
    pub fn header(mut self, name: header::HeaderName, value: HeaderValue) -> Self {
        self.default_headers.insert(name, value);
        self
    }

    /// Sets a per-request timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Overrides the `User-Agent` header.
    #[must_use]
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Supplies a pre-configured `reqwest` client, bypassing the other options
    /// that would otherwise build one (timeout, user agent).
    #[must_use]
    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Builds the transport.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the HTTP client cannot be constructed.
    pub fn build(self) -> Result<ReqwestTransport> {
        let client = if let Some(client) = self.client {
            client
        } else {
            let mut builder = reqwest::Client::builder().user_agent(
                self.user_agent
                    .unwrap_or_else(|| concat!("rutmf/", env!("CARGO_PKG_VERSION")).into()),
            );
            if let Some(timeout) = self.timeout {
                builder = builder.timeout(timeout);
            }
            builder
                .build()
                .map_err(|e| Error::Transport(TransportError::new(e)))?
        };

        Ok(ReqwestTransport {
            client,
            auth: self.auth,
            token: Arc::new(Mutex::new(None)),
            default_headers: self.default_headers,
        })
    }
}

/// Minimal standard base64, avoiding a dependency for one header.
fn base64_encode(input: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);

        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A transport is exactly the kind of value that reaches a log line, so its
    /// `Debug` must not be the thing that publishes a token.
    #[test]
    fn debug_output_carries_no_credentials() {
        const SECRET: &str = "s3cr3t-do-not-print";

        let cases = [
            Auth::Bearer(SECRET.to_owned()),
            Auth::Basic {
                username: "user".to_owned(),
                password: SECRET.to_owned(),
            },
            Auth::ClientCredentials(Box::new(
                ClientCredentials::new("https://idp.example/token", "client", SECRET)
                    .with_scopes(["catalog:read"]),
            )),
        ];

        for auth in cases {
            let rendered = format!("{auth:?}");
            assert!(!rendered.contains(SECRET), "Auth leaked: {rendered}");

            let transport = ReqwestTransport::builder()
                .auth(auth)
                .header(
                    header::AUTHORIZATION,
                    HeaderValue::from_static("Bearer s3cr3t-do-not-print"),
                )
                .build()
                .expect("a default client builds");
            let rendered = format!("{transport:?}");
            assert!(
                !rendered.contains(SECRET),
                "ReqwestTransport leaked: {rendered}"
            );
        }
    }

    /// A cached token must not reach a log line either — it is a live
    /// credential with the same power as the one that minted it.
    #[test]
    fn a_cached_token_is_not_printable() {
        let transport = ReqwestTransport::builder()
            .auth(Auth::None)
            .build()
            .expect("a default client builds");
        *transport.token.try_lock().expect("uncontended") = Some(CachedToken {
            value: "cached-s3cr3t".to_owned(),
            refresh_at: std::time::Instant::now(),
        });
        assert!(!format!("{transport:?}").contains("cached-s3cr3t"));
    }

    #[test]
    fn a_short_lived_token_is_still_cached_for_part_of_its_life() {
        use std::time::Duration;

        // `refresh_at` reads the clock itself, so the window each case is
        // checked against is the one bracketing that read.
        let margin_for = |lifetime: Duration| {
            let before = std::time::Instant::now();
            let at = refresh_at(lifetime);
            (at.saturating_duration_since(before), before.elapsed())
        };

        // A fixed 30-second margin against a 10-second token would leave a
        // cache entry that is already stale, so every API call fetches a token
        // first. Capping the margin at half the lifetime keeps it useful: the
        // token is still good for about five seconds.
        let (ahead, slack) = margin_for(Duration::from_secs(10));
        assert!(
            ahead >= Duration::from_secs(5) && ahead <= Duration::from_secs(5) + slack,
            "a 10-second token should refresh at about 5s, got {ahead:?}"
        );

        // A long-lived token keeps the full 30-second margin.
        let (ahead, slack) = margin_for(Duration::from_secs(3600));
        assert!(
            ahead >= Duration::from_secs(3570) && ahead <= Duration::from_secs(3570) + slack,
            "an hour-long token should refresh 30s early, got {ahead:?}"
        );

        // A zero lifetime must not panic or land in the future.
        assert!(refresh_at(Duration::ZERO) <= std::time::Instant::now());
    }

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("f"), "Zg==");
        assert_eq!(base64_encode("fo"), "Zm8=");
        assert_eq!(base64_encode("foo"), "Zm9v");
        assert_eq!(base64_encode("foob"), "Zm9vYg==");
        assert_eq!(base64_encode("fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode("foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode("user:pass"), "dXNlcjpwYXNz");
    }
}
