//! What a backend must provide, and what it gets for free.

use std::collections::BTreeMap;

use serde_json::Value;

use super::semantics::{matches_filters, sort_resources};

/// What a `GET` on a collection selects, ordered and paged.
///
/// This is the request the handler hands a store: *which* resources, in *what*
/// order, and *which slice*. It deliberately does not carry `fields=` — the
/// handler projects the response itself, so a store never has to think about
/// it.
///
/// An in-memory store can satisfy the whole thing with [`Selection::apply`]. A
/// database-backed one translates it into a query, which is the point of
/// passing it down rather than fetching everything and filtering above.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Selection {
    /// Attribute filters, keyed by the TMF630 parameter name — the attribute
    /// path with an optional `.gt`/`.gte`/`.lt`/`.lte`/`.ne`/`.regex` suffix.
    /// A value containing commas is a list meaning "any of". Filters are
    /// combined with `AND`.
    pub filters: BTreeMap<String, String>,
    /// The `sort=` parameter verbatim: comma-separated attributes, each
    /// optionally prefixed with `-` to reverse it.
    pub sort: Option<String>,
    /// How many matching resources to skip.
    pub offset: usize,
    /// How many to return, when the client asked for a bound.
    pub limit: Option<usize>,
    /// An opaque cursor: start immediately after the resource it names.
    ///
    /// TMF621 and TMF639 declare `after`/`before` on three collections. The
    /// cursor is opaque *to the client*; a store chooses what it means. The
    /// in-memory implementation reads it as a resource `id`, which is stable
    /// under the sort the same selection applies.
    pub after: Option<String>,
    /// An opaque cursor: stop immediately before the resource it names.
    pub before: Option<String>,
}

/// The resources a [`Selection`] selected, and how many it matched in total.
///
/// `total` counts everything the filters matched, not the length of `items` —
/// that difference is what fills `X-Total-Count` and decides whether the
/// response is `200` or `206`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Matched {
    /// The slice the selection asked for.
    pub items: Vec<Value>,
    /// How many resources matched the filters, before `offset`/`limit`.
    pub total: usize,
}

impl Matched {
    /// A page of `items` out of `total` matches.
    #[must_use]
    pub fn new(items: Vec<Value>, total: usize) -> Self {
        Self { items, total }
    }

    /// A result that is the whole of what matched.
    #[must_use]
    pub fn complete(items: Vec<Value>) -> Self {
        Self {
            total: items.len(),
            items,
        }
    }
}

