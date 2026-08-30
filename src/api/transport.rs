//! The transport seam.

use std::collections::BTreeMap;
use std::fmt;

use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode};

use super::error::Error;

/// A request produced by a client, ready for a [`Transport`] to execute.
///
/// The URL is already absolute: the client resolves it against its base URL
/// before handing the request over, so a transport never needs to know where
/// the API lives.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TmfRequest {
    /// HTTP method.
    pub method: Method,
    /// Absolute request URL.
    ///
    /// Normally carries no query string — the parameters a client builds live
    /// in [`query`](Self::query), so a transport can log or rewrite them
    /// without re-parsing a URL.
    ///
    /// The exception is a URL the *server* composed and the client is
    /// following whole: a `Link: rel="next"` target may carry an opaque cursor
    /// in its own query string. A transport must therefore **append**
    /// [`query`](Self::query) to whatever this already has rather than
    /// replacing it. `reqwest`'s `RequestBuilder::query` appends, which is why
    /// the bundled transport is correct here by construction; a hand-written
    /// one has to be deliberate about it.
    pub url: String,
    /// Query parameters, in a stable order so requests are reproducible.
    ///
    /// Appended to any query string [`url`](Self::url) already carries.
    pub query: BTreeMap<String, String>,
    /// Request headers, including `Content-Type` where a body is present.
    pub headers: HeaderMap,
    /// Serialised request body.
    pub body: Option<Bytes>,
}

impl TmfRequest {
    /// Starts a request for the absolute `url` using `method`.
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            query: BTreeMap::new(),
            headers: HeaderMap::new(),
            body: None,
        }
    }
}

/// A response handed back by a [`Transport`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TmfResponse {
    /// HTTP status code.
    pub status: StatusCode,
    /// Response headers, including the TMF pagination counters.
    pub headers: HeaderMap,
    /// Response body.
    pub body: Bytes,
}

impl TmfResponse {
    /// Constructs a response.
    #[must_use]
    pub fn new(status: StatusCode, headers: HeaderMap, body: Bytes) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    /// Reads a header as a `usize`, ignoring malformed values.
    #[must_use]
    pub fn header_count(&self, name: &str) -> Option<usize> {
        self.headers.get(name)?.to_str().ok()?.parse().ok()
    }

    /// The `X-Total-Count` header: how many resources match the query overall.
    #[must_use]
    pub fn total_count(&self) -> Option<usize> {
        self.header_count("x-total-count")
    }

    /// The `X-Result-Count` header: how many resources this response carries.
    #[must_use]
    pub fn result_count(&self) -> Option<usize> {
        self.header_count("x-result-count")
    }
}

/// Executes [`TmfRequest`]s.
///
/// Implement this to run TMF calls over your own HTTP stack, or to stub them in
/// tests. The `transport-reqwest` feature provides a ready-made implementation.
///
/// Clients in this crate never speak HTTP directly — they build a
/// [`TmfRequest`] and hand it to a `Transport`. That keeps `reqwest` (and its
/// TLS stack) out of the dependency graph unless you ask for it, makes clients
/// testable without a socket, and leaves the door open for wasm or a `tower`
/// stack.
///
/// # Composing behaviour
///
/// A `Transport` wraps another one, so cross-cutting concerns are ordinary
/// types rather than configuration. [`RetryTransport`](super::RetryTransport)
/// is one; correlation identifiers and tracing are the two most telco
/// integrations reach for next, and both are a dozen lines:
///
/// ```
/// use rutmf::api::{Result, TmfRequest, TmfResponse, Transport};
///
/// /// Stamps every request with a correlation id and logs the outcome.
/// struct Traced<T> {
///     inner: T,
///     service: &'static str,
/// }
///
/// #[async_trait::async_trait]
/// impl<T: Transport> Transport for Traced<T> {
///     async fn execute(&self, mut request: TmfRequest) -> Result<TmfResponse> {
///         let correlation = format!("{}-{}", self.service, request.url.len());
///         request.headers.insert(
///             "x-correlation-id",
///             http::HeaderValue::from_str(&correlation).expect("ascii"),
///         );
///
///         let outcome = self.inner.execute(request).await;
///         match &outcome {
///             Ok(response) => eprintln!("{correlation}: {}", response.status),
///             Err(error) => eprintln!("{correlation}: {error}"),
///         }
///         outcome
///     }
/// }
/// ```
///
/// Stack them in whichever order the behaviour needs — a
/// `Traced<RetryTransport<_>>` logs one line per call, a
/// `RetryTransport<Traced<_>>` logs one per attempt.
///
/// # Credentials and untrusted URLs
///
/// A transport applies its credentials to **whatever URL it is handed**, so a
/// client never hands it one that came from a server payload without checking
/// the origin first — see [`same_origin`](super::same_origin). An
/// implementation that resolves or rewrites URLs itself takes on that same
/// obligation.
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Executes one request.
    ///
    /// Implementations return [`Error::Transport`] for connection-level
    /// failures; interpreting HTTP status codes is the client's job.
    async fn execute(&self, request: TmfRequest) -> Result<TmfResponse, Error>;
}

#[async_trait::async_trait]
impl<T: Transport + ?Sized> Transport for &T {
    async fn execute(&self, request: TmfRequest) -> Result<TmfResponse, Error> {
        (**self).execute(request).await
    }
}

#[async_trait::async_trait]
impl<T: Transport + ?Sized> Transport for std::sync::Arc<T> {
    async fn execute(&self, request: TmfRequest) -> Result<TmfResponse, Error> {
        (**self).execute(request).await
    }
}

/// A transport error from an underlying HTTP stack.
#[derive(Debug)]
pub struct TransportError(Box<dyn std::error::Error + Send + Sync>);

impl TransportError {
    /// Wraps an arbitrary error from a transport implementation.
    pub fn new(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self(error.into())
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}
