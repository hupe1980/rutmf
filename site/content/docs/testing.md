+++
title = "Testing"
description = "Write tests against a real TM Forum server without a socket: MockTmfServer implements TMF630 filtering, sorting, paging, projection and both patch formats in process."
weight = 50
+++

## An in-process TMF server

Testing integration code against TMF APIs usually means one of two bad options:
hand-maintained JSON fixtures that drift from what the server actually does, or a
container you have to start.

`MockTmfServer` is the third: a real implementation of the TMF630 collection
semantics, in process, behind the same `Transport` seam the real clients use.

```rust
use rutmf::api::{Query, tmf620::ProductCatalogClient};
use rutmf::mock::MockTmfServer;

let server = MockTmfServer::new();
server.seed("productOffering", serde_json::json!({
    "id": "7655", "name": "Basic Firewall", "@type": "ProductOffering",
}));

// The server owns its base URL, so there is no string to keep in step.
let client = ProductCatalogClient::new(server.base_url(), server.transport())?;

let page = client.list_product_offerings(&Query::new()).await?;
assert_eq!(page.total_count, Some(1));
```

No socket, no port allocation, no teardown.

## What it actually implements

Everything [the server layer](@/docs/serving-an-api.md) does, because it *is*
the server layer — `MockTmfServer` is `TmfHandler<MemoryStore>` with a
`Transport` shim in front of it:

- attribute filtering with the TMF630 comparison operators, comma-separated
  value lists, dotted paths and collection traversal;
- `sort=`, including multiple keys and the `-` prefix;
- `fields=` projection, including dotted names that narrow a nested member;
- `offset`/`limit` paging with `X-Total-Count` and `X-Result-Count`, and a `206`
  for a partial page;
- merge patch and **atomic** RFC 6902 JSON Patch, with `add` and `replace`
  behaving differently on arrays as the RFC requires;
- `ETag`, `If-Match` on writes and `If-None-Match` on reads, so you can test
  optimistic concurrency and conditional polling against a real `412` and a real
  `304` rather than a hand-stubbed one;
- `/hub` subscriptions, and the change events a write raises against them;
- TMF630 error bodies and the right status code for each outcome.

Routing keys off the API version segment, so it serves collections this crate has
no typed client for. If you need a `TMF666 billingAccount` endpoint for one test,
seed it and call it.

## Testing event subscriptions

A write through the API raises its own event — the handler names it, matches it
against each subscription's TMF630 filter and works out the callback URL. The
mock is a `Notifier` that **records rather than delivers**, so a test can assert
what would have gone out without standing up a callback endpoint:

```rust
use rutmf::api::{HubCreate, HubOps};
use rutmf::core::EventKind;

client
    .register_listener(&HubCreate::for_resource::<ProductOffering>(
        "https://me/callback",
        EventKind::Create,
    ))
    .await?;

client.create_product_offering(&body).await?;

let delivered = server.notifications();
assert_eq!(delivered.len(), 1);
assert_eq!(delivered[0].event_type, "ProductOfferingCreateEvent");
assert_eq!(
    delivered[0].delivery_url(),
    "https://me/callback/listener/productOfferingCreateEvent",
);
```

## What it is honest about not being

- **No schema validation.** It stores what you send. If you want requiredness
  enforced, deserialise into the `…Create` type first — that is what it is for.
- **No lifecycle rules.** It will happily move an order from `completed` back to
  `acknowledged`. Business rules belong to whoever owns the business.
- **A wildcard matcher, not a regex engine.** The `.regex` filter operator
  supports `*` and `?`; anything else simply does not match. Pulling a regex
  engine into a test double would be the tail wagging the dog.
- **Notifications are recorded, not delivered.** There is no HTTP callback.

## Testing a store over a real socket

When you are implementing a `ResourceStore` and want to exercise the whole stack
— router, serialisation, headers, status codes — serve it on an ephemeral port
and point a real client at it:

```rust
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
let port = listener.local_addr()?.port();
let base_url = format!("http://127.0.0.1:{port}/tmf-api/productCatalogManagement/v5");

let app = axum::Router::new().nest(
    "/tmf-api/productCatalogManagement/v5",
    rutmf::server::router(TmfHandler::new(&base_url, store)),
);
tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

let client = ProductCatalogClient::new(base_url, ReqwestTransport::new()?)?;
```

This is how the crate tests its own server layer, and it is worth copying: if the
two ends disagree about TMF630, the test fails, because each end is the other's
oracle.

