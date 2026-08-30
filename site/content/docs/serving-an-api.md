+++
title = "Serving an API"
description = "Implement a conformant TM Forum API in Rust: the ResourceStore trait, TMF630 semantics for free, ETag/If-Match concurrency control, hub notifications and pluggable identifiers."
weight = 40
+++

The other direction: you have the data, and you need to expose it as a
conformant TM Forum API. Implement five methods — none of them about HTTP — and
`TmfHandler` supplies TMF630.

```rust
use rutmf::server::{
    Matched, ResourceStore, Selection, StoreError, StoreResult,
};

#[async_trait::async_trait]
impl ResourceStore for Catalog {
    async fn list(&self, _c: &str, selection: &Selection) -> StoreResult<Matched> {
        // `Selection::apply` filters, sorts and pages an in-memory collection.
        // A SQL store turns the selection into WHERE / ORDER BY / LIMIT.
        Ok(selection.apply(self.offerings.lock().unwrap().clone()))
    }

    async fn create(&self, _c: &str, resource: Value) -> StoreResult<Value> {
        // Your rules, not the schema's — requiredness is already enforced by
        // `ProductOfferingCreate`, so what is left here is business logic.
        if self.name_taken(&resource) {
            return Err(StoreError::Conflict("that name is taken".into()));
        }
        self.offerings.lock().unwrap().push(resource.clone());
        Ok(resource)
    }

    // get, replace, delete — and that is the whole trait.
}
```

Then serve it:

```rust
let handler = TmfHandler::new(
    "https://mycsp.com/tmf-api/productCatalogManagement/v5",
    catalog,
);

let app = axum::Router::new().nest(
    "/tmf-api/productCatalogManagement/v5",
    rutmf::server::router(handler),   // feature: server-axum
);
```

## Why the split is by concern, not by operation

A generated server — `openapi-generator`'s `rust-axum`, and every tool like
it — emits a trait with one method per operation per API and leaves the HTTP
semantics to you. That is the wrong decomposition for TM Forum, because **every
TMF API is the same API**: one or two collections, `GET`/`POST` on the
collection, `GET`/`PATCH`/`DELETE` on an item, plus a hub. TMF630 defines, once,
what all of it means. A per-API trait makes every implementation re-derive those
semantics, and each gets them slightly wrong.

So the split is:

| Concern | Who decides |
|---|---|
| Routing, filtering, sorting, `fields=`, paging, count headers, `200` vs `206`, four `PATCH` content types, `ETag`/`If-Match`, `Location`, error bodies, status codes | `TmfHandler` — once, for every API |
| What is stored, what the business rules are, what a conflict means | `ResourceStore` — five methods, yours |

Nothing in `ResourceStore` is about HTTP, so nothing in it can get the wire
format wrong. The `Selection` is passed *down* rather than the handler fetching
everything and filtering above it, so a database-backed store can translate it
into a query instead of loading the collection.

A missing resource is `Ok(None)`, not an error — the handler turns it into a
`404` with a TMF630 body. Reserve `StoreError` for things that actually went
wrong.

## What the handler gets right that hand-rolled ones usually do not

**Filters descend into collections.** Most of what is worth filtering on in TM
Forum is an array — `relatedParty`, `characteristic`, `productOfferingPrice`. A
path that stopped at the array could never match `relatedParty.id=42`, so a
dotted path distributes over the elements and matches if any element satisfies
it. An explicitly numeric segment still selects one element, so
`relatedParty.0.id` addresses the first.

`ne` is the deliberate exception: over a collection it means **no** element
matches. Reading it as "some element differs" would match a resource that plainly
has the value you are excluding.

**Reserved parameters are not filters.** `fields`, `offset`, `limit`, `sort`,
`after`, `before` and `filter` name the request rather than the resource, so the
handler never matches them against a member. The three collections that declare
cursor pagination are served properly: `after`/`before` bound the window after
sorting and before `offset`/`limit`, and an unknown cursor selects nothing
rather than quietly returning page one — a client looping until it sees nothing
new would otherwise never stop.

**An unsupported filter is refused, not ignored.** The handler implements
attribute filtering, not the JSONPath `filter` that TMF621 and TMF639 declare.
A request carrying one gets a `400` with a TMF630 body. Ignoring it would answer
a request to *narrow* a collection with the whole collection, which is the wrong
way to be wrong.

**A `limit` the server cannot read is refused too, for the same reason.**
`?limit=abc` read as "no limit" answers a request for one page with the entire
collection, and answers it `200`. `offset` and `limit` must be non-negative
integers or the request is a `400` naming the parameter.

