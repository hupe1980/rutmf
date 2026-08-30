//! The shared machinery every per-API client is built from.

use std::sync::Arc;

use bytes::Bytes;
use http::{HeaderValue, Method, StatusCode, header};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::core::{PatchBody, TmfError};

use super::error::{Error, Result};
use super::page::{Page, next_link};
use super::patch::Patch;
use super::query::Query;
use super::transport::{TmfRequest, TmfResponse, Transport};

/// A configured TM Forum API endpoint.
///
/// Wraps a [`Transport`] with a base URL and the request/response handling that
/// TMF630 prescribes: content negotiation, error-body parsing and pagination
/// headers. Per-API clients such as [`ProductCatalogClient`] delegate to this.
///
/// [`ProductCatalogClient`]: crate::api::tmf620::ProductCatalogClient
#[derive(Clone)]
pub struct TmfClient {
    transport: Arc<dyn Transport>,
    base_url: String,
}

impl std::fmt::Debug for TmfClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TmfClient")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl TmfClient {
    /// Creates a client for `base_url`, dispatching through `transport`.
    ///
    /// The base URL should include the API root, e.g.
    /// `https://host/tmf-api/productCatalogManagement/v5`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidBaseUrl`] if the URL is empty.
    pub fn new(base_url: impl Into<String>, transport: impl Transport + 'static) -> Result<Self> {
        let base_url = base_url.into();
        if base_url.trim().is_empty() {
            return Err(Error::InvalidBaseUrl("base URL must not be empty".into()));
        }
        Ok(Self {
            transport: Arc::new(transport),
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    /// The configured base URL, without a trailing slash.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Resolves a resource path against the base URL.
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// `GET {path}/{id}` returning a single resource.
    pub async fn get<T: DeserializeOwned>(&self, path: &str, id: &str, query: &Query) -> Result<T> {
        let mut request = TmfRequest::new(Method::GET, self.url(&format!("{path}/{id}")));
        request.query = query.to_params();
        let response = self.send(request).await?;
        decode(&response)
    }

    /// `GET` an absolute URL on this API's own origin, returning a resource.
    ///
    /// Used to follow an `href` that names a different API than this client is
    /// configured for — the catalog referring to party management, say. In a
    /// TM Forum deployment those sit at different *paths* on one host
    /// (`/tmf-api/productCatalogManagement/v5` beside
    /// `/tmf-api/partyManagement/v5`), which is why the common case needs no
    /// escape hatch.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CrossOrigin`] if the URL leaves this client's origin.
    /// An `href` is server-controlled and the transport attaches credentials to
    /// whatever URL it is handed, so following one to another host would hand a
    /// bearer token to whoever owns that host. Genuine federation across
    /// origins goes through [`get_cross_origin`](Self::get_cross_origin), where
    /// leaving the origin is the point and saying so is the caller's decision.
    pub async fn get_absolute<T: DeserializeOwned>(&self, url: &str, query: &Query) -> Result<T> {
        self.require_same_origin(url)?;
        self.get_cross_origin(url, query).await
    }

    /// `GET` an absolute URL **on any origin**, returning a resource.
    ///
    /// The unguarded counterpart of [`get_absolute`](Self::get_absolute), for a
    /// deployment that really does federate across hosts.
    ///
    /// The transport applies this client's credentials to the URL it is given,
    /// so calling this with a URL that came from a payload sends your token
    /// wherever that payload said. Use it with a URL you chose, or with one from
    /// a server you trust to name only hosts you would authenticate against.
    pub async fn get_cross_origin<T: DeserializeOwned>(
        &self,
        url: &str,
        query: &Query,
    ) -> Result<T> {
        let mut request = TmfRequest::new(Method::GET, url.to_owned());
        request.query = query.to_params();
        let response = self.send(request).await?;
        decode(&response)
    }

    /// Refuses a server-supplied URL that leaves this client's origin.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CrossOrigin`] when the origins differ, or when either
    /// URL cannot be read confidently — see [`same_origin`].
    pub fn require_same_origin(&self, url: &str) -> Result<()> {
        if same_origin(&self.base_url, url) {
            return Ok(());
        }
        Err(Error::CrossOrigin {
            url: url.to_owned(),
            base: self.base_url.clone(),
        })
    }

    /// `GET {path}` returning one page of resources plus what the server said
    /// about the rest.
    pub async fn list<T: DeserializeOwned>(&self, path: &str, query: &Query) -> Result<Page<T>> {
        let mut request = TmfRequest::new(Method::GET, self.url(path));
        request.query = query.to_params();
        let response = self.send(request).await?;
        Self::page_from(&response, query.offset_value().unwrap_or(0))
    }

    /// Reads a page and whatever the response said about the rest of it.
    fn page_from<T: DeserializeOwned>(response: &TmfResponse, offset: usize) -> Result<Page<T>> {
        Ok(Page {
            items: decode(response)?,
            total_count: response.total_count(),
            result_count: response.result_count(),
            next_link: response
                .headers
                .get(header::LINK)
                .and_then(|v| v.to_str().ok())
                .and_then(next_link),
            // TMF630 marks a slice of a larger match this way, and it is the
            // only "there is more" a server can send without computing a total.
            partial: response.status == StatusCode::PARTIAL_CONTENT,
            offset,
        })
    }

    /// `GET` an absolute URL returning one page of resources.
    ///
    /// Used to follow a `Link: …; rel="next"` header, whose target is the
    /// server's own and may carry a cursor rather than an offset.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CrossOrigin`] if the URL is not on the same origin as
    /// this client's base. A `Link` header is server-controlled and the
    /// transport attaches credentials to whatever it is given, so a next-page
    /// link that leaves the API is refused rather than followed.
    pub async fn list_absolute<T: DeserializeOwned>(&self, url: &str) -> Result<Page<T>> {
        self.require_same_origin(url)?;
        let response = self
            .send(TmfRequest::new(Method::GET, url.to_owned()))
            .await?;
        // The offset of a followed page is the server's business, not ours —
        // its cursor may not be an index at all. `PageStream` does not read it
        // once the server is leading.
        Self::page_from(&response, 0)
    }

    /// `POST {path}` creating a resource.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Accepted`] when the server answers `202` — it took the
    /// request but has not created anything yet.
    pub async fn create<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let mut request = TmfRequest::new(Method::POST, self.url(path));
        request.body = Some(encode(body)?);
        request.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let response = self.send(request).await?;
        decode(&response)
    }

    /// `PATCH {path}/{id}` updating a resource.
    ///
    /// [`Patch`] carries the body and the semantics together, so the
    /// `Content-Type` always describes what is actually being sent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Accepted`] when the server answers `202` — it took the
    /// request but has not applied it yet.
    pub async fn patch<'a, U: PatchBody + 'a, T: DeserializeOwned>(
        &self,
        path: &str,
        id: &str,
        body: impl Into<Patch<'a, U>>,
    ) -> Result<T> {
        let request = self.patch_request(path, id, &body.into())?;
        let response = self.send(request).await?;
        decode(&response)
    }

    /// Builds the `PATCH` request, body and content type paired.
    ///
    /// Shared with the conditional form so that adding `If-Match` cannot drift
    /// into sending a differently-shaped request from the unconditional one.
    pub(super) fn patch_request<U: PatchBody>(
        &self,
        path: &str,
        id: &str,
        body: &Patch<'_, U>,
    ) -> Result<TmfRequest> {
        let mut request = TmfRequest::new(Method::PATCH, self.url(&format!("{path}/{id}")));
        request.body = Some(Bytes::from(body.to_json().map_err(Error::Encode)?));
        request.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(body.content_type()),
        );
        Ok(request)
    }

    /// `DELETE {path}/{id}`.
    ///
    /// Succeeds on `204 No Content` and on `202 Accepted`, which conformant
    /// servers return when the deletion is asynchronous. Unlike a create or a
    /// patch, a delete has no resource to hand back either way, so the two are
    /// not worth distinguishing here.
    pub async fn delete(&self, path: &str, id: &str) -> Result<()> {
        let request = TmfRequest::new(Method::DELETE, self.url(&format!("{path}/{id}")));
        self.send(request).await.map(|_| ())
    }

    /// Executes a request, applying TMF error handling to the response.
    pub async fn send(&self, request: TmfRequest) -> Result<TmfResponse> {
        let response = self.dispatch(request).await?;
        if response.status.is_success() {
            return Ok(response);
        }
        Err(interpret_failure(&response))
    }

    /// Executes a request, treating `304 Not Modified` as an answer rather than
    /// a failure.
    ///
    /// `304` is a 3xx, so [`send`](Self::send) reports it as an error — which is
    /// right for every request that did not ask for it, because a client that
    /// sent no precondition and gets one back has been told something it cannot
    /// act on. A conditional read *did* ask, and for it a `304` is the whole
    /// point.
    pub(super) async fn send_conditional(&self, request: TmfRequest) -> Result<TmfResponse> {
        let response = self.dispatch(request).await?;
        if response.status.is_success() || response.status == StatusCode::NOT_MODIFIED {
            return Ok(response);
        }
        Err(interpret_failure(&response))
    }

    /// Hands a request to the transport, with the headers every TMF call needs.
    async fn dispatch(&self, mut request: TmfRequest) -> Result<TmfResponse> {
        request
            .headers
            .entry(header::ACCEPT)
            .or_insert_with(|| HeaderValue::from_static("application/json"));
        self.transport.execute(request).await
    }
}