impl Selection {
    /// Reads a selection out of the query parameters of a request.
    ///
    /// Everything that is not a TMF630 reserved parameter is a filter.
    ///
    /// # A malformed `offset` or `limit` is an error, not a default
    ///
    /// Reading `limit=abc` as "no limit" answers a request for one page with the
    /// whole collection, and answers it `200`, so the client cannot tell. A
    /// paging parameter that is not a non-negative integer is refused instead,
    /// and [`TmfHandler`](super::TmfHandler) turns that into a `400` naming the
    /// parameter.
    ///
    /// # Errors
    ///
    /// Returns the reason `offset` or `limit` could not be read.
    pub fn from_query(query: &BTreeMap<String, String>) -> Result<Self, String> {
        fn number(query: &BTreeMap<String, String>, name: &str) -> Result<Option<usize>, String> {
            let Some(raw) = query.get(name) else {
                return Ok(None);
            };
            raw.parse()
                .map(Some)
                .map_err(|_| format!("`{name}` must be a non-negative integer, not {raw:?}"))
        }

        Ok(Self {
            filters: query
                .iter()
                .filter(|(key, _)| !super::semantics::is_reserved(key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            sort: query.get("sort").cloned(),
            offset: number(query, "offset")?.unwrap_or(0),
            limit: number(query, "limit")?,
            after: query.get("after").cloned(),
            before: query.get("before").cloned(),
        })
    }

    /// Bounds the page size, and applies it when the client asked for none.
    ///
    /// The store-side counterpart of
    /// [`TmfHandler::with_max_page_size`](super::TmfHandler::with_max_page_size).
    ///
    /// ```
    /// use rutmf::server::Selection;
    /// use std::collections::BTreeMap;
    ///
    /// let mut query = BTreeMap::new();
    /// query.insert("limit".to_owned(), "5000".to_owned());
    ///
    /// let capped = Selection::from_query(&query).unwrap().capped_at(100);
    /// assert_eq!(capped.limit, Some(100));
    ///
    /// // And a request that named no limit gets one.
    /// let defaulted = Selection::default().capped_at(100);
    /// assert_eq!(defaulted.limit, Some(100));
    /// ```
    #[must_use]
    pub fn capped_at(mut self, max: usize) -> Self {
        self.limit = Some(self.limit.map_or(max, |limit| limit.min(max)));
        self
    }

    /// Applies the whole selection to a set of resources held in memory.
    ///
    /// Filters, then sorts, then pages — in that order, which is the order that
    /// makes `total` mean "how many matched" rather than "how many were
    /// stored".
    ///
    /// A store backed by a database should translate the selection into a query
    /// instead; this exists so one backed by a `Vec` does not have to.
    ///
    /// ```
    /// use rutmf::server::Selection;
    /// use std::collections::BTreeMap;
    ///
    /// let resources = vec![
    ///     serde_json::json!({"id": "1", "lifecycleStatus": "Active"}),
    ///     serde_json::json!({"id": "2", "lifecycleStatus": "Retired"}),
    ///     serde_json::json!({"id": "3", "lifecycleStatus": "Active"}),
    /// ];
    ///
    /// let mut query = BTreeMap::new();
    /// query.insert("lifecycleStatus".to_owned(), "Active".to_owned());
    /// query.insert("limit".to_owned(), "1".to_owned());
    ///
    /// let matched = Selection::from_query(&query).unwrap().apply(resources);
    ///
    /// // One returned, but two matched — which is what makes this a `206`.
    /// assert_eq!(matched.items.len(), 1);
    /// assert_eq!(matched.total, 2);
    /// ```
    #[must_use]
    pub fn apply(&self, resources: Vec<Value>) -> Matched {
        let mut kept: Vec<Value> = resources
            .into_iter()
            .filter(|resource| matches_filters(resource, &self.filters))
            .collect();

        if let Some(sort) = &self.sort {
            sort_resources(&mut kept, sort);
        }

        // Cursors bound the window before `offset`/`limit` narrow it, and
        // after sorting — a cursor names a position in the ordered result, so
        // applying it earlier would name a position in a different order.
        if self.after.is_some() || self.before.is_some() {
            // An unknown cursor selects nothing. Falling back to the start
            // would hand a client that sent a stale cursor page one again, and
            // a loop that pages until it sees no new items would never end.
            let start = match &self.after {
                Some(cursor) => match position_of(&kept, cursor) {
                    Some(index) => index + 1,
                    None => kept.len(),
                },
                None => 0,
            };
            let end = match &self.before {
                Some(cursor) => position_of(&kept, cursor).unwrap_or(0),
                None => kept.len(),
            };
            kept = kept
                .into_iter()
                .take(end.max(start))
                .skip(start)
                .collect::<Vec<_>>();
        }

        // `total` counts what the cursors left, so a client paging by cursor
        // still learns whether more remain.
        let total = kept.len();
        let items = kept
            .into_iter()
            .skip(self.offset)
            .take(self.limit.unwrap_or(usize::MAX))
            .collect();

        Matched { items, total }
    }
}

/// Where a cursor points within an ordered result.
fn position_of(resources: &[Value], cursor: &str) -> Option<usize> {
    resources
        .iter()
        .position(|resource| resource.get("id").and_then(Value::as_str) == Some(cursor))
}

/// Why a store could not do what was asked.
///
/// # `Accepted` is here on purpose
///
/// Every v5 `POST` and `PATCH` declares `202` alongside its synchronous answer,
/// because a deployment may fulfil a write asynchronously. A store says so by
/// returning [`StoreError::Accepted`] — the same shape the client side takes,
/// where [`Error::Accepted`] is a variant of the error enum for the same
/// reason: the call asked for a resource and did not produce one. It is
/// control flow, not a failure, and both ends model it the same way.
///
/// [`Error::Accepted`]: crate::api::Error::Accepted
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The write was taken but has not completed. Answers `202 Accepted`.
    #[error("accepted for asynchronous processing")]
    Accepted {
        /// Where the client should poll, sent as `Location`.
        monitor: Option<String>,
    },

    /// The request is malformed or violates a constraint. Answers `400`.
    #[error("{0}")]
    Invalid(String),

    /// The request is well-formed but cannot be satisfied in this state.
    /// Answers `422 Unprocessable Content`.
    #[error("{0}")]
    Unprocessable(String),

    /// The resource conflicts with one that exists. Answers `409`.
    #[error("{0}")]
    Conflict(String),

    /// The caller may not do this. Answers `403`.
    #[error("{0}")]
    Forbidden(String),

    /// Something went wrong that is not the caller's fault. Answers `500`.
    #[error("{0}")]
    Internal(String),
}

/// The result of a store operation.
pub type StoreResult<T> = std::result::Result<T, StoreError>;

/// What a conditional write did.
///
/// The three outcomes are the three answers HTTP needs: `200` with the new
/// resource, `404`, or `412 Precondition Failed`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Replaced {
    /// The write happened; this is the resource as stored.
    Updated(Value),
    /// There is no resource at that id.
    Missing,
    /// The resource changed since the tag the caller was holding, so the write
    /// was refused rather than allowed to discard someone else's.
    Stale,
}

