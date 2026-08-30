//! Turning a [`ResourceStore`] into a TMF630-conformant HTTP surface.

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use serde_json::Value;

use crate::api::{TmfRequest, TmfResponse};

use crate::core::EventKind;

use super::notify::{
    HUB_COLLECTION, Listener, change_event, event_type_for, matching_listeners, state_change_kind,
};
use super::semantics::{apply_json_patch, apply_merge_patch, project_fields};
use super::store::{Replaced, ResourceStore, Selection, StoreError};

/// Serves a [`ResourceStore`] over the TMF630 collection semantics.
///
/// The handler owns everything between the socket and the store: routing a URL
/// to a collection and an id, reading the query into a [`Selection`], projecting
/// `fields=`, setting the count headers, choosing `200` or `206`, applying the
/// right kind of `PATCH`, and rendering failures as TMF630 error bodies.
///
/// It is framework-agnostic. [`handle`](Self::handle) takes a [`TmfRequest`] and
/// returns a [`TmfResponse`] — the same pair the client side uses, so an
/// adapter for any HTTP server is a small function. One for `axum` ships behind
/// the `server-axum` feature; [`MockTmfServer`] is another, wiring the handler
/// straight into the client's [`Transport`] with no socket at all.
///
/// [`MockTmfServer`]: crate::mock::MockTmfServer
/// [`Transport`]: crate::api::Transport
///
/// ```
/// use rutmf::server::{MemoryStore, TmfHandler};
///
/// let handler = TmfHandler::new(
///     "https://mycsp.com/tmf-api/productCatalogManagement/v5",
///     MemoryStore::new(),
/// );
/// assert_eq!(
///     handler.base_url(),
///     "https://mycsp.com/tmf-api/productCatalogManagement/v5"
/// );
/// ```
#[derive(Clone)]
pub struct TmfHandler<S> {
    base_url: String,
    store: S,
    ids: std::sync::Arc<dyn IdGenerator>,
    /// `None` until a notifier is configured, so a deployment that does not
    /// use notifications never pays a store read per write.
    notifier: Option<std::sync::Arc<dyn Notifier>>,
    /// `None` means an unbounded `GET` on a collection returns everything the
    /// filters matched — see [`TmfHandler::with_max_page_size`].
    max_page_size: Option<usize>,
}

impl<S: std::fmt::Debug> std::fmt::Debug for TmfHandler<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TmfHandler")
            .field("base_url", &self.base_url)
            .field("store", &self.store)
            .finish_non_exhaustive()
    }
}

impl<S> TmfHandler<S> {
    /// Serves `store` as the API rooted at `base_url`.
    ///
    /// The base URL is what the handler stamps into the `href` of a created
    /// resource and into the `Location` header, so it should be the URL clients
    /// reach the API on, not a local bind address.
    pub fn new(base_url: impl Into<String>, store: S) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            store,
            ids: std::sync::Arc::new(RandomId::default()),
            notifier: None,
            max_page_size: None,
        }
    }

    /// Bounds how many resources one `GET` on a collection may return.
    ///
    /// Without it, a request that names no `limit` returns everything the
    /// filters matched, so the size of a response is chosen by whoever sends the
    /// request.
    ///
    /// The cap lowers a `limit` larger than the maximum and supplies one where
    /// the request named none, so a client that pages properly is unaffected.
    /// `X-Total-Count` still reports the full match, making a capped response a
    /// `206`.
    ///
    /// Off by default: TMF630 permits a maximum without naming one, and turning
    /// this on silently would change what a working deployment returns.
    ///
    /// ```
    /// use rutmf::server::{MemoryStore, TmfHandler};
    ///
    /// let handler = TmfHandler::new("https://host/tmf-api/x/v5", MemoryStore::new())
    ///     .with_max_page_size(100);
    /// # let _ = handler;
    /// ```
    #[must_use]
    pub fn with_max_page_size(mut self, max: usize) -> Self {
        self.max_page_size = Some(max);
        self
    }

    /// Assigns server-chosen identifiers with `generator` instead of
    /// [`RandomId`].
    ///
    /// See [`IdGenerator`] for why this is a seam rather than a fixed policy.
    #[must_use]
    pub fn with_id_generator(mut self, generator: impl IdGenerator + 'static) -> Self {
        self.ids = std::sync::Arc::new(generator);
        self
    }

    /// Delivers change notifications through `notifier`.
    ///
    /// Without one, the handler still *serves* `/hub` — a subscription is
    /// stored and read back like any other resource — but nothing is ever
    /// delivered. See [`Notifier`] for what the handler works out on your
    /// behalf and what it leaves to you.
    #[must_use]
    pub fn with_notifier(mut self, notifier: impl Notifier + 'static) -> Self {
        self.notifier = Some(std::sync::Arc::new(notifier));
        self
    }

    /// Re-roots the API at `base_url`, for when the URL is only known after the
    /// handler is assembled — once a port is bound, say.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        base_url
            .into()
            .trim_end_matches('/')
            .clone_into(&mut self.base_url);
        self
    }

    /// The URL this API is rooted at.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The store behind the handler.
    #[must_use]
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The store behind the handler, mutably.
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }
}