/// Turns a non-2xx response into the richest error the body supports.
///
/// Every member of [`TmfError`] is optional, so *any* JSON object deserializes
/// into one; [`TmfError::is_populated`] is what distinguishes a real TMF630
/// error body from a gateway's own JSON, which belongs in [`Error::Status`]
/// where the raw text survives.
pub(crate) fn interpret_failure(response: &TmfResponse) -> Error {
    match serde_json::from_slice::<TmfError>(&response.body) {
        Ok(error) if error.is_populated() => Error::Api {
            status: response.status,
            error: Box::new(error),
        },
        _ => Error::Status {
            status: response.status,
            body: truncate(&String::from_utf8_lossy(&response.body)),
        },
    }
}

/// Decodes a JSON body, distinguishing "not finished" from "no content".
///
/// Every v5 `POST` and `PATCH` declares `202 Accepted` with an empty body
/// beside its synchronous answer, because a deployment may fulfil a write
/// asynchronously. Feeding that to serde produced `invalid type: null,
/// expected struct …`, which tells the caller nothing about what happened.
pub(super) fn decode<T: DeserializeOwned>(response: &TmfResponse) -> Result<T> {
    if response.body.is_empty() {
        if response.status == StatusCode::ACCEPTED {
            return Err(Error::Accepted {
                status: response.status,
                monitor: monitor_url(response),
            });
        }
        if response.status == StatusCode::NO_CONTENT {
            // `null` is the only JSON value a unit-like `T` can be built from.
            return serde_json::from_str("null").map_err(Error::Decode);
        }
    }
    serde_json::from_slice(&response.body).map_err(Error::Decode)
}