/// The storage behind a TM Forum API.
///
/// Implement this and [`TmfHandler`](super::TmfHandler) supplies the rest: URL
/// routing, attribute filtering, sorting, `fields=` projection, `offset`/`limit`
/// paging with `X-Total-Count`/`X-Result-Count` and a `206` for a partial page,
/// all four `PATCH` content types, TMF630 error bodies, and the status code for
/// each outcome.
///
/// That division is the point. The methods here are about *storage*; nothing in
/// them is about HTTP, and none of them can get the wire format wrong.
///
/// # Not found is `Ok(None)`
///
/// A missing resource is an ordinary outcome, not a [`StoreError`] — the
/// handler turns it into a `404` with a TMF630 body. Reserve the error type for
/// things that went wrong.
///
/// # Example
///
/// ```
/// use std::collections::BTreeMap;
/// use std::sync::Mutex;
///
/// use serde_json::Value;
/// use rutmf::server::{Matched, ResourceStore, Selection, StoreResult};
///
/// #[derive(Default)]
/// struct Offerings(Mutex<Vec<Value>>);
///
/// #[async_trait::async_trait]
/// impl ResourceStore for Offerings {
///     async fn list(&self, _collection: &str, selection: &Selection) -> StoreResult<Matched> {
///         Ok(selection.apply(self.0.lock().unwrap().clone()))
///     }
///
///     async fn get(&self, _collection: &str, id: &str) -> StoreResult<Option<Value>> {
///         Ok(self.0.lock().unwrap().iter()
///             .find(|r| r.get("id").and_then(Value::as_str) == Some(id))
///             .cloned())
///     }
///
///     async fn create(&self, _collection: &str, resource: Value) -> StoreResult<Value> {
///         self.0.lock().unwrap().push(resource.clone());
///         Ok(resource)
///     }
///
///     async fn replace(&self, _c: &str, id: &str, resource: Value) -> StoreResult<Option<Value>> {
///         let mut held = self.0.lock().unwrap();
///         let Some(slot) = held.iter_mut()
///             .find(|r| r.get("id").and_then(Value::as_str) == Some(id)) else {
///             return Ok(None);
///         };
///         *slot = resource.clone();
///         Ok(Some(resource))
///     }
///
///     async fn delete(&self, _collection: &str, id: &str) -> StoreResult<bool> {
///         let mut held = self.0.lock().unwrap();
///         let before = held.len();
///         held.retain(|r| r.get("id").and_then(Value::as_str) != Some(id));
///         Ok(held.len() != before)
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait ResourceStore: Send + Sync {
    /// The resources in `collection` that `selection` selects.
    ///
    /// The returned [`Matched::total`] must count everything the filters
    /// matched, not the length of the slice — the handler needs the difference
    /// to set `X-Total-Count` and to decide between `200` and `206`.
    async fn list(&self, collection: &str, selection: &Selection) -> StoreResult<Matched>;

    /// One resource by id, or `Ok(None)` if there is no such resource.
    async fn get(&self, collection: &str, id: &str) -> StoreResult<Option<Value>>;

    /// Stores a new resource and returns it as stored.
    ///
    /// The handler has already assigned `id` and `href` if the client omitted
    /// them, so what arrives here is complete. Return the resource the server
    /// will report — a store that normalises or defaults members should return
    /// the normalised form, because that is what the client is sent.
    async fn create(&self, collection: &str, resource: Value) -> StoreResult<Value>;

    /// Overwrites the resource at `id`, or `Ok(None)` if there is none.
    ///
    /// This is the write half of a `PATCH`: the handler reads the resource,
    /// applies the patch — merge or RFC 6902, whichever the request asked for —
    /// and hands back the result. A store never sees a patch document, which is
    /// why it cannot apply one wrongly.
    async fn replace(
        &self,
        collection: &str,
        id: &str,
        resource: Value,
    ) -> StoreResult<Option<Value>>;

    /// Removes the resource at `id`, reporting whether there was one.
    async fn delete(&self, collection: &str, id: &str) -> StoreResult<bool>;

    /// Overwrites the resource at `id`, but only while it still hashes to
    /// `expected_tag`.
    ///
    /// This is what makes `If-Match` mean something. A `PATCH` is
    /// read-modify-write, so checking the tag and *then* writing leaves the very
    /// lost update the header was sent to prevent: another write landing between
    /// the two is discarded silently, with `200` to both clients.
    ///
    /// # The default is not atomic
    ///
    /// It reads, compares and calls [`replace`](Self::replace), which narrows
    /// the window without closing it — the most a default can do over methods
    /// that promise no atomicity between them.
    ///
    /// **Override it.** Nearly every backend can do this in one operation: a
    /// `WHERE version = ?` on the update, a conditional write, a compare-and-
    /// swap. [`MemoryStore`](super::MemoryStore) does it under its own lock.
    ///
    /// `expected_tag` is what [`entity_tag`](super::entity_tag) computes, so a
    /// store with no version column can answer by hashing what it holds.
    async fn replace_if_unchanged(
        &self,
        collection: &str,
        id: &str,
        resource: Value,
        expected_tag: &str,
    ) -> StoreResult<Replaced> {
        match self.get(collection, id).await? {
            None => Ok(Replaced::Missing),
            Some(current) if super::entity_tag(&current) != expected_tag => Ok(Replaced::Stale),
            Some(_) => Ok(self
                .replace(collection, id, resource)
                .await?
                .map_or(Replaced::Missing, Replaced::Updated)),
        }
    }

    /// Removes the resource at `id`, but only while it still hashes to
    /// `expected_tag`.
    ///
    /// The delete half of [`replace_if_unchanged`](Self::replace_if_unchanged),
    /// and carrying the same caveat: the default is a read-then-write, and a
    /// backend that can make it one operation should.
    async fn delete_if_unchanged(
        &self,
        collection: &str,
        id: &str,
        expected_tag: &str,
    ) -> StoreResult<Replaced> {
        match self.get(collection, id).await? {
            None => Ok(Replaced::Missing),
            Some(current) if super::entity_tag(&current) != expected_tag => Ok(Replaced::Stale),
            Some(current) => {
                if self.delete(collection, id).await? {
                    Ok(Replaced::Updated(current))
                } else {
                    Ok(Replaced::Missing)
                }
            }
        }
    }

    /// Whether this server serves `collection` at all.
    ///
    /// The default accepts every name, which is what a general-purpose store
    /// wants. Override it to answer `404` for a path the API does not define,
    /// rather than an empty list — an empty collection and a collection that
    /// does not exist are different answers, and a client can act on the
    /// difference.
    ///
    /// # Include [`HUB_COLLECTION`]
    ///
    /// Subscriptions are an ordinary collection named `hub`, so an override
    /// listing only the resource collections makes `POST /hub` a `404` and
    /// **nobody can subscribe to anything**:
    ///
    /// ```
    /// # use rutmf::server::HUB_COLLECTION;
    /// # async fn has_collection(collection: &str) -> bool {
    /// collection == "productOffering" || collection == HUB_COLLECTION
    /// # }
    /// ```
    ///
    /// The store then receives `create`/`get`/`delete` for `hub` and has to keep
    /// those rows somewhere — the `serve_catalog` example uses a second `Vec`.
    ///
    /// [`HUB_COLLECTION`]: super::HUB_COLLECTION
    async fn has_collection(&self, collection: &str) -> bool {
        let _ = collection;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn reserved_parameters_are_not_filters() {
        let selection = Selection::from_query(&query(&[
            ("fields", "id,name"),
            ("sort", "-name"),
            ("offset", "10"),
            ("limit", "5"),
            ("lifecycleStatus", "Active"),
        ]))
        .expect("well-formed paging parameters");

        assert_eq!(selection.filters, query(&[("lifecycleStatus", "Active")]));
        assert_eq!(selection.sort.as_deref(), Some("-name"));
        assert_eq!(selection.offset, 10);
        assert_eq!(selection.limit, Some(5));
    }

    #[test]
    fn total_counts_matches_not_the_page() {
        let resources = (0..10)
            .map(|i| serde_json::json!({"id": i.to_string(), "state": "active"}))
            .collect();

        let matched = Selection::from_query(&query(&[("state", "active"), ("limit", "3")]))
            .expect("well-formed")
            .apply(resources);

        assert_eq!(matched.items.len(), 3);
        assert_eq!(matched.total, 10);
    }

    #[test]
    fn an_absent_limit_returns_everything_after_the_offset() {
        let resources = (0..5)
            .map(|i| serde_json::json!({"id": i.to_string()}))
            .collect();

        let matched = Selection::from_query(&query(&[("offset", "2")]))
            .expect("well-formed")
            .apply(resources);

        assert_eq!(matched.items.len(), 3);
        assert_eq!(matched.total, 5);
    }

    #[test]
    fn filtering_happens_before_paging() {
        // If paging came first, a filter would only see the first page — and
        // `total` would be the size of that page rather than of the match.
        let resources = (0..10)
            .map(|i| serde_json::json!({"id": i.to_string(), "odd": i % 2 == 1}))
            .collect();

        let matched = Selection::from_query(&query(&[("odd", "true"), ("limit", "2")]))
            .expect("well-formed")
            .apply(resources);

        assert_eq!(matched.total, 5);
        assert_eq!(matched.items.len(), 2);
        assert_eq!(matched.items[0]["id"], "1");
    }
}

#[cfg(test)]
mod cursor_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn resources() -> Vec<Value> {
        (1..=5)
            .map(|n| serde_json::json!({"id": n.to_string(), "name": format!("r{n}")}))
            .collect()
    }

    fn select(pairs: &[(&str, &str)]) -> Selection {
        let query: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Selection::from_query(&query).expect("well-formed")
    }

    fn ids(matched: &Matched) -> Vec<&str> {
        matched
            .items
            .iter()
            .filter_map(|item| item.get("id").and_then(Value::as_str))
            .collect()
    }

    #[test]
    fn after_starts_from_the_resource_following_the_cursor() {
        let matched = select(&[("after", "2")]).apply(resources());
        assert_eq!(ids(&matched), ["3", "4", "5"]);
        // `total` reports what is left after the cursor, so a client paging by
        // cursor still learns whether more remain.
        assert_eq!(matched.total, 3);
    }

    #[test]
    fn before_stops_at_the_resource_preceding_the_cursor() {
        let matched = select(&[("before", "4")]).apply(resources());
        assert_eq!(ids(&matched), ["1", "2", "3"]);
    }

    #[test]
    fn the_two_cursors_bound_a_window() {
        let matched = select(&[("after", "1"), ("before", "5")]).apply(resources());
        assert_eq!(ids(&matched), ["2", "3", "4"]);
    }

    #[test]
    fn an_unknown_cursor_selects_nothing() {
        // Paging from the start instead would hand a client that sent a stale
        // cursor page one again — and a loop that pages until nothing new
        // arrives would never terminate.
        assert!(
            select(&[("after", "nope")])
                .apply(resources())
                .items
                .is_empty()
        );
        assert!(
            select(&[("before", "nope")])
                .apply(resources())
                .items
                .is_empty()
        );
    }

    #[test]
    fn a_cursor_is_not_mistaken_for_an_attribute_filter() {
        // Treated as a filter, `after` would match a member named `after` —
        // that is, nothing at all, for every cursor request TMF621 and TMF639
        // define.
        let selection = select(&[("after", "2"), ("before", "5"), ("filter", "$[*]")]);
        assert!(selection.filters.is_empty());
    }

    #[test]
    fn cursors_compose_with_limit() {
        let matched = select(&[("after", "1"), ("limit", "2")]).apply(resources());
        assert_eq!(ids(&matched), ["2", "3"]);
        // Four remain after the cursor; two were returned, so this is a 206.
        assert_eq!(matched.total, 4);
    }
}