impl<S: ResourceStore> TmfHandler<S> {
    /// Notifies every subscription that asked for this change.
    ///
    /// Reads the `hub` collection, builds the `{Resource}{Kind}Event` envelope,
    /// keeps the subscriptions whose `query` the event satisfies, and hands each
    /// to the [`Notifier`]. A failure to read the hubs is swallowed: a
    /// notification that cannot be sent must not turn a successful write into an
    /// error the client sees.
    ///
    /// Public because a store that changes a resource *outside* a request —
    /// a fulfilment worker moving an order to `completed` — owes the same
    /// notification, and should not have to rebuild the envelope to send it.
    pub async fn notify(&self, collection: &str, kind: EventKind, resource: &Value) {
        // Reading the subscriptions costs a store round-trip, so a deployment
        // that configured no notifier does not pay one on every write.
        let Some(notifier) = &self.notifier else {
            return;
        };
        // `hub` is a collection like any other, so a server that does not serve
        // one has no subscriptions and nothing to do.
        let Ok(hubs) = self.store.list(HUB_COLLECTION, &Selection::default()).await else {
            return;
        };
        if hubs.items.is_empty() {
            return;
        }

        let event = change_event(collection, kind, resource, &self.ids.next_id());
        let event_type = event_type_for(collection, kind);
        for listener in matching_listeners(&hubs.items, &event) {
            notifier.notify(&listener, &event_type, &event).await;
        }
    }

    /// Whether a change to `collection` is itself notifiable.
    ///
    /// Registering a subscription is not a domain event, so `POST /hub` does not
    /// raise a `HubCreateEvent` — which would be delivered to the subscription
    /// that had just been created, and to every other one.
    fn is_notifiable(collection: &str) -> bool {
        collection != HUB_COLLECTION
    }
}

impl<S: ResourceStore> TmfHandler<S> {
    /// Answers one request.
    ///
    /// Never fails: every outcome, including a store error, is a response. That
    /// is deliberate — an HTTP server has to say *something*, and deciding what
    /// is this type's job rather than the caller's.
    pub async fn handle(&self, request: &TmfRequest) -> TmfResponse {
        let Some((collection, id)) = route(&self.base_url, &request.url) else {
            return error_response(
                StatusCode::NOT_FOUND,
                "40401",
                &format!("No TM Forum resource at {}", request.url),
            );
        };

        if !self.store.has_collection(&collection).await {
            return error_response(
                StatusCode::NOT_FOUND,
                "40401",
                &format!("This API does not serve a {collection} collection"),
            );
        }

        match (request.method.clone(), id) {
            (Method::GET, None) => self.list(&collection, request).await,
            (Method::GET, Some(id)) => self.get(&collection, &id, request).await,
            (Method::POST, None) => self.create(&collection, request).await,
            (Method::PATCH, Some(id)) => self.patch(&collection, &id, request).await,
            (Method::DELETE, Some(id)) => self.delete(&collection, &id, request).await,
            (method, _) => error_response(
                StatusCode::METHOD_NOT_ALLOWED,
                "40501",
                &format!("{method} is not allowed on this path"),
            ),
        }
    }

