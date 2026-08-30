//! An in-process TM Forum server for tests.
//!
//! Integration-testing code that calls a TMF API usually means either hitting a
//! real system or hand-stubbing JSON. [`MockTmfServer`] is a third option: an
//! in-memory server implementing the TMF630 collection semantics — attribute
//! filtering with the comparison operators, sorting, `fields=` projection,
//! `offset`/`limit` paging with the count headers, and both patch families —
//! reached through the same [`Transport`] seam the real clients use, with no
//! socket involved.
//!
//! ```
//! use rutmf::api::{Query, tmf620::ProductCatalogClient};
//! use rutmf::mock::MockTmfServer;
//!
//! # #[tokio::main] async fn main() -> rutmf::api::Result<()> {
//! let server = MockTmfServer::new();
//! server.seed("productOffering", serde_json::json!({
//!     "id": "7655",
//!     "name": "Basic Firewall for Business",
//!     "lifecycleStatus": "Active",
//!     "@type": "ProductOffering",
//! }));
//!
//! // The server owns its base URL, so there is no string to keep in step.
//! let client = ProductCatalogClient::new(server.base_url(), server.transport())?;
//! let page = client.list_product_offerings(&Query::new()).await?;
//!
//! assert_eq!(page.total_count, Some(1));
//! assert_eq!(page.items[0].name.as_deref(), Some("Basic Firewall for Business"));
//! # Ok(()) }
//! ```
//!
//! # It is the real server, storing in memory
//!
//! This is a thin wrapper over [`TmfHandler`]`<`[`MemoryStore`]`>` plus a
//! [`Transport`] shim and the notification recorder below. The routing, the
//! filtering, the status codes and the error bodies are not a second
//! implementation that might drift from the first — they are [`crate::server`],
//! the same code a real deployment runs. Which is why the mock is worth
//! trusting, and why the conformance suite that exercises it vouches for both.
//!
//! # What it does not do
//!
//! It is a test double, not a TMF implementation. It does not validate bodies
//! against the schemas, enforce lifecycle transitions, or run a real regex
//! engine for `.regex` filters (see [`matches_filters`]).
//!
//! Notifications are *recorded* rather than delivered — delivering them would
//! need a socket, which is the thing this exists to avoid. Everything up to that
//! point is the server layer's own code: a write through the API raises the
//! right `{Resource}{Kind}Event`, matches it against each subscription's filter,
//! and works out the `/listener/{eventName}` URL it would have gone to. The mock
//! is a [`Notifier`] that writes those down instead of
//! sending them, so asserting on [`notifications`] tests the same routing a real
//! deployment runs.
//!
//! [`notifications`]: MockTmfServer::notifications
//!
//! It pages by `offset`/`limit` with the count headers, which is what most TMF
//! deployments do. A server that pages by cursor and leads with a
//! `Link: …; rel="next"` header is a different shape, and the client follows
//! those too; to exercise that path, implement [`Transport`] directly — it is a
//! single method.
//!
//! [`Transport`]: crate::api::Transport
//! [`matches_filters`]: crate::server::matches_filters
//! [`Notifier`]: crate::server::Notifier

use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value;

use crate::api::{Error, Result, TmfRequest, TmfResponse, Transport};
use crate::server::{Listener, MemoryStore, Notifier, TmfHandler};

/// The base URL a [`MockTmfServer`] answers on unless told otherwise.
///
/// The host is under the reserved `.test` TLD (RFC 2606), so a request that
/// escapes the mock cannot reach a real system.
pub const DEFAULT_BASE_URL: &str = "http://mock.tmforum.test/tmf-api/v5";

/// A notification the server would have delivered to a registered listener.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Notification {
    /// Identifier of the `hub` subscription this was matched to.
    pub hub_id: String,
    /// The callback URL the subscription registered.
    pub callback: String,
    /// The `{Resource}{Kind}Event` class name.
    pub event_type: String,
    /// The event body.
    pub event: Value,
}

impl Notification {
    /// Where TMF630 says this event would have been `POST`ed.
    ///
    /// The registered callback plus `/listener/{eventName}` — see
    /// [`Listener::delivery_url`].
    #[must_use]
    pub fn delivery_url(&self) -> String {
        Listener {
            hub_id: self.hub_id.clone(),
            callback: self.callback.clone(),
        }
        .delivery_url(&self.event_type)
    }
}

/// The [`Notifier`] behind [`MockTmfServer::notifications`].
#[derive(Debug, Clone)]
struct Recorder(Arc<Mutex<Vec<Notification>>>);

#[async_trait::async_trait]
impl Notifier for Recorder {
    async fn notify(&self, listener: &Listener, event_type: &str, event: &Value) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Notification {
                hub_id: listener.hub_id.clone(),
                callback: listener.callback.clone(),
                event_type: event_type.to_owned(),
                event: event.clone(),
            });
    }
}

/// An in-memory TM Forum server.
///
/// Cloning shares the same underlying state.
#[derive(Debug, Clone)]
pub struct MockTmfServer {
    handler: Arc<TmfHandler<MemoryStore>>,
    notifications: Arc<Mutex<Vec<Notification>>>,
}

