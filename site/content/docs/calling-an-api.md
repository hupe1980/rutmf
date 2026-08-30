+++
title = "Calling an API"
description = "TMF630 queries and filters, the four PATCH flavours, pagination as a stream, retries and Retry-After, OAuth2 auth, asynchronous 202 writes and typed hub events."
weight = 30
+++

## Queries and filters

TMF630 gives collection endpoints a query vocabulary: attribute selection,
paging, sorting, and filtering with comparison operators. `Query` builds all of
it:

```rust
use rutmf::api::{FilterOp, Query};

let q = Query::new()
    .fields(["id", "name", "lifecycleStatus"])   // fields=id,name,lifecycleStatus
    .filter("lifecycleStatus", "Active")         // lifecycleStatus=Active
    .sort("-lastUpdate")                         // sort=-lastUpdate
    .limit(20);
```

Beyond equality, operators render as a suffix on the attribute name, and
alternatives as a comma-separated list:

```rust
let q = Query::new()
    .filter_any("state", ["acknowledged", "inProgress"])   // state=acknowledged,inProgress
    .filter_op("orderDate", FilterOp::Gte, "2026-01-01")   // orderDate.gte=2026-01-01
    .filter_op("orderDate", FilterOp::Lt,  "2027-01-01");  // orderDate.lt=2027-01-01
```

Repeating an attribute with the same operator **widens** the filter rather than
replacing it, which is what the comma list means on the wire:

```rust
let q = Query::new().filter("state", "held").filter("state", "pending");
assert_eq!(q.to_query_string(), "state=held%2Cpending");
```

Dotted paths reach nested attributes (`productSpecification.id`), and against a
server built on this crate's [server layer](@/docs/serving-an-api.md) they also
reach *into collections* — `relatedParty.id=42` matches a resource that lists
party 42 among several.

### Two collections' worth of exceptions

Attribute filtering is what TMF630 describes and what forty of the forty-one
collections use. Three do something else, and `Query` can express it:

```rust
// TMF621 and TMF639 declare cursor pagination on `troubleTicket`,
// `troubleTicketSpecification` and `resource` — and nothing else does.
let q = Query::new().after("eyJpZCI6IjQyIn0").limit(50);

// The same three declare `filter` as a JSONPath expression, which is a
// different mechanism from the attribute filtering above, not a synonym.
let q = Query::new().json_path("$[?(@.severity=='critical')]");
```

A cursor is opaque: it comes from a server and the only thing to do with one is
send it back. Prefer following the `next` link, which a stream does on its own;
reach for `after` to *resume* from a cursor you stored.

## Patching

TMF v5 declares four `PATCH` content types and pairs each with a body schema:
the two merge flavours take the resource's `_MVO`, the two JSON Patch flavours
take an operation list. Passing the body and the content type as separate
arguments lets you send a combination every conformant server rejects — so they
are one type here.

```rust
use rutmf::api::{JsonPatchOp, Patch};

// A merge patch — the safe default. `&update` converts, so this stays short.
client.update_product_offering("42", &update).await?;

// An RFC 6902 operation list, to change one array element without
// resending the whole array.
let ops = [JsonPatchOp::replace("/productOfferingPrice/0/name", "Promo")];
client.update_product_offering("42", &ops).await?;

// The TM Forum JSONPath dialect, to target by predicate rather than index.
client.update_product_offering("42", Patch::Query(&ops)).await?;
```

The pairing cannot come apart: an `…Update` type is a `PatchBody` and an
operation list is not, which is what keeps the two conversions distinct.

### Removing a member

RFC 7386 has two halves: naming a member sets it, and naming it with `null`
removes it. Setting a field to `None` is neither — it means the patch does not
mention the member at all, which is what leaves it unchanged. So a deletion is
said outright:

```rust
client
    .update_product_offering("42", &update.deleting("description"))
    .await?;
```

Under `Patch::Operations` the same edit is `JsonPatchOp::remove("/description")`,
which a server rejects if the member is not there — where the merge form
silently does nothing. Prefer the operation where that difference matters.

### Patching without discarding someone else's edit

A `PATCH` is read-modify-write. Two operators editing different members of one
order — one adding a note, the other moving `state` — each read it, each patch
it, and the second write discards the first, with `200` to both.