/// Where a `202` said to poll, if it said.
///
/// The v5 documents declare only `X-Total-Count` and `X-Result-Count`, and
/// name no header for this at all — so this reads the two HTTP-standard ones
/// and settles for `None` when neither is present.
fn monitor_url(response: &TmfResponse) -> Option<String> {
    [header::LOCATION, header::CONTENT_LOCATION]
        .iter()
        .find_map(|name| response.headers.get(name))
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn encode<B: Serialize>(body: &B) -> Result<Bytes> {
    serde_json::to_vec(body)
        .map(Bytes::from)
        .map_err(Error::Encode)
}

/// Whether two URLs share a scheme, host and port.
///
/// Deliberately a string comparison over the authority rather than a URL
/// parse: the crate has no URL dependency, and anything this cannot read
/// confidently is treated as a different origin. That is the safe direction to
/// fail in — a URL this cannot read is one whose destination it cannot vouch
/// for.
///
/// Public because the same question comes up in a hand-written [`Transport`],
/// and answering it inconsistently is how a guard gets bypassed.
///
/// ```
/// use rutmf::api::same_origin;
///
/// let base = "https://catalog.example/tmf-api/productCatalogManagement/v5";
/// assert!(same_origin(base, "https://catalog.example/tmf-api/partyManagement/v5/individual/7"));
/// assert!(!same_origin(base, "https://attacker.example/collect"));
/// assert!(!same_origin(base, "https://catalog.example:8443/other"), "a port is part of the origin");
/// ```
///
/// # The default port is the same origin
///
/// `https://host` and `https://host:443` are one origin (RFC 6454 §4), and a
/// deployment writes both — the base URL comes from configuration, the `href`
/// from whatever the server was told its own address is. Refusing that
/// difference would push callers to `resolve_cross_origin`, turning a spelling
/// into a reason to switch the guard off.
///
/// ```
/// # use rutmf::api::same_origin;
/// assert!(same_origin("https://host/tmf-api/x/v5", "https://host:443/tmf-api/y/v5/a/1"));
/// assert!(same_origin("http://host:80/x", "http://host/x"));
/// assert!(!same_origin("https://host:8443/x", "https://host/x"));
/// ```
///
/// [`Transport`]: super::Transport
#[must_use]
pub fn same_origin(base: &str, other: &str) -> bool {
    /// The scheme and authority of `url`, with an explicit default port dropped.
    ///
    /// Userinfo is deliberately *kept* as part of the authority. It is not part
    /// of an origin, but stripping it would make
    /// `https://catalog.example@attacker.example/` compare equal to
    /// `https://catalog.example/` — the credentials would go to
    /// `attacker.example`, which is the host after the `@`. Leaving it in makes
    /// such a URL simply not match, which is the answer that keeps the token at
    /// home.
    fn origin(url: &str) -> Option<(&str, &str)> {
        let (scheme, rest) = url.split_once("://")?;
        let authority = rest.split(['/', '?', '#']).next()?;
        if scheme.is_empty() || authority.is_empty() {
            return None;
        }
        // Only a trailing `:<digits>` is a port. An IPv6 literal is bracketed,
        // so its inner colons are before the `]` and never mistaken for one.
        let host = match authority.rsplit_once(':') {
            Some((host, port))
                if !host.is_empty()
                    && !port.is_empty()
                    && port.bytes().all(|b| b.is_ascii_digit())
                    && default_port(scheme) == Some(port) =>
            {
                host
            }
            _ => authority,
        };
        Some((scheme, host))
    }

    fn default_port(scheme: &str) -> Option<&'static str> {
        if scheme.eq_ignore_ascii_case("https") {
            Some("443")
        } else if scheme.eq_ignore_ascii_case("http") {
            Some("80")
        } else {
            None
        }
    }

    match (origin(base), origin(other)) {
        (Some((base_scheme, base_host)), Some((other_scheme, other_host))) => {
            base_scheme.eq_ignore_ascii_case(other_scheme)
                && base_host.eq_ignore_ascii_case(other_host)
        }
        _ => false,
    }
}

