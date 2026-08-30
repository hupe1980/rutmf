+++
title = "Getting started"
description = "Install rutmf, pick the feature flags you need, build your first TM Forum v5 resource and make your first API call."
weight = 10
+++

## Install

```console
cargo add rutmf
```

That gives you the domain model and nothing else — no HTTP client, no TLS stack,
no async runtime. `rutmf::product` does not know that HTTP exists.

To call an API, add the client for it and a transport:

```console
cargo add rutmf --features api-tmf620,transport-reqwest
```

The crate targets Rust **1.88** or later, verified in CI rather than asserted.

## Your first resource

Every entity is built through a `bon` builder. Setters take `impl Into<_>`, so
you rarely have to name a type:

```rust
use rutmf::product::ProductOffering;

let offering = ProductOffering::builder()
    .name("Business Internet")
    .description("Symmetric fibre for small sites")
    .is_sellable(true)
    .build();

assert_eq!(offering.type_name(), "ProductOffering");
```

Anything the crate builds declares its `@type`, because a request without one is
the request a conformant server rejects.

## Your first call

A client wraps a base URL and a transport. The base URL includes the API root —
the part TM Forum fixes per API, ending in the version segment:

```rust
use rutmf::api::{Query, ReqwestTransport, tmf620::ProductCatalogClient};

let client = ProductCatalogClient::new(
    "https://mycsp.com/tmf-api/productCatalogManagement/v5",
    ReqwestTransport::new()?,
)?;

let page = client
    .list_product_offerings(
        &Query::new()
            .filter("lifecycleStatus", "Active")
            .fields(["id", "name", "lifecycleStatus"])
            .limit(20),
    )
    .await?;

for offering in page {
    println!("{:?}", offering.name);
}
```

If you would rather not repeat the API root, `from_host` appends the conventional
one for you:

```rust
let client = ProductCatalogClient::from_host("https://mycsp.com", transport)?;
```

### The prelude, for the methods that live on traits

Several of the calls you reach for first are trait methods —
`offering.reference()`, `reference.resolve(…)`, `client.inner().fetch(…)`,
`client.register_listener(…)` — and Rust does not offer a trait method until the
trait is in scope. So:

```rust
use rutmf::prelude::*;
```

Traits and `Query`, nothing else. Concrete types stay where they are:
`rutmf::product::ProductOffering` says which API it belongs to, and several of
the fourteen declare a `Product` or a `Category` of their own.

## Choosing features

Domain models are on by default and pull in no I/O. Everything that does
I/O — clients, transports, the server layer — is opt-in, so `cargo add rutmf`
never drags a TLS stack into your dependency graph by surprise.

| Feature | Default | Enables |
|---|---|---|
| `party`, `customer`, `product`, `order`, `service`, `resource`, `ticket`, `alarm`, `bill`, `account` | ✅ | domain models (pure types, no I/O) |
| `api` | | the transport-agnostic client layer |
| `transport-reqwest` | | a ready-made `reqwest` transport, with OAuth2 and retries |
| `api-tmf620` | | Product Catalog Management client |
| `api-tmf621` | | Trouble Ticket client |
| `api-tmf622` | | Product Ordering client |
| `api-tmf629` | | Customer Management client |
| `api-tmf632` | | Party Management client |
| `api-tmf634` | | Resource Catalog Management client |
| `api-tmf637` | | Product Inventory Management client |
| `api-tmf638` | | Service Inventory Management client |
| `api-tmf639` | | Resource Inventory Management client |
| `api-tmf642` | | Alarm Management client |
| `api-tmf666` | | Account Management client |
| `api-tmf678` | | Customer Bill client |
| `server` | | implement a TMF API: `ResourceStore` + `TmfHandler` |
| `server-axum` | | a ready-made `axum` `Router` for the above |
| `mock` | | an in-process TMF server for tests |
| `schemars` | | `JsonSchema` on every type |
| `full` | | everything above |

An `api-*` feature pulls in the domain it needs, so
`features = ["api-tmf620"]` is enough to get `rutmf::product` with it.

The domain model builds for `wasm32-unknown-unknown`, which CI checks — useful if
you are validating TMF payloads in a browser or an edge worker.

## Running the examples

Six runnable examples ship with the crate. The first five need no network: they
run against the in-process test server.

```console
cargo run --example catalog_sync        --features api-tmf620,mock
cargo run --example customer_onboarding --features api-tmf629,api-tmf632,mock
cargo run --example order_lifecycle     --features api-tmf620,api-tmf622,mock
cargo run --example inventory_chain     --features api-tmf634,api-tmf637,api-tmf638,api-tmf639,mock
cargo run --example assurance_workflow  --features api-tmf621,api-tmf642,api-tmf678,mock
cargo run --example serve_catalog       --features server-axum,api-tmf620,transport-reqwest
```

- **`catalog_sync`** streams a catalog and creates an offering.
- **`customer_onboarding`** walks the TMF632 → TMF629 split: create a party,
  then engage it as a customer.
- **`order_lifecycle`** runs catalog → order → fulfilment → cancellation.
- **`inventory_chain`** walks the other direction — specification → resource →
  service → product — and shows a port alarm degrading a service without
  changing what the customer has.
- **`assurance_workflow`** is the loop *after* delivery: an alarm fires, the NOC
  acknowledges it, a trouble ticket is raised and resolved, the alarm clears,
  and the bill is settled. It is the clearest place to see the two shapes the
  assurance APIs use that the commerce ones do not — operations modelled as
  their own collections, and resources that are deliberately read-only.
- **`serve_catalog`** implements a TMF620 server, serves it on a real port, and
  calls it with this crate's own client.

## Where to go next

- [The domain model](@/docs/domain-model.md) — how v5 resources become Rust types.
- [Calling an API](@/docs/calling-an-api.md) — queries, patches, paging, retries, events.
- [Serving an API](@/docs/serving-an-api.md) — the other direction.