**Timestamps compare as instants.** `2026-01-01T01:00:00+02:00` sorts *before*
`2026-01-01T00:00:00Z` as an instant and after it as text, and TM Forum's own
examples carry non-`Z` offsets — so filtering and sorting parse before comparing.
A bare `YYYY-MM-DD` bound is that day's midnight UTC; a value that is not a date
still compares as text, so lifecycle names order as expected.

**A repeated parameter widens rather than replaces.** `?state=held&state=pending`
is what most HTTP client libraries produce from a list, and it means what
`?state=held,pending` means. Keeping the last occurrence would answer with
`pending` alone.

### Bound the page size

A collection `GET` that names no `limit` returns everything the filters matched,
so the size of the response is chosen by whoever sends the request.

```rust
let handler = TmfHandler::new(base_url, catalog).with_max_page_size(100);
```

It lowers a `limit` larger than the maximum and supplies one where the request
named none, so a client that pages properly is unaffected — and `X-Total-Count`
still reports the whole match, making a capped response a `206`.

Off by default: TMF630 permits a maximum without naming one, and turning it on
silently would change what a working deployment returns. Turn it on.

**`fields=` selects into a member.** `fields=productSpecification.id` narrows
that member rather than dropping it, and arrays project element-wise. `id`, `href`
and `@type` are always returned, whatever was asked for.

**`add` and `replace` are different operations.** On an array, RFC 6902 `add`
*inserts* and shifts every later element along, while `replace` overwrites in
place; `replace` also requires its target to exist. Treating the two alike
silently lengthens the array a client meant to edit — with no error anywhere.

Patches are **atomic** (RFC 6902 §5): applied to a copy and written back only on
success, so a failed `test` operation is a real precondition rather than leaving
the resource half-patched.

## Concurrent edits

A `PATCH` is read-modify-write, so two clients editing different members of one
resource can each discard the other's change with no error on either side. HTTP's
answer is conditional requests, and the handler implements them:

- `GET`, `POST` and `PATCH` return an `ETag` derived from the stored resource;
- `PATCH` and `DELETE` honour `If-Match`, answering `412 Precondition Failed`
  when the tag no longer matches (RFC 9110 §13.1.1, strong comparison);
- `GET` honours `If-None-Match`, answering `304 Not Modified` with no body when
  the client already holds the current version (§13.1.2).

Your store needs no version column to take part — the tag is computed from the
resource's own content, by `rutmf::server::entity_tag`.

```console
GET /productOffering/42          →  200, ETag: "9f2b…"
GET /productOffering/42          →  304   (If-None-Match: "9f2b…", unchanged)
PATCH /productOffering/42        →  412   (If-Match: "9f2b…", but it changed)
```

The [client layer](@/docs/calling-an-api.md) is the other end of the same
exchange — `Conditional::fetch`, `Tagged::update`, `fetch_if_changed` — and
`tests/server.rs` drives the loop over a socket.

### Make the write conditional too, or the race reopens

Checking the tag and *then* writing is two steps, and a `PATCH` is
read-modify-write: the handler reads the resource, applies the patch and writes
the result back. If another request lands in between, an unconditional write
discards it — silently, with `200` to both clients. That is the exact lost update
`If-Match` was sent to prevent, so a check followed by a separate write leaves
the hole open.

`ResourceStore` therefore has two more methods, both **defaulted**, so nothing
above is untrue — you can still implement five and get correct behaviour:

```rust
async fn replace_if_unchanged(
    &self, collection: &str, id: &str, resource: Value, expected_tag: &str,
) -> StoreResult<Replaced>;

async fn delete_if_unchanged(
    &self, collection: &str, id: &str, expected_tag: &str,
) -> StoreResult<Replaced>;
```

`Replaced` is the three answers HTTP needs — `Updated`, `Missing`, `Stale` —
mapping onto `200`, `404` and `412`.

The default reads, compares and writes as three steps, which narrows the window
without closing it; that is the most a default can do over methods that promise
no atomicity between them. **Override them.** Almost every real backend can do
this in one operation — a `WHERE version = ?` on the update, a conditional write,
a compare-and-swap — and that is where the guarantee actually comes from.
`MemoryStore` does it under its own lock.

The handler only takes the conditional path when the client sent `If-Match`. A
bare `PATCH` is allowed to clobber, and turning ordinary concurrent edits into
`412`s nobody asked for would be the wrong kind of strict.

## Identifiers

`IdGenerator` is a seam, because identifier policy is a deployment's decision:
real systems want a UUIDv7 for index locality, a ULID for sortability, or a
database sequence their operations tooling already understands.

```rust
use rutmf::server::{IdGenerator, TmfHandler};
use std::sync::atomic::{AtomicU64, Ordering};

struct Sequence(AtomicU64);

impl IdGenerator for Sequence {
    fn next_id(&self) -> String {
        self.0.fetch_add(1, Ordering::Relaxed).to_string()
    }
}

let handler = TmfHandler::new(base_url, catalog).with_id_generator(Sequence(AtomicU64::new(1)));
```

