+++
title = "API coverage"
description = "Which TM Forum Open APIs rutmf implements, at which v5 version, and which are deliberately absent because TM Forum has not published a v5."
weight = 60
+++

Fourteen APIs are implemented end to end — domain types, client, test server,
conformance fixtures and schema coverage.

| API | Version | Resources |
|---|---|---|
| **TMF620** Product Catalog Management | v5.0.0 | `ProductOffering`, `ProductSpecification`, `ProductOfferingPrice`, `ProductCatalog`, `Category`, `ImportJob`, `ExportJob` |
| **TMF621** Trouble Ticket | v5.0.1 | `TroubleTicket`, `TroubleTicketSpecification` |
| **TMF622** Product Ordering | v5.0.0 | `ProductOrder`, `CancelProductOrder` |
| **TMF629** Customer Management | v5.0.1 | `Customer` |
| **TMF632** Party Management | v5.0.0 | `Individual`, `Organization` |
| **TMF669** Party Role Management | v5.0.0 | `PartyRole` (supplier / consumer / producer / business partner), `PartyRoleSpecification` |
| **TMF634** Resource Catalog Management | v5.0.0 | `ResourceCatalog`, `ResourceCategory`, `ResourceCandidate`, `ResourceSpecification`, `ImportJob`, `ExportJob` |
| **TMF637** Product Inventory Management | v5.0.0 | `Product` |
| **TMF679** Product Offering Qualification | v5.0.0 | `CheckProductOfferingQualification`, `QueryProductOfferingQualification` |
| **TMF642** Alarm Management | v5.0.1 | `Alarm` + six task collections |
| **TMF666** Account Management | v5.0.0 | `Account` (four subclasses), `BillFormat`, `BillPresentationMedia`, `BillingCycleSpecification` |
| **TMF678** Customer Bill | v5.0.0 | `CustomerBill`, `CustomerBillOnDemand`, `AppliedCustomerBillingRate`, `BillCycle` |
| **TMF638** Service Inventory Management | v5.0.0 | `Service` |
| **TMF639** Resource Inventory Management | v5.0.0 | `Resource` (four subclasses), `ResourceGraph` |

Every CRUD resource carries the read / `Create` / `Update` triple, and every one
is member-complete against its v5 schema — the build fails otherwise. Every
client implements `HubOps` for event subscriptions.

## The loop these close

Together the fourteen cover commerce, fulfilment, assurance and monetisation:

**Browse** a catalog (TMF620) → **identify** who is buying (TMF632) and in what
capacity (TMF669) → **engage** them as a customer (TMF629) → **place** the order
(TMF622) → then find **what the customer has** (TMF637), **what delivers it**
(TMF638), and **what that runs on** (TMF639).

The last three chain by construction, because the specifications do:

```rust
let product  = products.get_product("P1", &q).await?;                              // TMF637
let service  = services.get_service(&product.realizing_service.unwrap()[0].id, &q).await?;    // TMF638
let resource = resources.get_resource(&service.supporting_resource.unwrap()[0].id, &q).await?; // TMF639
```

## Deliberate absences

**TMF641 Service Ordering** and **TMF688 Event Management** are not here, and
that is a decision rather than a gap: neither has a v5 release. The upstream
TMF641 repository stops at v4.2.0, and TMF688 remains v4-only. This crate models
v5 and does not pretend a v5 exists where one does not.

TMF638 does reference service orders, so `service::RelatedServiceOrderItem` is
present — what is absent is a TMF641 *client*.

Because TMF688 has no v5, what v5 actually ships for events is the per-API
hub/listener pattern rather than a central event bus. That is what
[the event support](@/docs/calling-an-api.md#events) models.

## Why fourteen and not ninety-six

TM Forum publishes around ninety-six API repositories. Covering them all would
mean generating the model from the OpenAPI documents, and this crate deliberately
does not.

The reason is that the gate would stop meaning anything. The schema-coverage
suite compares the Rust model against the OpenAPI document; if the model were
*generated from* that document, the comparison is `f(X)` against `X` — it passes
by construction and proves nothing. Every defect that gate has caught was
catchable only because a human wrote the model independently of the check.

The second reason is that the interesting decisions are not derivable from the
schemas at all:

- Twelve `…Characteristic` and five `…ContactMedium` schemas are each **one**
  Rust type, because they are a polymorphic family. Schema-by-schema generation
  emits eighteen types and loses the union.
- TMF622 and TMF637 both declare `Product`. It is **one** type, because they
  declare it identically. A generator emits two.
- TMF638 and TMF639 both declare `Feature`. It is **two** types, because they are
  different schemas under one name. A generator that deduplicated by name emits
  one, and lets you set a member the server drops.

Each of those is a judgement about what the API *means*. So the offer is fourteen
APIs done properly and provably, rather than ninety-six done mechanically.

Adding an API is a tractable contribution: the coverage suite's failure output
*is* the to-do list. See
[CONTRIBUTING](https://github.com/hupe1980/rutmf/blob/main/CONTRIBUTING.md).

## Status and stability

**0.1 — early.** The design is settled and proven against the specifications, and
both directions work: calling a TM Forum API and serving one. The API surface
will still move before 1.0, so pin an exact version.

Four things stand between here and a 1.0 that promises stability:

1. **A release cycle of real use** with the module layout held still. It has
   moved with each round of review, and each move would have been a breaking
   change after 1.0.
2. **Conformance against TM Forum's own CTK.** The kit and the reference
   implementations ship as Docker images, but through TM Forum's own channels
   rather than a public registry — so this needs membership, not engineering.
   Until a real run happens, "conformant" here means three specific things: the
   model agrees with the specification documents, every vendored example parses
   and round-trips, and the server answers every collection every document
   declares with the right status codes and headers. What none of those can tell
   you is whether the crate agrees with somebody else's *implementation*.
3. **A second implementor.** Every API here was modelled by one person reading
   the same documents. The coverage gate catches transcription errors; it cannot
   catch a misreading the mapping shares.
4. **A settled decision on `#[non_exhaustive]`.** It makes a TM Forum minor
   release non-breaking, at the permanent cost of struct-update syntax
   downstream. 1.0 is where that stops being reversible.

Two things earlier plans promised are **dropped, not pending**: a code generator
for the long-tail APIs (for the reasons above) and an `mcp` feature. MCP is an
application-level decision — which tools, which scopes, whose credentials — and
the `schemars` feature already supplies everything needed to build one outside
this crate.