    async fn list(&self, collection: &str, request: &TmfRequest) -> TmfResponse {
        // TMF621 and TMF639 declare a `filter` parameter carrying a JSONPath
        // expression — a different mechanism from the attribute filtering every
        // other collection uses, and one this handler does not implement.
        // Ignoring it would answer a request to narrow a collection with the
        // whole collection, which is the wrong way to be wrong.
        if request.query.contains_key("filter") {
            return error_response(
                StatusCode::BAD_REQUEST,
                "40001",
                "JSONPath `filter` is not supported by this server; filter by \
                 attribute name instead",
            );
        }
        let selection = match Selection::from_query(&request.query) {
            Ok(selection) => match self.max_page_size {
                Some(max) => selection.capped_at(max),
                None => selection,
            },
            // A `limit` the server cannot read must not become "no limit": that
            // answers a request for one page with the whole collection, and
            // answers it with `200`, so the client cannot tell.
            Err(reason) => return error_response(StatusCode::BAD_REQUEST, "40001", &reason),
        };
        let matched = match self.store.list(collection, &selection).await {
            Ok(matched) => matched,
            Err(error) => return store_error_response(&error),
        };

        let fields = request.query.get("fields").map(String::as_str);
        let items: Vec<Value> = matched
            .items
            .iter()
            .map(|item| project_fields(item, fields))
            .collect();

        let mut headers = HeaderMap::new();
        insert_count(&mut headers, "x-total-count", matched.total);
        insert_count(&mut headers, "x-result-count", items.len());

        // TMF630 marks a partial collection with 206, which is what tells a
        // client the count headers are worth reading.
        let status = if selection.offset + items.len() < matched.total {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        };
        json_response(status, headers, &Value::Array(items))
    }

    async fn get(&self, collection: &str, id: &str, request: &TmfRequest) -> TmfResponse {
        match self.store.get(collection, id).await {
            Ok(Some(found)) => {
                // The tag identifies the *stored* resource, so it stays valid
                // when `fields=` narrows what is sent back.
                let tag = entity_tag(&found);
                let mut headers = HeaderMap::new();
                if let Ok(value) = HeaderValue::from_str(&tag) {
                    headers.insert(header::ETAG, value);
                }

                // RFC 9110 §13.1.2. A client that already holds this version
                // says so, and gets told to keep it. TMF payloads are large —
                // a catalog offering with its prices and characteristics runs
                // to kilobytes — and a polling integration re-reads the same
                // resources on every cycle, so this is the difference between
                // a heartbeat and a load problem.
                if if_none_match(request).is_some_and(|expected| expected.matches(&tag)) {
                    return TmfResponse::new(StatusCode::NOT_MODIFIED, headers, Bytes::new());
                }

                json_response(
                    StatusCode::OK,
                    headers,
                    &project_fields(&found, request.query.get("fields").map(String::as_str)),
                )
            }
            Ok(None) => not_found(collection, id),
            Err(error) => store_error_response(&error),
        }
    }

    async fn create(&self, collection: &str, request: &TmfRequest) -> TmfResponse {
        let mut resource = match json_body(request) {
            Ok(body) => body,
            Err(reason) => return error_response(StatusCode::BAD_REQUEST, "40001", &reason),
        };

        // A conformant server assigns id and href when the client omits them,
        // and the href is absolute so a `Ref` resolves against it.
        let Some(object) = resource.as_object_mut() else {
            return error_response(
                StatusCode::BAD_REQUEST,
                "40001",
                "A resource must be a JSON object",
            );
        };
        let id = match object.get("id").and_then(Value::as_str) {
            Some(id) => id.to_owned(),
            None => self.ids.next_id(),
        };
        object.insert("id".to_owned(), Value::String(id.clone()));
        let href = format!("{}/{collection}/{id}", self.base_url);
        object
            .entry("href")
            .or_insert_with(|| Value::String(href.clone()));

        match self.store.create(collection, resource).await {
            Ok(created) => {
                if Self::is_notifiable(collection) {
                    self.notify(collection, EventKind::Create, &created).await;
                }
                // RFC 9110: a 201 should say where the new resource lives. The
                // vendored v5 documents declare exactly two response headers —
                // `X-Total-Count` and `X-Result-Count`, both on collections —
                // and no `Location`, so this is the HTTP rule rather than a TMF
                // one.
                //
                // It is read back off the stored resource rather than reused
                // from the value composed above, because either end may have
                // changed it: a client may send its own `href`, and a store may
                // normalise one. Sending a `Location` that disagrees with the
                // `href` in the body would name a resource that is not the one
                // just created.
                let mut headers = HeaderMap::new();
                let location = created
                    .get("href")
                    .and_then(Value::as_str)
                    .unwrap_or(href.as_str());
                if let Ok(value) = HeaderValue::from_str(location) {
                    headers.insert(header::LOCATION, value);
                }
                // A create is a change, so the caller can hold the tag it
                // answers with and `PATCH` conditionally without re-reading.
                insert_etag(&mut headers, &created);
                json_response(StatusCode::CREATED, headers, &created)
            }
            Err(error) => store_error_response(&error),
        }
    }

