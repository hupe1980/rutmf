+++
title = "rutmf"
description = "Ergonomic, v5-first Rust types and clients for the TM Forum Open APIs. Typed references, real decimals, guaranteed round-trip fidelity, and a conformance suite over 591 official specification examples."
template = "index.html"

[extra]
lede = "A v5-first Rust crate for the TM Forum Open APIs: SID-aligned domain types with the HTTP clients layered strictly on top. Create and update are different types because the specification says so, money is a decimal, references know what they point at — and every claim here is checked against the official documents in CI."

[[extra.features]]
title = "Create and update are different types"
body = "TMF v5 defines each resource three times — read, `_FVO` for `POST`, `_MVO` for `PATCH`. Flattening the three into one all-optional struct leaves every mistake to the server. Here a missing required member is a compile error, and a server-owned field is absent from the type entirely."

[[extra.features]]
title = "Payloads survive a round trip"
body = "Decode then re-encode is lossless by value. Vendor extensions are kept in document order, timestamps keep their UTC offset, and nothing is invented — a payload that omits `@type` comes back without one."

[[extra.features]]
title = "References are typed"
body = "`Ref<ProductOffering>` rather than a stringly-typed identifier. The compiler tracks what a reference points at, and resolving one returns the right type with no turbofish and no path string."

[[extra.features]]
title = "A PATCH cannot discard someone else's edit"
body = "A TMF `PATCH` is read-modify-write, so two clients editing different members of one order each overwrite the other — both get `200`, and nothing says so. Read a resource with the `ETag` the server issued and write back through it, and a concurrent edit is a `412` instead."

[[extra.features]]
title = "Money is a decimal"
body = "The v5 schema types `Money.value` as a float. Storing money in binary floating point is a defect whatever the schema says, so values parse into `rust_decimal::Decimal` and re-emit as JSON numbers, keeping integers integral."

[[extra.features]]
title = "The transport is yours"
body = "Clients are generic over a minimal `Transport` trait. `reqwest` is one line away behind a feature flag, with OAuth2 client-credentials, retries and full-jitter backoff — or bring a `tower` stack, or a fake."

[[extra.features]]
title = "Serve an API, not just call one"
body = "Implement five storage methods, none of them about HTTP, and get TMF630 routing, filtering, paging, projection, the four `PATCH` flavours, `ETag` with `If-Match` and `If-None-Match`, and the `/hub` notifications a conformant server owes."

[[extra.apis]]
id = "TMF620"
name = "Product Catalog Management"
version = "v5.0.0"
resources = "ProductOffering, ProductSpecification, ProductOfferingPrice, ProductCatalog, Category, ImportJob, ExportJob"

[[extra.apis]]
id = "TMF621"
name = "Trouble Ticket"
version = "v5.0.1"
resources = "TroubleTicket, TroubleTicketSpecification"

[[extra.apis]]
id = "TMF622"
name = "Product Ordering"
version = "v5.0.0"
resources = "ProductOrder, CancelProductOrder"

[[extra.apis]]
id = "TMF629"
name = "Customer Management"
version = "v5.0.1"
resources = "Customer"

[[extra.apis]]
id = "TMF632"
name = "Party Management"
version = "v5.0.0"
resources = "Individual, Organization"

[[extra.apis]]
id = "TMF669"
name = "Party Role Management"
version = "v5.0.0"
resources = "PartyRole (supplier, consumer, producer, business partner), PartyRoleSpecification"

[[extra.apis]]
id = "TMF634"
name = "Resource Catalog Management"
version = "v5.0.0"
resources = "ResourceCatalog, ResourceCategory, ResourceCandidate, ResourceSpecification, ImportJob, ExportJob"

[[extra.apis]]
id = "TMF642"
name = "Alarm Management"
version = "v5.0.1"
resources = "Alarm, AckAlarm, UnAckAlarm, ClearAlarm, CommentAlarm, GroupAlarm, UnGroupAlarm"

[[extra.apis]]
id = "TMF666"
name = "Account Management"
version = "v5.0.0"
resources = "Account (billing, financial, party, settlement), BillFormat, BillPresentationMedia, BillingCycleSpecification"

[[extra.apis]]
id = "TMF678"
name = "Customer Bill"
version = "v5.0.0"
resources = "CustomerBill, CustomerBillOnDemand, AppliedCustomerBillingRate, BillCycle"

[[extra.apis]]
id = "TMF637"
name = "Product Inventory Management"
version = "v5.0.0"
resources = "Product"

[[extra.apis]]
id = "TMF679"
name = "Product Offering Qualification"
version = "v5.0.0"
resources = "CheckProductOfferingQualification, QueryProductOfferingQualification"

[[extra.apis]]
id = "TMF638"
name = "Service Inventory Management"
version = "v5.0.0"
resources = "Service"

[[extra.apis]]
id = "TMF639"
name = "Resource Inventory Management"
version = "v5.0.0"
resources = "Resource (four subclasses), ResourceGraph"
+++

## Built for integration work

TM Forum's Open APIs are the contract between OSS/BSS systems, and code that
speaks them lives or dies on details: a vendor extension that must survive a
round trip, a price that must not be a float, a `PATCH` body that must match the
content type it was sent under.

`rutmf` puts those details in the type system, then proves it. The domain model
is checked member by member against the vendored v5 OpenAPI documents, and every
official example in those documents must parse and re-serialise unchanged —
both on every commit.

```rust
use rutmf::api::{Query, tmf620::ProductCatalogClient};
use rutmf::product::ProductOfferingCreate;

let client = ProductCatalogClient::new(
    "https://mycsp.com/tmf-api/productCatalogManagement/v5",
    transport,
)?;

// `name`, `lifecycle_status` and `last_update` are required on create.
// Leaving one out does not compile.
let offering = client.create_product_offering(
    &ProductOfferingCreate::builder()
        .name("Business Internet")
        .lifecycle_status("Active")
        .last_update(chrono::Utc::now())
        .build(),
).await?;
```

Together the fourteen APIs close the loop from shelf to network: browse a catalog,
identify who is buying, engage them as a customer, place the order — then find
what the customer actually has, what delivers it, and what that runs on.