impl Default for MockTmfServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTmfServer {
    /// Creates an empty server answering on [`DEFAULT_BASE_URL`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    /// Creates an empty server answering on `base_url`.
    ///
    /// Knowing its own base URL is what lets the router tell
    /// `GET /…/v5/productOffering` (a collection) from
    /// `GET /…/v5/productOffering/7655` (an item) without a hard-coded list of
    /// collection names — so a mock of an API this crate has no client for
    /// still routes correctly.
    #[must_use]
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        Self {
            handler: Arc::new(
                TmfHandler::new(base_url, MemoryStore::new())
                    .with_notifier(Recorder(Arc::clone(&notifications))),
            ),
            notifications,
        }
    }

    /// The base URL to configure a client with.
    #[must_use]
    pub fn base_url(&self) -> &str {
        self.handler.base_url()
    }

    /// A URL for `path` under this server's base, for hand-built requests.
    #[must_use]
    pub fn url_for(&self, path: &str) -> String {
        format!("{}/{}", self.base_url(), path.trim_start_matches('/'))
    }

    /// The store behind the server, for assertions the API does not expose.
    #[must_use]
    pub fn store(&self) -> &MemoryStore {
        self.handler.store()
    }

    /// Adds one resource to a collection, e.g. `productOffering`.
    pub fn seed(&self, collection: &str, resource: Value) {
        self.store().seed(collection, resource);
    }

    /// Adds many resources to a collection.
    pub fn seed_all(&self, collection: &str, resources: impl IntoIterator<Item = Value>) {
        self.store().seed_all(collection, resources);
    }

    /// The current contents of a collection.
    #[must_use]
    pub fn collection(&self, collection: &str) -> Vec<Value> {
        self.store().collection(collection)
    }

    /// Removes every resource from every collection, and clears notifications.
    pub fn clear(&self) {
        self.store().clear();
        self.lock().clear();
    }

    /// Every notification [`emit`](Self::emit) matched to a subscription.
    ///
    /// The mock has no socket, so events are recorded here rather than
    /// delivered. Assert on this to test that your code subscribes to what you
    /// think it does.
    #[must_use]
    pub fn notifications(&self) -> Vec<Notification> {
        self.lock().clone()
    }

    /// Raises `event` yourself, against every subscription whose filter it
    /// satisfies.
    ///
    /// Writes through the API notify on their own — a `POST` raises a
    /// `…CreateEvent`, a `PATCH` a `…StateChange` or `…AttributeValueChange`
    /// event, a `DELETE` a `…DeleteEvent` — so this is for the events a *domain*
    /// raises rather than a write: a `ProductOrderJeopardyAlertEvent`, or a
    /// state change made by a fulfilment worker rather than by a request.
    ///
    /// A hub's `query` is read as the TMF630 filter it is, so
    /// `eventType=ProductOfferingCreateEvent` matches only that event type and
    /// a hub with no query receives everything.
    ///
    /// Returns how many subscriptions matched.
    pub fn emit(&self, event: &Value) -> usize {
        let event_type = event
            .get("eventType")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();

        let matched: Vec<Notification> =
            crate::server::matching_listeners(&self.collection("hub"), event)
                .into_iter()
                .map(|listener| Notification {
                    hub_id: listener.hub_id,
                    callback: listener.callback,
                    event_type: event_type.clone(),
                    event: event.clone(),
                })
                .collect();

        let count = matched.len();
        self.lock().extend(matched);
        count
    }

    /// A [`Transport`] routing requests into this server.
    ///
    /// [`Transport`]: crate::api::Transport
    #[must_use]
    pub fn transport(&self) -> MockTransport {
        MockTransport {
            server: self.clone(),
        }
    }

    /// Runs a request and returns the client-level error it would produce.
    ///
    /// For tests that assert on error handling rather than on success paths.
    /// The failure is interpreted by the **client layer's own** code, so this
    /// hands back the same [`Error`] the same request would produce through
    /// [`transport`](Self::transport) and a real client — including the
    /// distinction between a TMF630 error body and a gateway's own JSON, which
    /// every member of `TmfError` being optional makes easy to get wrong.
    ///
    /// # Panics
    ///
    /// Panics if the request succeeds.
    pub async fn expect_error(&self, request: &TmfRequest) -> Error {
        let response = self.handler.handle(request).await;
        assert!(!response.status.is_success(), "expected a failure response");
        crate::api::interpret_failure(&response)
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Notification>> {
        self.notifications
            .lock()
            // A panic in another test thread must not cascade into a confusing
            // second panic here; the data is still consistent.
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The [`Transport`] produced by [`MockTmfServer::transport`].
///
/// [`Transport`]: crate::api::Transport
#[derive(Debug, Clone)]
pub struct MockTransport {
    server: MockTmfServer,
}

#[async_trait::async_trait]
impl Transport for MockTransport {
    async fn execute(&self, request: TmfRequest) -> Result<TmfResponse> {
        Ok(self.server.handler.handle(&request).await)
    }
}

#[cfg(test)]
mod tests;