    async fn patch(&self, collection: &str, id: &str, request: &TmfRequest) -> TmfResponse {
        let patch = match json_body(request) {
            Ok(body) => body,
            Err(reason) => return error_response(StatusCode::BAD_REQUEST, "40001", &reason),
        };

        let mut target = match self.store.get(collection, id).await {
            Ok(Some(found)) => found,
            Ok(None) => return not_found(collection, id),
            Err(error) => return store_error_response(&error),
        };

        // RFC 9110 §13.1.1. A `PATCH` is read-modify-write, so two clients
        // editing different members of one resource can each overwrite the
        // other's change with no error anywhere. `If-Match` is how HTTP says
        // "only if it still looks like what I read".
        //
        // The tag of what was just read is kept, because rejecting here is only
        // half the job: the write below has to be conditional on the resource
        // still being *this* one, or the race reopens between the two.
        let read_tag = entity_tag(&target);
        let before = target.clone();
        let conditional = if_match(request);
        if let Some(expected) = &conditional
            && !expected.matches(&read_tag)
        {
            return precondition_failed();
        }

        let content_type = request
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/json");

        // Four content types, two behaviours. `application/json` and
        // `application/merge-patch+json` are both RFC 7386 merges; the two
        // `json-patch` families are RFC 6902 operation lists.
        if content_type.starts_with("application/json-patch") {
            if let Err(reason) = apply_json_patch(&mut target, &patch) {
                return error_response(StatusCode::UNPROCESSABLE_ENTITY, "42201", &reason);
            }
        } else {
            apply_merge_patch(&mut target, &patch);
        }

        // `id` is the address, not a member a patch may move.
        if let Some(object) = target.as_object_mut() {
            object.insert("id".to_owned(), Value::String(id.to_owned()));
        }

        // Unconditional when the client sent no precondition — HTTP allows a
        // bare `PATCH` to clobber, and turning concurrent edits into `412`s
        // nobody asked for would be the wrong kind of strict.
        let written = if conditional.is_some() {
            self.store
                .replace_if_unchanged(collection, id, target, &read_tag)
                .await
        } else {
            self.store
                .replace(collection, id, target)
                .await
                .map(|found| found.map_or(Replaced::Missing, Replaced::Updated))
        };

        match written {
            Ok(Replaced::Updated(updated)) => {
                if Self::is_notifiable(collection) {
                    // TMF630 distinguishes a lifecycle move from an ordinary
                    // edit, and a client subscribes to them separately. The
                    // handler can tell which this was: it holds the resource
                    // before and after.
                    let kind = change_kind(collection, &before, &updated);
                    self.notify(collection, kind, &updated).await;
                }
                let mut headers = HeaderMap::new();
                insert_etag(&mut headers, &updated);
                json_response(StatusCode::OK, headers, &updated)
            }
            Ok(Replaced::Missing) => not_found(collection, id),
            Ok(Replaced::Stale) => precondition_failed(),
            Err(error) => store_error_response(&error),
        }
    }

    async fn delete(&self, collection: &str, id: &str, request: &TmfRequest) -> TmfResponse {
        let Some(expected) = if_match(request) else {
            return match self.store.delete(collection, id).await {
                Ok(true) => {
                    if Self::is_notifiable(collection) {
                        // A delete event names what is gone, and by then only
                        // the address is left to name it with.
                        let gone = serde_json::json!({"id": id});
                        self.notify(collection, EventKind::Delete, &gone).await;
                    }
                    TmfResponse::new(StatusCode::NO_CONTENT, HeaderMap::new(), Bytes::new())
                }
                Ok(false) => not_found(collection, id),
                Err(error) => store_error_response(&error),
            };
        };

        // The tag has to be read and the removal has to happen as one step, or
        // a resource edited between the two is deleted on the strength of a
        // precondition that no longer holds.
        let current = match self.store.get(collection, id).await {
            Ok(Some(current)) => current,
            Ok(None) => return not_found(collection, id),
            Err(error) => return store_error_response(&error),
        };
        let tag = entity_tag(&current);
        if !expected.matches(&tag) {
            return precondition_failed();
        }

        match self.store.delete_if_unchanged(collection, id, &tag).await {
            Ok(Replaced::Updated(removed)) => {
                if Self::is_notifiable(collection) {
                    self.notify(collection, EventKind::Delete, &removed).await;
                }
                TmfResponse::new(StatusCode::NO_CONTENT, HeaderMap::new(), Bytes::new())
            }
            Ok(Replaced::Missing) => not_found(collection, id),
            Ok(Replaced::Stale) => precondition_failed(),
            Err(error) => store_error_response(&error),
        }
    }
}