The default gives 128 unpredictable bits. A sequential id leaks how many
resources the server holds and makes the neighbouring one guessable, which across
tenants is a disclosure rather than an inconvenience — so if you replace it, be
deliberate about that trade.

A client that sends its own `id` keeps it; the generator is consulted only when
the creating request left the choice to the server.

## Two things that follow

**The test server is not a second implementation.** `MockTmfServer` *is*
`TmfHandler<MemoryStore>` plus a `Transport` shim. The semantics the
591-fixture conformance corpus exercises are the same code a real deployment
runs, so the corpus vouches for both and the two cannot drift.

**The client checks the server.** The repository's `tests/server.rs` serves a
custom store over a real socket and calls it with this crate's own client. If the
two ends disagree about what `206` means, which header carries the count, or when
a `412` is due, the tests fail — because each end is the other's oracle.

## Notifications

Serving `/hub` is only half the job. A conformant server then `POST`s to the
registered callback whenever a resource changes — and getting there means naming
the event `{Resource}{Kind}Event`, wrapping the resource under the right payload
member, reading each subscription's `query` as a TMF630 filter to decide who
wants it, and appending `/listener/{eventName}` to the callback.

All of that is TMF630 semantics, so the handler does it. A `POST`, `PATCH` or
`DELETE` through the API raises the right event by itself — including telling a
lifecycle move from an ordinary edit (`…AttributeValueChangeEvent`), which the
handler can do because a `PATCH` is read-modify-write and it holds the resource
on both sides.

### Naming a lifecycle move is not the one-liner it looks like

Most APIs raise `…StateChangeEvent`. **TMF621 and TMF634 raise
`…StatusChangeEvent` for the same thing**, and TMF634 does so over a member it
still spells `lifecycleStatus`, so the difference cannot be read off the
resource. TMF638 separates the two outright: its `Service` carries both a `state`
and an `operatingStatus`, with a listener for each.

Getting it wrong is invisible in testing and total in production — a subscriber
registered for the name TMF634 declares receives nothing, ever, from a server
raising the other spelling, and neither end reports anything.

So `rutmf::server::state_change_kind` records which collections use which,
transcribed from the vendored `/listener/…` paths, and `tests/coverage.rs`
re-reads those paths and fails if it has drifted:

```rust
use rutmf::core::EventKind;
use rutmf::server::state_change_kind;

assert_eq!(state_change_kind("productOffering"), EventKind::StateChange);
assert_eq!(state_change_kind("resourceCatalog"), EventKind::StatusChange);
```

You do not call it — the handler does. It is public because a fulfilment worker
raising its own events through `TmfHandler::notify` needs the same answer.

What is left is the one part only a deployment can decide: whether delivery is a
blocking `POST`, a queue publish, or a retry loop.

```rust
use rutmf::server::{Listener, Notifier};

struct Enqueue(tokio::sync::mpsc::UnboundedSender<(String, Value)>);

#[async_trait::async_trait]
impl Notifier for Enqueue {
    async fn notify(&self, listener: &Listener, event_type: &str, event: &Value) {
        // `delivery_url` is where TMF630 says this event goes.
        let _ = self.0.send((listener.delivery_url(event_type), event.clone()));
    }
}

let handler = TmfHandler::new(base_url, catalog).with_notifier(Enqueue(tx));
```

`notify` is awaited before the handler answers, so hand the event to a channel
if you want the write to return first — spawning would need a runtime this layer
does not have.

Without a `Notifier`, subscriptions are still stored and read back; nothing is
sent. And `TmfHandler::notify` is public, because a change made *outside* a
request — a fulfilment worker moving an order to `completed` — owes the same
notification and should not have to rebuild the envelope to send it.

`MockTmfServer` is a `Notifier` that writes notifications down instead of
sending them, so `server.notifications()` asserts on the same routing a real
deployment runs.

> ⚠️ Subscriptions are stored as an ordinary collection named `hub`. If you
> override `has_collection` to restrict which collections exist, **include
> `HUB_COLLECTION`** — otherwise `POST /hub` answers `404` and nobody can
> subscribe. Your store then receives `create`/`get`/`delete` for `hub` and has
> to keep those rows somewhere; the `serve_catalog` example holds them in a
> second `Vec`.

## What it deliberately does not do

Validate bodies against the v5 schemas, enforce lifecycle transitions, or
authenticate. Those are decisions only your implementation can make.

Schema validation is available anyway, from the other half of the crate:
deserialise the request body into `ProductOfferingCreate` and requiredness is
enforced by the type.

See the `serve_catalog` example for a working end-to-end version.