`If-Match` (RFC 9110 §13.1.1) is the guard. Read the resource with the tag the
server issued, and write back through it:

```rust
use rutmf::api::{Conditional, Query, Tagged};

// `fetch` is `get` plus the ETag. The collection comes from the type, so
// there is still no path string.
let held: Tagged<ProductOrder> = client.inner().fetch("42", &Query::new()).await?;

// `Tagged<T>` derefs to `T`, so reading it needs no ceremony.
println!("{:?}", held.state);

match held.update(client.inner(), &update).await {
    Ok(updated) => { /* nobody touched it in between */ }
    Err(e) if e.is_precondition_failed() => {
        // Someone did. Re-read and decide; that is not a call this can make
        // for you, which is why it is not retried.
    }
    Err(e) => return Err(e),
}
```

`held.remove(client.inner())` is the same guard on a `DELETE`.

**This is RFC 9110, not TMF.** The v5 documents declare no request headers, so a
deployment that ignores the precondition answers as it would without one. `fetch`
reports what the server issued, so the two are distinguishable: a `Tagged` with
no tag means conditional writes are unavailable here, and `update` says so with
`Error::NoEntityTag` rather than writing unconditionally.

The [server layer](@/docs/serving-an-api.md) is the other end of the same
exchange.

### Reading only what changed

The mirror image, for an integration that polls. `If-None-Match` asks the server
for the resource *unless* you already have it:

```rust
let mut held: Tagged<ProductOffering> = client.inner().fetch("7655", &Query::new()).await?;

// On the next cycle: pay for the body only if it changed.
if let Some(fresh) = client.inner().fetch_if_changed("7655", &Query::new(), held.etag()).await? {
    held = fresh;
}
```

`None` means `304 Not Modified`: what you hold is current, and no body was
transferred. A server that does not implement `If-None-Match` answers `200`, so
this degrades to an ordinary read rather than breaking.

## Pagination

`list_*` returns one `Page`. `stream_*` returns a `Stream` over the whole
collection, fetching pages as it goes:

```rust
use futures::StreamExt;

let mut offerings = client.stream_product_offerings(Query::new().limit(50));
while let Some(offering) = offerings.next().await {
    println!("{:?}", offering?.name);
}
```

Servers signal "there is more" in four different ways, and the stream handles all
of them, in this order of reliability:

1. **`X-Total-Count`** — an exact answer.
2. **`206 Partial Content`** — TMF630's own mark for a slice of a larger match.
   A server is allowed to omit the counters (computing a total can be expensive)
   and still say this much.
3. **`Link: <…>; rel="next"`** — where the next page is.
4. **A short page** — the only signal left when a server sends none of the above.

The `206` is what keeps a server that omits the counters *and* caps the page size
from truncating the stream: its short page would otherwise read as the end of the
collection. A `200` is deliberately *not* read as "there is no more" — plenty of
deployments answer `200` to everything.

Three details matter in production:

- **When there is a link, the stream follows it.** A cursor is opaque, so
  re-deriving an `offset` request would fetch page one forever. The stream also
  refuses to revisit a link it has already followed, so a server that stops
  advancing ends the stream instead of looping.
- **An empty page ends the stream, whatever the headers say.** A server that
  keeps naming a fresh next page and serving nothing would otherwise be polled
  forever.
- **A pagination link may not leave the API's origin.** A `Link` header is
  written by the server and your transport attaches credentials to whatever URL
  it is handed, so a next-page link pointing elsewhere is refused.

> Only `X-Total-Count` and `X-Result-Count` are declared by the v5 documents as
> *headers*. `Link` support is an accommodation for real deployments, where API
> gateways commonly add it and page by an opaque cursor.

## Following a reference

`Ref<T>` knows its target, so resolving one hands back a `T` with no turbofish
and no path string:

```rust
use rutmf::api::{Query, ResolveRef};

if let Some(reference) = &offering.product_specification {
    let spec = reference.resolve(client, &Query::new()).await?;  // ProductSpecification
}
```

It prefers the server-supplied `href` when there is one, so a reference into a
*different* API resolves against the API that owns it.

### References may not leave the origin either

An `href` is **payload data**. It is written by the server, and in a telco
integration it has usually passed through several systems before it reaches you.
Your transport attaches credentials to whatever URL it is handed — so if
`resolve` followed an `href` anywhere it pointed, every `…Ref` in every response
would be a place to put an attacker's host and collect a live bearer token.