/// Splits a request URL into its collection and, for an item operation, id.
///
/// Every TMF path is `…/{apiRoot}/v{N}/{collection}[/{id}]`, so the version
/// segment locates the rest when the URL does not sit under `base_url` — which
/// happens for an absolute `href` into another API, or a client pointed at a
/// hand-written base.
pub(crate) fn route(base_url: &str, url: &str) -> Option<(String, Option<String>)> {
    let path = url.split('?').next().unwrap_or_default();
    let rest = path
        .strip_prefix(base_url)
        .or_else(|| after_version_segment(path))
        .unwrap_or_else(|| trailing_segments(path));

    let mut segments = rest.split('/').filter(|segment| !segment.is_empty());
    let collection = segments.next()?.to_owned();
    let id = segments.next().map(ToOwned::to_owned);
    // Anything deeper is not a shape TMF v5 defines.
    if segments.next().is_some() {
        return None;
    }
    Some((collection, id))
}

/// Everything after the `v5`-style API version segment, if the path has one.
fn after_version_segment(path: &str) -> Option<&str> {
    let without_scheme = path.split_once("://").map_or(path, |(_, rest)| rest);
    let mut offset = path.len() - without_scheme.len();

    for segment in without_scheme.split('/') {
        let is_version = segment.starts_with('v')
            && segment.len() > 1
            && segment[1..].chars().all(|c| c.is_ascii_digit() || c == '.');
        if is_version {
            return Some(&path[offset + segment.len()..]);
        }
        offset += segment.len() + 1;
    }
    None
}

/// The last one or two segments of a path, as a fallback for an unrecognised
/// root.
fn trailing_segments(path: &str) -> &str {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let take = segments.len().min(2);
    let tail = &segments[segments.len() - take..];
    tail.first().map_or("", |first| {
        let start = path.rfind(first).unwrap_or(0);
        &path[start..]
    })
}

/// Delivers a change notification to a registered listener.
///
/// The handler names the event, matches it against each subscription's filter
/// and works out where it goes — see [`crate::server`]. This is
/// the part only a deployment can decide: whether delivery is a blocking `POST`,
/// a queue publish, a retry loop, or a log line.
///
/// ```
/// use rutmf::server::{Listener, MemoryStore, Notifier, TmfHandler};
/// use serde_json::Value;
///
/// /// Hands each notification to a queue rather than blocking the write.
/// struct Enqueue(tokio::sync::mpsc::UnboundedSender<(String, Value)>);
///
/// #[async_trait::async_trait]
/// impl Notifier for Enqueue {
///     async fn notify(&self, listener: &Listener, event_type: &str, event: &Value) {
///         // `delivery_url` is where TMF630 says this event goes.
///         let _ = self.0.send((listener.delivery_url(event_type), event.clone()));
///     }
/// }
///
/// # fn demo(tx: tokio::sync::mpsc::UnboundedSender<(String, Value)>) {
/// let handler = TmfHandler::new("https://host/tmf-api/x/v5", MemoryStore::new())
///     .with_notifier(Enqueue(tx));
/// # let _ = handler;
/// # }
/// ```
///
/// `notify` is awaited before the handler answers, so a slow notifier slows the
/// write that caused it — spawning instead would need a runtime this layer does
/// not have. Hand the event to a channel, as above, to return first.
#[async_trait::async_trait]
pub trait Notifier: Send + Sync {
    /// Delivers `event` to `listener`.
    ///
    /// `event_type` is the `{Resource}{Kind}Event` class name, passed separately
    /// because [`Listener::delivery_url`] needs it.
    ///
    /// Returns nothing: a notification that cannot be delivered must not fail
    /// the write that produced it, so errors are the implementation's to handle.
    async fn notify(&self, listener: &Listener, event_type: &str, event: &Value);
}

/// The explicit no-op: subscriptions are served, nothing is delivered.
///
/// Configuring this is the same as configuring nothing, and it exists so that
/// "notifications are deliberately off here" can be written down.
#[async_trait::async_trait]
impl Notifier for () {
    async fn notify(&self, _: &Listener, _: &str, _: &Value) {}
}

/// Assigns the identifier of a newly created resource.
///
/// The default, [`RandomId`], is deliberately replaceable. Identifier policy is
/// a deployment's decision, not a library's: real systems want a `UUIDv7` for
/// index locality, a ULID for sortability, a database sequence, or a
/// tenant-prefixed scheme their operations tooling already understands. Supply
/// one with [`TmfHandler::with_id_generator`].
///
/// A client that sends its own `id` keeps it; this is consulted only when the
/// creating request left the choice to the server.
///
/// ```
/// use rutmf::server::{IdGenerator, MemoryStore, TmfHandler};
/// use std::sync::atomic::{AtomicU64, Ordering};
///
/// /// Ids from a database-style sequence, for a deployment that wants them.
/// struct Sequence(AtomicU64);
///
/// impl IdGenerator for Sequence {
///     fn next_id(&self) -> String {
///         self.0.fetch_add(1, Ordering::Relaxed).to_string()
///     }
/// }
///
/// let handler = TmfHandler::new("https://host/tmf-api/x/v5", MemoryStore::new())
///     .with_id_generator(Sequence(AtomicU64::new(1)));
/// ```
pub trait IdGenerator: Send + Sync {
    /// The identifier for a resource about to be created.
    fn next_id(&self) -> String;
}