## The suites that back all this

The crate is verified by three suites that are blind in different places, and
all run on every commit:

**Against the official examples.** All **591** `components.examples` values are
vendored verbatim from the TM Forum specification repositories, and every one
must parse and round-trip by value. A separate test fails if any fixture falls
outside the classification rules, so a newly vendored example cannot be a
silently untested one.

**Against the schemas.** Round-tripping proves nothing about *coverage* — an
unknown member survives in `extensions` whether or not the model understands it.
So a second suite reads the OpenAPI documents and checks **215 Rust types against
462 v5 schemas**: that every specified member has a typed field and every typed
field is specified, that each member has the shape the spec gives it, that
enumerations admit exactly the specified values, that requiredness matches, that
discriminators are the values each schema's own `discriminator.mapping` names,
that every typed reference names the class the specification declares, that a
member the spec gives a closed vocabulary is a Rust `enum` rather than a
`String`, that a `date-time` member is a `Timestamp`, and that the fourteen
specifications do not disagree about a shared type.

**Against the schemas, in the other direction.** All of the above asks "does
every Rust type match a schema?", which says nothing about completeness — a crate
modelling three schemas out of three thousand passes it. So
`every_declared_schema_is_modelled_or_excused` asks the reverse: every schema the
fourteen documents declare must be mapped to a Rust type, absorbed into a mapped
schema through `allOf`, handled generically (an event, a `…Ref`, a write
variant), paired in the enumeration table, or listed in `NOT_MODELLED` **with the
reason it is not modelled**. The list is checked both ways, so an excuse for a
schema TM Forum has since removed fails too.

What that leaves excused is the value arms of `PlaceRefOrValue` (TMF673/674/675
geography and its GeoJSON geometry) and `IntentRefOrValue` (TMF921) — APIs
outside the fourteen, where the crate models the *reference* arm and a value arm
round-trips through `extensions`. Everything reachable from a modelled resource
by a typed path is modelled.

**Against the schemas, about behaviour rather than shape.** The same suite also
checks the parts of an API that are not a schema at all, because those are where
a library is wrong in ways nothing else notices:

- every operation a client exposes is one its specification declares, so a client
  cannot grow a `create_customer_bill` against a `POST` that does not exist;
- every query parameter a specification declares can be built with `Query`, and
  only `fields`, `offset` and `limit` are claimed to be universal — `sort`,
  `filter`, `after` and `before` are declared by TMF621 and TMF639 alone;
- every one of the **157** `/listener/…` endpoints decomposes into a collection
  that API serves plus an `EventKind` the crate names, so an event it can neither
  subscribe to nor raise is a failure rather than a silence;
- the collections spelling a lifecycle move `…StatusChangeEvent` are the ones the
  documents declare it for — a server raising the other spelling delivers nothing
  to a correctly registered subscriber, and says nothing about it.

**Against a running server.** The first two check documents; a document cannot
tell you whether the code answers a request correctly. So a third suite reads
each vendored specification, discovers the collections and methods it declares,
and drives the server layer over a real socket for **all 43** of them —
asserting the status codes, the `X-Total-Count`/`X-Result-Count` headers, the
`ETag`, and that a missing resource is a `404` carrying a TMF630 error body.

`tests/server.rs` adds the exchanges needing *both* halves right at once: a
conditional `PATCH` that must refuse to discard a concurrent edit, and a
conditional `GET` that must answer `304`. Each step is a place the two ends can
disagree in a way that looks like success, so they are asserted end to end.

It is spec-driven: vendoring a fifteenth API extends it without a line being
added. It is also the nearest thing to Conformance Test Kit evidence the project
can produce for itself — the real CTK is distributed through TM Forum's own
channels, so running it needs membership rather than engineering.

```console
cargo test --test conformance
cargo test --test coverage --features schemars
cargo test --test server_conformance --features server-axum,transport-reqwest
```

Between them these have caught a v4 member name surviving in a v5 type, some 250
members with no typed field, members typed `String` where the spec says `array`,
references stamped with the wrong class, ten members whose closed vocabulary was
typed as a `String`, and reference shapes that could not be parsed at all — every
one of them while the round-trip suite was green.

The pattern is worth naming, because it is how the suite grew: **each new *kind*
of check finds defects every existing check was blind to.** The reference gate
found five wrong discriminators in shipped code; the enumeration gate, written
the same way a day later, found ten more. Both had been passing every other test
in the project.