So the same rule that governs pagination links governs references: same origin
by default, and a URL that leaves it raises `Error::CrossOrigin` rather than
being followed. Within a deployment the TM Forum APIs share a host and differ
only by *path*, so this refuses nothing that ordinarily happens.

Federation across hosts is still possible. It just has to be asked for, so that
trusting the other end is a decision someone made rather than a default:

```rust
use rutmf::api::{Error, Query, ResolveRef};

match reference.resolve(client, &Query::new()).await {
    Ok(spec) => { /* … */ }
    Err(Error::CrossOrigin { url, .. }) => {
        // Only if you would authenticate against `url` deliberately.
        let spec = reference.resolve_cross_origin(client, &Query::new()).await?;
    }
    Err(e) => return Err(e),
}
```

`TmfClient::get_cross_origin` is the same escape hatch one level down, and
`rutmf::api::same_origin` is the check itself, for a hand-written transport that
needs to answer the same question.

## Authentication

`transport-reqwest` ships the three schemes TMF deployments actually use:

```rust
use rutmf::api::{Auth, ClientCredentials, ReqwestTransport};

// A fixed token.
let transport = ReqwestTransport::builder()
    .auth(Auth::Bearer(token))
    .build()?;

// OAuth2 client credentials, the usual shape behind an API gateway.
// Tokens are cached and refreshed ahead of expiry, single-flight.
let transport = ReqwestTransport::builder()
    .auth(Auth::ClientCredentials(Box::new(
        ClientCredentials::new("https://idp.example/token", client_id, client_secret)
            .with_scopes(["catalog:read"]),
    )))
    .build()?;
```

**Secrets are redacted in `Debug`.** A transport is one `dbg!`, `tracing` span or
panic report away from an error tracker, so `Auth`, `ClientCredentials` and
`ReqwestTransport` all print placeholders instead of the token, the password, the
client secret, the cached token and the values of default headers:

```rust
println!("{transport:?}");
// ReqwestTransport { auth: Bearer(<redacted>), default_headers: [], .. }
```

## Retries

`RetryTransport` wraps any transport — including your own:

```rust
use std::time::Duration;
use rutmf::api::{ReqwestTransport, RetryPolicy, RetryTransport};

let transport = RetryTransport::new(
    ReqwestTransport::new()?,
    RetryPolicy::default()
        .max_retries(5)
        .base_delay(Duration::from_millis(200)),
);
```

It backs off exponentially with **full jitter**, honours `Retry-After` in both
its seconds and HTTP-date forms — measured against the server's own `Date`
header, so clock skew does not turn five seconds into an hour — and never
re-sends a `POST`, which is not idempotent.

One policy detail is worth knowing: `max_delay` bounds the *computed* backoff and
**not** the server's instruction. Clamping a `Retry-After: 60` down to a
ten-second ceiling would re-ask a rate-limited gateway while the limit is still in
force, spending the whole retry budget. A wait longer than `max_retry_after`
(60s by default) ends the retries and hands back the response, so you learn you
were throttled and for how long.

Backing off needs a timer, and the domain model has no runtime dependency, so
`Sleeper` is a trait. `transport-reqwest` supplies a `tokio` one; implement it
for `async-std`, `smol`, a wasm timer or a fake clock in a test.

## Errors

Failures surface as a typed enum. A server that answers with a TMF630 error body
gives you `code` and `reason` as data rather than as a formatted string:

```rust
match client.get_product_offering("7655", &Query::new()).await {
    Ok(offering) => { /* … */ }
    Err(e) if e.is_not_found() => { /* 404 */ }
    Err(e) if e.is_retryable() => { /* 408, 429, 5xx, or a transport failure */ }
    Err(e) => {
        if let Some(tmf) = e.tmf_error() {
            eprintln!("{:?}: {:?}", tmf.code, tmf.reason);
        }
    }
}
```

A body carrying neither `code` nor `reason` is reported as a raw status with the
text preserved, so a gateway's own error message is not thrown away by parsing it
into an empty TMF error.

### Writes that have not happened yet

Every v5 `POST` and `PATCH` declares `202 Accepted` with an empty body alongside
its synchronous answer, because a deployment may fulfil a write asynchronously.
That is neither a resource nor a failure, so it is neither fed to serde nor
silently swallowed:

```rust
match client.create_product_offering(&body).await {
    Ok(offering) => println!("created {:?}", offering.id),
    Err(e) if e.is_accepted() => println!("queued; poll {:?}", e.monitor()),
    Err(e) => return Err(e),
}
```

`monitor()` carries the URL the server named in `Location` or `Content-Location`,
when it named one.

## Events

TM Forum names every notification `{Resource}{Kind}Event`, so **neither end of an
event is stringly-typed**:

```rust
use rutmf::api::{HubCreate, HubOps};
use rutmf::core::EventKind;

// Subscribing: a misspelled `eventType` filter registers happily and then
// delivers nothing. This one cannot be misspelled.
client
    .register_listener(&HubCreate::for_resource::<ProductOffering>(
        "https://me/callback",
        EventKind::Create,
    ))
    .await?;

// In your webhook handler — no "productOffering" string to get wrong:
let offering: ProductOffering = event.resource()?.expect("a create event");
```

`TmfEvent` lives in `rutmf::core`, not the client layer: it is I/O-free wire
data, so a webhook handler or a queue consumer gets it without pulling in an
HTTP client.

Note what v5 does **not** offer. There is no `GET /hub`, so **no API lets you
list your subscriptions**, and only TMF621, TMF629, TMF639, TMF642 and TMF679 let
you read one back with `GET /hub/{id}`. Keep the id you were given at
registration — it is the only handle on the subscription you will get.

Generic Event Management (TMF688) is still v4-only, so what v5 ships is this
per-API hub rather than a central event bus.

### The kinds are the ones the specifications declare

Ten of `EventKind`'s variants appear across many APIs; two belong to one API
each, and are the ones a library written from the catalog documents alone would
miss:

| Kind | Declared by | Why it exists |
|---|---|---|
| `OperatingStatusChange` | TMF638 | `Service` carries both a `state` and an `operatingStatus`, and declares a listener for each — the only resource of the fourteen that separates an operational move from an administrative one |
| `Batch` | TMF637 | `ProductBatchEvent` carries an *array* of products, so read it with `event.payload::<Vec<Product>>("product")` rather than `event.resource()` |

A kind the crate does not know is an event nothing can subscribe to or raise, and
the failure is silent at both ends: the hub registers happily against a name no
server emits. So it is not a remembered list — `tests/coverage.rs` reads all
**157** `/listener/…` paths out of the fourteen documents and fails unless each
decomposes into a collection that API serves plus a kind `EventKind` names.

One upstream irregularity is recorded rather than papered over: TMF637 exposes
`ProductBatchEvent` at `/listener/productProductBatchEvent`. The doubling is in
the path only; `eventType` carries the class name.

## Not every client has the same methods

The five CRUD operations are the common case, not the rule. Eleven collections
across the covered specifications lack at least one, and TMF678 has none with
all five — so each client offers exactly what its specification declares:

```rust
// TMF678 declares no `POST /customerBill`, so there is no method for it.
// A bill is produced by a billing run; you ask for one out of cycle instead.
let request = bills.request_bill_on_demand(&on_demand).await?;

// And no `DELETE`, so `delete_customer_bill` does not exist either.
bills.update_customer_bill("CB-778", &update).await?;   // state, and that is all
```

This is checked, not just intended: `every_client_operation_is_declared_by_its_specification`
reads each client's operation macros back out of the source and compares them
against the vendored paths. A client cannot grow a method for an endpoint no
conformant server serves.

## Bringing your own transport

Clients are generic over a minimal trait, so cross-cutting concerns are ordinary
types rather than configuration:

```rust
use rutmf::api::{Result, TmfRequest, TmfResponse, Transport};

struct Traced<T> { inner: T, service: &'static str }

#[async_trait::async_trait]
impl<T: Transport> Transport for Traced<T> {
    async fn execute(&self, mut request: TmfRequest) -> Result<TmfResponse> {
        request.headers.insert("x-correlation-id", correlation_id());
        let outcome = self.inner.execute(request).await;
        // …log it…
        outcome
    }
}
```

Stack them in whichever order the behaviour needs: a `Traced<RetryTransport<_>>`
logs one line per call, a `RetryTransport<Traced<_>>` logs one per attempt.