/// The default [`IdGenerator`]: 128 unpredictable bits, rendered as hex.
///
/// # Why not a counter, and why not the obvious one-liner
///
/// A sequential id tells anyone who creates a resource how many the server
/// holds, and makes the neighbouring one guessable — which across tenants is a
/// disclosure rather than an inconvenience.
///
/// The tempting zero-dependency shortcut, `RandomState::new().hash_one(…)`,
/// does not fix that as thoroughly as it looks. `RandomState::new()` seeds a
/// thread-local pair *once* from the operating system and then **increments**
/// it on every subsequent call, so hashing the same input each time is a keyed
/// hash over a marching key — a related-key construction, not a draw from a
/// random source, and only 64 bits wide.
///
/// So this keeps one `RandomState` for the life of the generator — that is the
/// secret, seeded once from the OS — and uses it as a keyed pseudo-random
/// function over a counter, which is what `SipHash` is actually built for. Two
/// domain-separated evaluations give 128 bits.
pub struct RandomId {
    key: std::hash::RandomState,
    counter: std::sync::atomic::AtomicU64,
}

impl Default for RandomId {
    fn default() -> Self {
        Self {
            key: std::hash::RandomState::new(),
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

impl std::fmt::Debug for RandomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The key is a secret; printing it would let an observer of one log
        // line predict every id the server goes on to mint.
        f.debug_struct("RandomId").finish_non_exhaustive()
    }
}

impl IdGenerator for RandomId {
    fn next_id(&self) -> String {
        use std::hash::BuildHasher;
        use std::sync::atomic::Ordering;

        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        // Distinct inputs, one key: two independent 64-bit PRF outputs.
        let high = self.key.hash_one((0u8, n));
        let low = self.key.hash_one((1u8, n));
        format!("{high:016x}{low:016x}")
    }
}

/// Reads a request body as JSON, or says why it could not be.
fn json_body(request: &TmfRequest) -> Result<Value, String> {
    let Some(body) = request.body.as_ref() else {
        return Err("Missing request body".to_owned());
    };
    serde_json::from_slice(body).map_err(|error| format!("Body is not valid JSON: {error}"))
}

/// Which kind of change a `PATCH` was.
///
/// TM Forum raises a lifecycle event for a state move and
/// `…AttributeValueChangeEvent` for any other edit. A `PATCH` is
/// read-modify-write, so the handler holds the resource on both sides.
///
/// Two things make this more than a member comparison:
///
/// - the state member is `lifecycleStatus` in the catalog APIs, `state` in
///   ordering and inventory and `status` in TMF621, so all three are compared;
/// - which event a move raises depends on the *collection*, not the member — see
///   [`state_change_kind`].
///
/// `operatingStatus` is checked first: TMF638's `Service` is the one resource
/// carrying both an administrative `state` and an operational `operatingStatus`,
/// with a listener for each.
fn change_kind(collection: &str, before: &Value, after: &Value) -> EventKind {
    const STATE_MEMBERS: [&str; 3] = ["lifecycleStatus", "state", "status"];

    if before.get("operatingStatus") != after.get("operatingStatus") {
        return EventKind::OperatingStatusChange;
    }

    let moved = STATE_MEMBERS
        .iter()
        .any(|member| before.get(member) != after.get(member));

    if moved {
        state_change_kind(collection)
    } else {
        EventKind::AttributeValueChange
    }
}

/// RFC 9110 §13.1.1: the resource is not the one the client's tag described.
fn precondition_failed() -> TmfResponse {
    error_response(
        StatusCode::PRECONDITION_FAILED,
        "41201",
        "The resource has changed since the tag in If-Match was issued",
    )
}

fn not_found(collection: &str, id: &str) -> TmfResponse {
    error_response(
        StatusCode::NOT_FOUND,
        "40401",
        &format!("No {collection} with id {id}"),
    )
}

/// Renders a [`StoreError`] as the status and TMF630 body it stands for.
fn store_error_response(error: &StoreError) -> TmfResponse {
    let (status, code) = match error {
        StoreError::Accepted { monitor } => {
            let mut headers = HeaderMap::new();
            if let Some(value) = monitor
                .as_deref()
                .and_then(|url| HeaderValue::from_str(url).ok())
            {
                headers.insert(header::LOCATION, value);
            }
            return TmfResponse::new(StatusCode::ACCEPTED, headers, Bytes::new());
        }
        StoreError::Invalid(_) => (StatusCode::BAD_REQUEST, "40001"),
        StoreError::Forbidden(_) => (StatusCode::FORBIDDEN, "40301"),
        StoreError::Conflict(_) => (StatusCode::CONFLICT, "40901"),
        StoreError::Unprocessable(_) => (StatusCode::UNPROCESSABLE_ENTITY, "42201"),
        StoreError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "50001"),
    };
    error_response(status, code, &error.to_string())
}