/// Keeps error messages readable when a server returns an HTML error page.
fn truncate(body: &str) -> String {
    const MAX: usize = 512;
    if body.len() <= MAX {
        return body.to_owned();
    }
    let mut end = MAX;
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… ({} bytes total)", &body[..end], body.len())
}

#[cfg(test)]
mod tests {
    use http::HeaderMap;

    use super::*;

    #[test]
    fn truncates_on_a_char_boundary() {
        let body = "ä".repeat(400); // 800 bytes
        let out = truncate(&body);
        assert!(out.ends_with("bytes total)"));
        assert!(out.is_char_boundary(0));
    }

    #[test]
    fn rejects_an_empty_base_url() {
        struct Never;
        #[async_trait::async_trait]
        impl Transport for Never {
            async fn execute(&self, _: TmfRequest) -> Result<TmfResponse> {
                unreachable!()
            }
        }
        assert!(matches!(
            TmfClient::new("  ", Never),
            Err(Error::InvalidBaseUrl(_))
        ));
    }

    fn failure(body: &'static str) -> Error {
        interpret_failure(&TmfResponse::new(
            StatusCode::BAD_GATEWAY,
            HeaderMap::new(),
            Bytes::from_static(body.as_bytes()),
        ))
    }

    #[test]
    fn a_pagination_link_may_not_leave_the_origin() {
        let base = "https://catalog.example/tmf-api/productCatalogManagement/v5";
        assert!(same_origin(
            base,
            "https://catalog.example/tmf-api/x?cursor=2"
        ));
        assert!(same_origin(base, "HTTPS://Catalog.Example/other"));

        assert!(!same_origin(base, "https://attacker.example/steal"));
        assert!(
            !same_origin(base, "https://catalog.example:8443/other"),
            "a port is part of the origin"
        );
        assert!(
            !same_origin(base, "http://catalog.example/other"),
            "so is the scheme"
        );
        assert!(
            !same_origin(base, "/relative/path"),
            "unreadable is not same-origin"
        );
    }

