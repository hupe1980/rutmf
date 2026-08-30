//! An `axum` adapter, so a [`TmfHandler`] can be served over HTTP.
//!
//! This is the server-side counterpart of [`ReqwestTransport`]: the layer is
//! framework-agnostic, and one ready-made binding ships with it.
//!
//! [`ReqwestTransport`]: crate::api::ReqwestTransport

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::response::Response;
use axum::routing::any;
use http::{StatusCode, Uri};

use crate::api::{TmfRequest, TmfResponse};

use super::handler::TmfHandler;
use super::store::ResourceStore;

/// A [`Router`] serving `handler` on every path beneath its mount point.
///
/// One route, not one per operation: [`TmfHandler`] already routes a TMF URL to
/// a collection and an id, and duplicating that in a route table would be a
/// second thing to keep in step.
///
/// Mount it wherever the API's base URL says it lives:
///
/// ```no_run
/// use rutmf::server::{MemoryStore, TmfHandler, router};
///
/// # async fn serve() -> Result<(), Box<dyn std::error::Error>> {
/// let handler = TmfHandler::new(
///     "https://mycsp.com/tmf-api/productCatalogManagement/v5",
///     MemoryStore::new(),
/// );
///
/// let app = axum::Router::new().nest(
///     "/tmf-api/productCatalogManagement/v5",
///     router(handler),
/// );
///
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
/// axum::serve(listener, app).await?;
/// # Ok(())
/// # }
/// ```
///
/// The handler's `base_url` is what ends up in each resource's `href` and in
/// the `Location` of a `201`, so it should be the URL clients reach the API on
/// — not the address it binds to. Those differ behind any proxy.
pub fn router<S>(handler: TmfHandler<S>) -> Router
where
    S: ResourceStore + 'static,
{
    Router::new()
        .route("/{*path}", any(serve))
        .route("/", any(serve))
        .with_state(Arc::new(handler))
}

async fn serve<S>(State(handler): State<Arc<TmfHandler<S>>>, request: Request) -> Response
where
    S: ResourceStore + 'static,
{
    // A body larger than this is not a TMF resource, and reading it unbounded
    // would let one request exhaust the server.
    const MAX_BODY: usize = 8 * 1024 * 1024;

    let (parts, body) = request.into_parts();
    let Ok(body) = axum::body::to_bytes(body, MAX_BODY).await else {
        return into_response(&super::handler::error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "41301",
            "Request body is too large",
        ));
    };

    let request = TmfRequest {
        method: parts.method.clone(),
        url: absolute_url(handler.base_url(), &parts.uri),
        query: parse_query(parts.uri.query()),
        headers: parts.headers,
        body: (!body.is_empty()).then_some(body),
    };

    into_response(&handler.handle(&request).await)
}

/// Rebuilds the absolute URL the handler routes on.
///
/// `axum` hands over a path relative to the mount point, and the handler
/// expects the API's own base URL in front of it — which is exactly the URL the
/// client asked for, reconstructed from the side that knows it.
fn absolute_url(base_url: &str, uri: &Uri) -> String {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        base_url.to_owned()
    } else {
        format!("{base_url}/{path}")
    }
}

/// Reads a query string into the parameter map the handler filters on.
///
/// # A repeated parameter widens rather than replaces
///
/// TMF630 spells alternatives as a comma-separated value list —
/// `state=held,pending` — and defines no meaning for the same parameter
/// appearing twice. Clients send it anyway, because `?state=held&state=pending`
/// is what most HTTP libraries produce from a list, and it plainly means the
/// same thing.
///
/// Keeping the last occurrence would answer that request with the resources in
/// `pending` alone: a narrower result than either value asked for, with nothing
/// to indicate that half the query was dropped. So repeats are joined into the
/// comma list they stand for, which
/// [`matches_filters`](super::matches_filters) already reads as "any of" —
/// making the two spellings the same query, which is what the sender meant.
///
/// The reserved parameters are not filters and have no "any of" reading, so for
/// them the last occurrence wins: two `limit`s are a malformed request either
/// way, and `limit=20,50` would parse as no limit at all.
fn parse_query(query: Option<&str>) -> BTreeMap<String, String> {
    let Some(query) = query else {
        return BTreeMap::new();
    };

    let mut params: BTreeMap<String, String> = BTreeMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = match pair.split_once('=') {
            Some((key, value)) => (decode(key), decode(value)),
            None => (decode(pair), String::new()),
        };
        match params.entry(key) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(value);
            }
            std::collections::btree_map::Entry::Occupied(mut slot) => {
                if super::semantics::is_reserved(slot.key()) {
                    slot.insert(value);
                } else {
                    let widened = format!("{},{value}", slot.get());
                    slot.insert(widened);
                }
            }
        }
    }
    params
}

/// Percent-decoding, plus the `+`-as-space convention of a query string.
fn decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let decoded = std::str::from_utf8(&bytes[index + 1..index + 3])
                    .ok()
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok());
                if let Some(byte) = decoded {
                    out.push(byte);
                    index += 3;
                } else {
                    out.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

fn into_response(response: &TmfResponse) -> Response {
    let mut builder = Response::builder().status(response.status);
    if let Some(headers) = builder.headers_mut() {
        headers.clone_from(&response.headers);
    }
    builder
        .body(Body::from(Bytes::clone(&response.body)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .expect("an empty body is always a valid response")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_string_is_decoded_including_plus_as_space() {
        let parsed = parse_query(Some("name=Basic+Firewall&note=a%20b&flag"));

        assert_eq!(parsed["name"], "Basic Firewall");
        assert_eq!(parsed["note"], "a b");
        assert_eq!(parsed["flag"], "");
    }

    #[test]
    fn a_malformed_escape_survives_rather_than_failing_the_request() {
        assert_eq!(parse_query(Some("a=100%"))["a"], "100%");
        assert_eq!(parse_query(Some("a=%zz"))["a"], "%zz");
    }

    #[test]
    fn an_operator_suffix_survives_decoding() {
        // `orderDate.gte=…` is a filter the handler must still recognise.
        let parsed = parse_query(Some("orderDate.gte=2026-01-01T00%3A00%3A00Z"));
        assert_eq!(parsed["orderDate.gte"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn the_mount_point_is_put_back_in_front_of_the_path() {
        let base = "https://mycsp.com/tmf-api/productCatalogManagement/v5";
        assert_eq!(
            absolute_url(base, &"/productOffering/7655".parse().unwrap()),
            format!("{base}/productOffering/7655")
        );
        assert_eq!(absolute_url(base, &"/".parse().unwrap()), base);
    }

    #[test]
    fn a_repeated_filter_widens_into_the_comma_list_it_stands_for() {
        // `?state=held&state=pending` is what most HTTP libraries produce from
        // a list. Keeping only `pending` would answer with a narrower result
        // than either value asked for and say nothing about it.
        assert_eq!(
            parse_query(Some("state=held&state=pending"))["state"],
            "held,pending"
        );
        // And it agrees with the comma spelling, so the two are one query.
        assert_eq!(
            parse_query(Some("state=held,pending"))["state"],
            parse_query(Some("state=held&state=pending"))["state"]
        );
    }

    #[test]
    fn a_repeated_reserved_parameter_keeps_the_last() {
        // `limit=20,50` parses as no limit at all, which is the one answer
        // worse than picking one of the two.
        assert_eq!(parse_query(Some("limit=20&limit=50"))["limit"], "50");
        assert_eq!(parse_query(Some("fields=id&fields=name"))["fields"], "name");
    }
}