/// The `ETag` of a stored resource: a strong validator over its content.
///
/// Derived from the resource itself rather than kept beside it, so a store
/// needs no version column and no extra method to take part. Two resources
/// with equal content share a tag, which is exactly the semantics RFC 9110
/// §8.8.3 gives a strong validator.
///
/// Public because a store implementing
/// [`replace_if_unchanged`](super::ResourceStore::replace_if_unchanged)
/// atomically has to compare against the same value the handler issued, and
/// computing it a second way is how a precondition quietly stops holding.
///
/// The hash is `DefaultHasher`, which is neither stable across Rust releases
/// nor collision-resistant against an adversary. Neither matters here: a tag is
/// meaningful only within one running server, and a client that forges one is
/// only able to overwrite a resource it could already overwrite by omitting
/// `If-Match` entirely.
#[must_use]
pub fn entity_tag(resource: &Value) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // Through the serialised form: `Value`'s own `Hash` is not implemented,
    // and the serialisation is what the client saw.
    serde_json::to_string(resource)
        .unwrap_or_default()
        .hash(&mut hasher);
    format!("\"{:016x}\"", hasher.finish())
}

fn insert_etag(headers: &mut HeaderMap, resource: &Value) {
    if let Ok(value) = HeaderValue::from_str(&entity_tag(resource)) {
        headers.insert(header::ETAG, value);
    }
}

/// A parsed `If-Match` precondition.
#[derive(Debug, PartialEq, Eq)]
enum IfMatch {
    /// `If-Match: *` — "only if the resource exists at all", which it does by
    /// the time this is consulted.
    Any,
    /// One or more entity tags, any of which satisfies the precondition.
    Tags(Vec<String>),
}

impl IfMatch {
    fn matches(&self, current: &str) -> bool {
        match self {
            Self::Any => true,
            // A weak comparison would be wrong here: RFC 9110 §13.1.1 requires
            // strong comparison for `If-Match`, because the point is to guard a
            // write against a resource that changed in any way at all.
            Self::Tags(tags) => tags.iter().any(|tag| tag == current),
        }
    }
}

/// Reads the `If-Match` header, if the request carries a usable one.
fn if_match(request: &TmfRequest) -> Option<IfMatch> {
    parse_precondition(request, header::IF_MATCH)
}

/// Reads the `If-None-Match` header, if the request carries a usable one.
///
/// Parsed into the same type as `If-Match` because the syntax is identical;
/// what differs is the sense in which the caller uses the answer, and the
/// comparison strength RFC 9110 prescribes. That difference is real: §13.1.2
/// requires *weak* comparison for `If-None-Match`, where §13.1.1 requires strong
/// comparison for `If-Match`. It makes no difference here, because
/// [`entity_tag`] only ever issues strong validators — a weak one would come
/// from a client, and a client's `W/"x"` genuinely does not match this server's
/// `"x"`, since the two were computed by different rules.
fn if_none_match(request: &TmfRequest) -> Option<IfMatch> {
    parse_precondition(request, header::IF_NONE_MATCH)
}

fn parse_precondition(request: &TmfRequest, name: header::HeaderName) -> Option<IfMatch> {
    let raw = request.headers.get(name)?.to_str().ok()?.trim();

    if raw == "*" {
        return Some(IfMatch::Any);
    }
    let tags: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        // A `W/` prefix marks a weak validator; it can never satisfy a strong
        // comparison, but keeping it verbatim makes that a mismatch rather
        // than an accidental match on the bare tag.
        .map(ToOwned::to_owned)
        .collect();
    (!tags.is_empty()).then_some(IfMatch::Tags(tags))
}

