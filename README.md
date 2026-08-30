# rutmf

**Ergonomic, v5-first Rust types and clients for the TM Forum Open APIs.**

[![CI](https://github.com/hupe1980/rutmf/actions/workflows/ci.yml/badge.svg)](https://github.com/hupe1980/rutmf/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rutmf.svg)](https://crates.io/crates/rutmf)
[![Documentation](https://docs.rs/rutmf/badge.svg)](https://docs.rs/rutmf)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#-license)

📖 **[Guides and documentation](https://hupe1980.github.io/rutmf)** ·
📚 **[API reference](https://docs.rs/rutmf)**

> ⚠️ **Unofficial.** A community implementation of the publicly available,
> Apache-2.0-licensed TM Forum Open API specifications. Not affiliated with,
> endorsed by, or certified by TM Forum. "TM Forum" is their trademark.

```rust
use rutmf::product::ProductOffering;

let offering = ProductOffering::builder()
    .name("Business Internet")
    .is_sellable(true)
    .build();
```

```rust
use rutmf::api::{Query, tmf620::ProductCatalogClient};

let page = client
    .list_product_offerings(&Query::new().filter("lifecycleStatus", "Active").limit(20))
    .await?;
```

The domain model and the HTTP clients are separate: `rutmf::product` has no idea
HTTP exists, and `cargo add rutmf` pulls in no TLS stack.

## ✨ Design decisions

TM Forum's specifications leave a lot of room, and most of the work of a client
library is in the choices. These are the ones this crate makes.

**✍️ Create and update are different types, because v5 says so.** TMF v5 defines
each resource three times — the read model, an `_FVO` for `POST`, an `_MVO` for
`PATCH` — and they differ in which members exist and which are required.
Flattening the three into one all-optional struct leaves every mistake to be
caught by the server at runtime. Keeping them apart makes it a compile error:

```rust
use rutmf::product::ProductOfferingCreate;

// `name`, `lifecycle_status` and `last_update` are required on create.
// Leaving one out does not compile.
let body = ProductOfferingCreate::builder()
    .name("Business Internet")
    .lifecycle_status("Active")
    .last_update(chrono::Utc::now())
    .build();
```

`ProductOfferingUpdate` has the mirror-image property: `id`, `href` and
`lastUpdate` are server-owned, so they are absent from the type entirely.

**🔄 Payloads survive a round trip.** Real TMF deployments are full of vendor
extensions. Anything the crate has no field for is captured in `extensions`, in
document order, and re-emitted:

```rust
let json = r#"{"id":"7655","name":"Basic Firewall","@type":"ProductOffering","x-vendor":{"tier":2}}"#;
let offering: ProductOffering = serde_json::from_str(json)?;

assert_eq!(offering.extensions.get("x-vendor").unwrap()["tier"], 2);
assert_eq!(serde_json::to_value(&offering)?, serde_json::from_str::<serde_json::Value>(json)?);
```

Decoding then re-encoding is **lossless by value**: every input member with a
value comes back with an equal value, extension order is preserved, timestamps
keep their UTC offset, and **nothing is invented** — a payload that omits the
spec-mandatory `@type` comes back without one, because middleware must not add
members to what it relays.

One exception: an explicit `null` on a member the crate *models* reads as
absence and is not re-emitted — `Option<T>` has two states where that needs
three. A `null` on an unmodelled member round-trips through `extensions`. Where
the distinction is real — RFC 7386 makes `null` how a **merge patch removes a
member** — the `…Update` types say it:

```rust
let update = ProductOfferingUpdate::builder()
    .name("Business Internet")
    .build()
    .deleting("description");     // → {"name": "…", "description": null}
```

**🧬 A subclass is a value, not a string.** TMF v5 gives a characteristic's value
shape its own class — `IntegerCharacteristic`, `StringArrayCharacteristic`, ten
more — and the subclass *follows from the value*, so it is derived:

```rust
use rutmf::core::{Characteristic, ValueKind};

let speed = Characteristic::new("downstreamSpeed", 100);
assert_eq!(speed.type_name(), "IntegerCharacteristic");
assert_eq!(speed.value_kind(), ValueKind::Integer);
```

Six enumerations cover v5's seven polymorphic families, each with `all()` /
`from_type_name` / `type_name`, and `tests/coverage.rs` checks each against its
schema's `discriminator.mapping` in both directions.

**🔗 References know what they point at.** `Ref<ProductOffering>` rather than a
stringly-typed identifier, so `reference.resolve(&client, &query)` hands back a
`ProductOffering` with no turbofish and no path string.

**🔐 A server cannot redirect your credentials.** TMF payloads are full of URLs the
*server* wrote — the `href` of every `…Ref`, the `Link: rel="next"` of every paged
collection — and a transport attaches its bearer token to whatever URL it is
handed. Following those wherever they point would make any reference in any
response a place to put an attacker's host and collect a live token. So both are
checked against the client's own origin, and one that leaves it raises
`Error::CrossOrigin` instead:

```rust
// Within a deployment the TMF APIs share a host and differ by path, so this
// is the ordinary case and it just works.
let spec = reference.resolve(client, &Query::new()).await?;

// Federating across hosts stays possible — it has to be asked for, so that
// trusting the other end is a decision rather than a default.
let spec = reference.resolve_cross_origin(client, &Query::new()).await?;
```

**🔒 A `PATCH` cannot silently discard someone else's edit.** A TMF `PATCH` is
read-modify-write, so two operators editing different members of one order each
overwrite the other — with `200` to both, and nothing to say so. A read can carry
the `ETag` the server issued, and writing through that tag makes the write
conditional:

```rust
use rutmf::api::{Conditional, Tagged};

let held: Tagged<ProductOrder> = client.inner().fetch("42", &Query::new()).await?;
println!("{:?}", held.state);              // `Tagged<T>` derefs to `T`

// Refused with a 412 if anyone edited the order in between, rather than
// overwriting their change.
match held.update(client.inner(), &update).await {
    Err(e) if e.is_precondition_failed() => { /* re-read and decide */ }
    other => other?,
}
```

The v5 documents declare no request headers, so this is RFC 9110 rather than TMF
— a server that ignores the precondition answers as it would without one, and
`fetch` reports whether a tag was issued. The server layer here is the other end
of the same exchange.

**💰 Money is a decimal.** The v5 OAS types `Money.value` as `number/float`. Storing
money in binary floating point is a defect no matter what the schema says, so
values parse into `rust_decimal::Decimal` and re-emit as JSON numbers, keeping
integers integral.

**🏷️ A `PATCH` body cannot be labelled wrong.** TMF v5 declares four `PATCH` content
types and pairs each with a body schema. Passing the body and the content type
separately lets you send a combination every server rejects, so they are one type
here:

```rust
use rutmf::api::{JsonPatchOp, Patch};

client.update_product_offering("42", &update).await?;               // merge patch
client.update_product_offering("42", &ops).await?;                  // RFC 6902 list
client.update_product_offering("42", Patch::Query(&ops)).await?;    // TMF JSONPath
```

**🎁 A prelude of traits, not types.** `reference()`, `resolve()`, `fetch()` and
`register_listener()` each live on a trait, and Rust does not offer a trait
method until the trait is in scope. `use rutmf::prelude::*;` brings those in and
nothing else — concrete types stay in their domain module.

**🧪 A TMF server for your tests.** `MockTmfServer` implements the TMF630 collection
semantics in process — filtering with the comparison operators, sorting,
`fields=` projection, paging with count headers and a `206`, merge patch and
*atomic* RFC 6902 patch — behind the same `Transport` seam the real clients use:

```rust
let server = MockTmfServer::new();
server.seed("productOffering", serde_json::json!({
    "id": "7655", "name": "Basic Firewall", "@type": "ProductOffering",
}));

let client = ProductCatalogClient::new(server.base_url(), server.transport())?;
let page = client.list_product_offerings(&Query::new()).await?;
assert_eq!(page.total_count, Some(1));
```

→ [Full guides](https://hupe1980.github.io/rutmf/docs/), including queries,
pagination, retries, OAuth2, events and the server layer.

## 📦 Installation

```console
cargo add rutmf
```

Domain models only, no I/O. To call an API:

```console
cargo add rutmf --features api-tmf620,transport-reqwest
```

| Feature | Default | Enables |
|---|---|---|
| `party`, `customer`, `product`, `order`, `service`, `resource`, `ticket`, `alarm`, `bill`, `account` | ✅ | domain models (pure types, no I/O) |
| `api` | | transport-agnostic client layer |
| `transport-reqwest` | | ready-made `reqwest` transport, with OAuth2 client-credentials |
| `api-tmf620` … `api-tmf639` | | one per covered API, see below |
| `server` | | implement a TMF API: `ResourceStore` + `TmfHandler` |
| `server-axum` | | ready-made `axum` `Router` for the above |
| `mock` | | in-process TMF server for tests |
| `schemars` | | `JsonSchema` on every type |
| `full` | | everything above |

MSRV is **1.88**, verified in CI. The domain model builds for
`wasm32-unknown-unknown`.

## 🗺️ Coverage

Fourteen APIs are implemented end to end — domain types, client, mock, conformance
fixtures and schema coverage:

| API | Version | Resources |
|---|---|---|
| **TMF620** Product Catalog Management | v5.0.0 | `ProductOffering`, `ProductSpecification`, `ProductOfferingPrice`, `ProductCatalog`, `Category`, `ImportJob`, `ExportJob` |
| **TMF621** Trouble Ticket | v5.0.1 | `TroubleTicket`, `TroubleTicketSpecification` |
| **TMF622** Product Ordering | v5.0.0 | `ProductOrder`, `CancelProductOrder` |
| **TMF629** Customer Management | v5.0.1 | `Customer` |
| **TMF632** Party Management | v5.0.0 | `Individual`, `Organization` |
| **TMF669** Party Role Management | v5.0.0 | `PartyRole` (supplier / consumer / producer / business partner), `PartyRoleSpecification` |
| **TMF642** Alarm Management | v5.0.1 | `Alarm` + six task collections (ack, clear, comment, group, …) |
| **TMF666** Account Management | v5.0.0 | `Account` (billing / financial / party / settlement), `BillFormat`, `BillPresentationMedia`, `BillingCycleSpecification` |
| **TMF678** Customer Bill | v5.0.0 | `CustomerBill`, `CustomerBillOnDemand`, `AppliedCustomerBillingRate`, `BillCycle` |
| **TMF634** Resource Catalog Management | v5.0.0 | `ResourceCatalog`, `ResourceCategory`, `ResourceCandidate`, `ResourceSpecification`, `ImportJob`, `ExportJob` |
| **TMF637** Product Inventory Management | v5.0.0 | `Product` |
| **TMF679** Product Offering Qualification | v5.0.0 | `CheckProductOfferingQualification`, `QueryProductOfferingQualification` |
| **TMF638** Service Inventory Management | v5.0.0 | `Service` |
| **TMF639** Resource Inventory Management | v5.0.0 | `Resource` (four subclasses), `ResourceGraph` |

Together these close the loop from shelf to network: browse a catalog (TMF620),
identify who is buying (TMF632) and in what capacity (TMF669), engage them as a
customer (TMF629), place the order (TMF622) — then find what the customer has
(TMF637), what delivers it (TMF638) and what that runs on (TMF639). TMF634 is the
catalog behind that last step, and TMF621/TMF642 are what you raise and what the
network reports when any of it breaks.

Every CRUD resource carries the read / `Create` / `Update` triple and is
member-complete against its v5 schema. Every client implements `HubOps` for
event subscriptions.

ℹ️ **TMF641 Service Ordering and TMF688 Event Management are deliberately absent**:
neither has a v5 release, and this crate models v5 rather than pretending
otherwise.

## 🏗️ Serving a TMF API

The other direction: you have the data, and you need to expose it as a conformant
TM Forum API. Implement five methods — none of them about HTTP — and `TmfHandler`
supplies TMF630 routing, filtering, sorting, `fields=` projection, paging with
count headers — offset/limit *and* the cursor pagination TMF621 and TMF639
declare — all four `PATCH` content types, `ETag` with `If-Match` on writes and
`If-None-Match` on reads, error bodies and status codes. Add
`.with_max_page_size(100)` and an unbounded collection `GET` stops being
something a caller can ask for.

```rust
#[async_trait::async_trait]
impl ResourceStore for Catalog {
    async fn list(&self, _c: &str, selection: &Selection) -> StoreResult<Matched> {
        // A SQL store turns the selection into WHERE / ORDER BY / LIMIT.
        Ok(selection.apply(self.offerings.lock().unwrap().clone()))
    }
    // get, create, replace, delete — and that is the whole trait.
}

let app = axum::Router::new().nest(
    "/tmf-api/productCatalogManagement/v5",
    rutmf::server::router(TmfHandler::new(base_url, catalog)),
);
```

It serves `/hub` and **raises the notifications** a conformant server owes: a
write through the API becomes the right `{Resource}{Kind}Event`, matched against
each subscription's filter and addressed to its `/listener/{eventName}` URL. The
`Notifier` seam is the one part a deployment must supply — whether delivery is a
blocking `POST`, a queue publish or a retry loop.

Getting the *right* event name is not the one-liner it looks like. TMF621 and
TMF634 raise `…StatusChangeEvent` where the other twelve raise
`…StateChangeEvent` — and TMF634 does so over a member it still spells
`lifecycleStatus`, so the difference cannot be read off the resource. TMF638
separates an operational move (`serviceOperatingStatusChangeEvent`) from an
administrative one. A server that guesses raises names nobody has subscribed to,
and the failure is silent at both ends. So the spelling is read out of the
vendored `/listener/…` paths, and `tests/coverage.rs` fails if all **157** of
them are not reproducible from the collection and the kind.

`MockTmfServer` *is* `TmfHandler<MemoryStore>` with a `Notifier` that records
instead of sending, so the test double and a real deployment run the same code
and cannot drift.

→ [Serving an API](https://hupe1980.github.io/rutmf/docs/serving-an-api/)

## 🔬 Conformance

Three suites, because each is blind where the others see.

**Against the examples.** `tests/fixtures/` holds **591 `components.examples`
values** vendored verbatim from the TM Forum [specification repositories][tmf-repos]
(Apache-2.0). Every one must parse and round-trip, and a separate test fails if
any fixture falls outside the mapping, so a new one cannot be a silently untested
one.

**Against the schemas.** Round-tripping proves nothing about coverage: an unknown
member survives in `extensions` whether or not the model understands it. So this
suite reads the vendored OpenAPI documents and checks **215 Rust types against
462 v5 schemas** — member presence in both directions, member types, enumeration
values, requiredness, discriminator values against each schema's own
`discriminator.mapping`, that every reference names the class the spec declares,
that a member with a closed vocabulary is an `enum` rather than a `String`, that
a `date-time` member is a `Timestamp`, and that the fourteen specifications do not
disagree about a shared type.

**Against the schemas, in reverse.** "Every Rust type maps to a schema" is
satisfied by a crate modelling three schemas out of three thousand, so the suite
also asks the opposite: **every schema the fourteen documents declare must be
modelled, or named in `NOT_MODELLED` with the reason.** Nothing may be merely
absent. What that leaves out is the value arms of `PlaceRefOrValue`
(TMF673/674/675 geography and its GeoJSON geometry) and `IntentRefOrValue`
(TMF921) — APIs outside the fourteen, where the crate models the *reference* arm
and a value arm round-trips through `extensions`.

**Against a running server.** The other two check documents. This one reads each
vendored specification, discovers the collections and methods it declares, and
drives the server layer over a real socket for **all 43** — asserting the status
codes, the count headers, the `ETag`, and that a missing resource is a `404`
with a TMF630 body. It is spec-driven, so a new API extends it with no new test
code.

```console
cargo test --test conformance
cargo test --test coverage --features schemars
cargo test --test server_conformance --features server-axum,transport-reqwest
```

`cargo deny check` runs in CI over the full dependency tree. Only `rustls` is
enabled as a TLS backend, and `openssl-sys` is banned outright.

[tmf-repos]: https://github.com/tmforum-apis

## 🚀 Examples

```console
cargo run --example catalog_sync        --features api-tmf620,mock
cargo run --example customer_onboarding --features api-tmf629,api-tmf632,mock
cargo run --example order_lifecycle     --features api-tmf620,api-tmf622,mock
cargo run --example inventory_chain     --features api-tmf634,api-tmf637,api-tmf638,api-tmf639,mock
cargo run --example assurance_workflow  --features api-tmf621,api-tmf642,api-tmf678,mock
cargo run --example serve_catalog       --features server-axum,api-tmf620,transport-reqwest
```

- **`catalog_sync`** streams a catalog and creates an offering.
- **`customer_onboarding`** walks the TMF632 → TMF629 split.
- **`order_lifecycle`** runs catalog → order → fulfilment → cancellation, and
  shows a stale `PATCH` being refused instead of quietly discarding another
  operator's edit.
- **`inventory_chain`** goes specification → resource → service → product.
- **`assurance_workflow`** is the loop *after* delivery: an alarm fires, the NOC
  acknowledges it, a ticket is raised and resolved, the alarm clears, and the
  bill is settled.
- **`serve_catalog`** implements a TMF620 server and calls it with this crate's
  own client.

💡 The first five need no network — they run against the in-process test server.

## 🚧 Status

**0.1 — early.** The design is settled and proven against the specifications, and
both directions work: calling a TM Forum API and serving one. The API surface
will still change before 1.0. Pin an exact version.

Four things stand between here and 1.0: a release cycle of real use with the
module layout held still, conformance against TM Forum's own CTK (which needs
membership — the kit is not publicly pullable), one API modelled by somebody
else, and a settled decision on the `#[non_exhaustive]` trade-off. See
[status and stability](https://hupe1980.github.io/rutmf/docs/coverage/#status-and-stability).

## 🙏 Credit

The specifications themselves are published by [TM Forum][tmf] under Apache-2.0,
and the vendored documents and examples under `specs/` and `tests/fixtures/` are
their work.

[tmf]: https://www.tmforum.org/oda/open-apis/

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Adding an API is a tractable first
contribution: the coverage suite's failure output *is* the to-do list.

## 📜 License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Vendored TM Forum specification material under `specs/` and
`tests/fixtures/` remains under its original Apache-2.0 license and copyright.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
