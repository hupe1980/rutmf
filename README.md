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

## Working with TMF v5

- Create and update builders follow the fields each operation accepts. Typed references identify the resource they resolve to.
- Vendor extensions retain their key order. Timestamps preserve their UTC offset. `Money` reads decimal numbers or numeric strings and emits exact JSON numbers.
- Clients support pagination, same-origin reference resolution and conditional writes with ETags. `Error::problem()` exposes RFC 9457 details and extension members. `Error::tmf_error()` exposes TMF630 errors.
- Use `MockTmfServer` to exercise client calls against an in-process server.

[Domain model](https://hupe1980.github.io/rutmf/docs/domain-model/) · [Client guide](https://hupe1980.github.io/rutmf/docs/calling-an-api/) · [Testing guide](https://hupe1980.github.io/rutmf/docs/testing/)

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
| `party`, `customer`, `product`, `order`, `service`, `resource`, `ticket`, `alarm`, `bill`, `account`, `usage` | ✅ | domain models (pure types, no I/O) |
| `api` | | transport-agnostic client layer |
| `transport-reqwest` | | ready-made `reqwest` transport, with OAuth2 client-credentials |
| `api-tmf620` … `api-tmf679` | | one per covered API, see below |
| `server` | | implement a TMF API: `ResourceStore` + `TmfHandler` |
| `server-axum` | | ready-made `axum` `Router` for the above |
| `mock` | | in-process TMF server for tests |
| `schemars` | | `JsonSchema` on every type |
| `full` | | everything above |

MSRV is **1.88**, verified in CI. The domain model builds for
`wasm32-unknown-unknown`.

### Hashing or signing JSON

rutmf enables serde_json’s `preserve_order` and `arbitrary_precision` features.
Cargo unifies features for the same serde_json package across normal dependencies,
so other `serde_json::Value` objects in your application also retain insertion
order and numeric spellings. If your digest contract uses lexically sorted keys,
apply the same rule before sealing and after loading JSON from storage.

```rust
let mut document: serde_json::Value = serde_json::from_str(r#"{"z":[{"y":1,"x":2}],"a":0}"#)?;
document.sort_all_objects();
let bytes = serde_json::to_vec(&document)?;
assert_eq!(bytes, br#"{"a":0,"z":[{"x":2,"y":1}]}"#);
```

This sorts object keys recursively and retains array order. Agree on number
formatting too when independent producers or a database can rewrite numbers.

## 🗺️ Coverage

Fifteen APIs are implemented end to end — domain types, client, mock, conformance
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
| **TMF635** Product Usage Management | v5.0.0 | `ProductUsage` (rated usage, prices on the record), `ExportJob` |
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
event subscriptions, and the six whose API lets you read a subscription back
also implement `HubLookup`. Each `rutmf::api::tmfXXX` module publishes its
declared surface as `API` — every collection and the operations on it, held
equal to the specification document by the test suite.

ℹ️ **TMF641 Service Ordering and TMF688 Event Management are deliberately absent**:
neither has a v5 release, and this crate models v5 rather than pretending
otherwise.

## 🏗️ Serving a TMF API

Implement `ResourceStore` and give `TmfHandler` the API to serve. The handler provides routing, paging, filtering, projection, conditional requests and JSON Patch.

```rust
#[async_trait::async_trait]
impl ResourceStore for Catalog {
    async fn list(&self, _c: &str, selection: &Selection) -> StoreResult<Matched> {
        // A SQL store turns the selection into WHERE / ORDER BY / LIMIT.
        Ok(selection.apply(self.offerings.lock().unwrap().clone()))
    }
    // get, create, replace, delete — and that is the whole trait.
}

let handler = TmfHandler::for_host(&tmf620::API, "https://mycsp.com", catalog);
let app = axum::Router::new().nest(
    &format!("/{}", tmf620::API.path),
    rutmf::server::router(handler),
);
```

Undeclared collections return HTTP 404. Unsupported methods return HTTP 405 with an `Allow` header. Configure a `Notifier` to deliver hub events. [Serving an API](https://hupe1980.github.io/rutmf/docs/serving-an-api/)

## 🔬 Conformance

The test suite checks 610 official examples, maps 219 Rust types to 490 schemas, and exercises server responses against the operations declared by all 15 supported APIs. See the [coverage guide](https://hupe1980.github.io/rutmf/docs/coverage/) for the scope of those checks.

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

## Stability

During 0.x, minor releases may change public APIs. See the [changelog](CHANGELOG.md) before upgrading.

## 🙏 Credit

The specifications themselves are published by [TM Forum][tmf] under Apache-2.0,
and the OpenAPI documents the test suites read and the examples vendored under
`tests/fixtures/` are their work.

[tmf]: https://www.tmforum.org/oda/open-apis/

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Adding an API is a tractable first
contribution: the coverage suite's failure output *is* the to-do list.

## 📜 License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Vendored TM Forum specification material under `tests/fixtures/`
remains under its original Apache-2.0 license and copyright.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