fn insert_count(headers: &mut HeaderMap, name: &'static str, value: usize) {
    if let Ok(value) = HeaderValue::from_str(&value.to_string()) {
        headers.insert(name, value);
    }
}

fn json_response(status: StatusCode, mut headers: HeaderMap, body: &Value) -> TmfResponse {
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let bytes = serde_json::to_vec(body).unwrap_or_else(|_| b"null".to_vec());
    TmfResponse::new(status, headers, Bytes::from(bytes))
}

/// Renders a TMF630 `Error` body.
pub(crate) fn error_response(status: StatusCode, code: &str, reason: &str) -> TmfResponse {
    let body = serde_json::json!({
        "code": code,
        "reason": reason,
        "status": status.as_u16().to_string(),
        "@type": "Error",
    });
    json_response(status, HeaderMap::new(), &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_collection_and_an_item_route_apart() {
        let base = "http://host/tmf-api/productCatalogManagement/v5";
        assert_eq!(
            route(base, &format!("{base}/productOffering")),
            Some(("productOffering".to_owned(), None))
        );
        assert_eq!(
            route(base, &format!("{base}/productOffering/7655")),
            Some(("productOffering".to_owned(), Some("7655".to_owned())))
        );
    }

    #[test]
    fn a_url_under_another_root_routes_by_its_version_segment() {
        // An absolute `href` into a different API still has to resolve.
        assert_eq!(
            route(
                "http://host/tmf-api/productCatalogManagement/v5",
                "https://elsewhere.example/tmf-api/partyManagement/v5/individual/7"
            ),
            Some(("individual".to_owned(), Some("7".to_owned())))
        );
    }

    #[test]
    fn a_path_deeper_than_a_resource_does_not_route() {
        let base = "http://host/tmf-api/v5";
        assert_eq!(route(base, &format!("{base}/a/b/c")), None);
    }

    #[test]
    fn ids_are_not_sequential() {
        // A sequential id leaks the size of the store and makes another
        // tenant's resource guessable.
        let generator = RandomId::default();
        let ids: std::collections::BTreeSet<String> =
            (0..64).map(|_| generator.next_id()).collect();

        assert_eq!(ids.len(), 64, "ids collided");
        assert!(!ids.contains("1"), "ids look like a counter");
        assert!(
            ids.iter().all(|id| id.len() == 32),
            "128 bits, rendered as hex"
        );
        // Two generators are independently keyed, so one server's ids say
        // nothing about another's.
        let other = RandomId::default();
        assert!(
            !ids.contains(&other.next_id()),
            "a second generator reproduced the first's sequence"
        );
    }

    #[test]
    fn an_id_generator_can_be_replaced() {
        struct Fixed;
        impl IdGenerator for Fixed {
            fn next_id(&self) -> String {
                "fixed".to_owned()
            }
        }
        let handler = TmfHandler::new(
            "http://host/tmf-api/x/v5",
            crate::server::MemoryStore::new(),
        )
        .with_id_generator(Fixed);
        assert_eq!(handler.ids.next_id(), "fixed");
    }

    #[test]
    fn if_match_is_parsed_and_compared_strongly() {
        fn request(value: &'static str) -> TmfRequest {
            let mut request = TmfRequest::new(Method::PATCH, "http://host/x/v5/a/1");
            request
                .headers
                .insert(header::IF_MATCH, HeaderValue::from_static(value));
            request
        }

        assert_eq!(if_match(&TmfRequest::new(Method::PATCH, "u")), None);
        assert_eq!(if_match(&request("*")), Some(IfMatch::Any));
        assert!(IfMatch::Any.matches("\"abc\""));

        let parsed = if_match(&request("\"abc\", \"def\"")).expect("two tags");
        assert!(parsed.matches("\"def\""));
        assert!(!parsed.matches("\"ghi\""));

        // RFC 9110 §13.1.1 requires strong comparison, so a weak validator
        // never satisfies `If-Match`.
        let weak = if_match(&request("W/\"abc\"")).expect("a weak tag");
        assert!(!weak.matches("\"abc\""));
    }

    #[test]
    fn an_entity_tag_tracks_the_content() {
        let a = serde_json::json!({"id": "1", "name": "old"});
        let b = serde_json::json!({"id": "1", "name": "new"});

        assert_eq!(entity_tag(&a), entity_tag(&a.clone()), "stable");
        assert_ne!(entity_tag(&a), entity_tag(&b), "an edit changes the tag");
        assert!(
            entity_tag(&a).starts_with('"') && entity_tag(&a).ends_with('"'),
            "RFC 9110 entity-tags are quoted"
        );
    }
}