    #[test]
    fn an_explicit_default_port_is_the_same_origin() {
        // RFC 6454 §4. A deployment's base URL comes from configuration and its
        // `href`s from whatever the server thinks its address is, so the two
        // disagreeing on `:443` is ordinary — and refusing it would push callers
        // to `resolve_cross_origin`, switching the guard off over a spelling.
        assert!(same_origin(
            "https://host/tmf-api/x/v5",
            "https://host:443/tmf-api/y/v5/a/1"
        ));
        assert!(same_origin("https://host:443/x", "https://host/x"));
        assert!(same_origin("http://host:80/x", "http://host/x"));

        // A non-default port is still part of the origin, and a default port
        // for the *other* scheme is not this scheme's default.
        assert!(!same_origin("https://host:8443/x", "https://host/x"));
        assert!(!same_origin("https://host:80/x", "https://host/x"));
        assert!(!same_origin("http://host:443/x", "http://host/x"));
    }

    #[test]
    fn userinfo_cannot_disguise_another_host() {
        // The host is what follows the `@`, so a token sent to
        // `https://catalog.example@attacker.example/` reaches `attacker.example`.
        // Keeping userinfo in the compared authority makes that not match.
        let base = "https://catalog.example/tmf-api/x/v5";
        assert!(!same_origin(
            base,
            "https://catalog.example@attacker.example/steal"
        ));
        assert!(!same_origin(
            base,
            "https://attacker.example@catalog.example/x"
        ));
    }

    #[test]
    fn an_ipv6_literal_keeps_its_brackets_and_its_port() {
        assert!(same_origin("https://[::1]/x", "https://[::1]:443/y"));
        assert!(!same_origin("https://[::1]/x", "https://[::2]/y"));
        assert!(!same_origin("https://[::1]/x", "https://[::1]:8443/y"));
    }

    #[test]
    fn a_tmf_error_body_becomes_a_typed_error() {
        let error = failure(r#"{"code":"50201","reason":"upstream down"}"#);
        assert_eq!(
            error.tmf_error().and_then(|e| e.code.as_deref()),
            Some("50201")
        );
    }

    #[test]
    fn an_unrelated_json_body_keeps_its_raw_text() {
        // Without the `is_populated` check this parses into an all-`None`
        // `TmfError` and the gateway's own message is thrown away.
        let error = failure(r#"{"message":"gateway timeout","upstream":"catalog"}"#);
        match error {
            Error::Status { body, .. } => assert!(body.contains("gateway timeout")),
            other => panic!("expected a raw status error, got {other:?}"),
        }
    }
}
