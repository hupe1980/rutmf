+++
title = "API coverage"
description = "Which TM Forum Open APIs rutmf implements, at which v5 version, and which are deliberately absent because TM Forum has not published a v5."
weight = 60
+++

Fifteen APIs are implemented end to end — domain types, client, test server,
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
| **TMF635** Product Usage Management | v5.0.0 | `ProductUsage` (rated usage, prices on the record), `ExportJob` |
| **TMF678** Customer Bill | v5.0.0 | `CustomerBill`, `CustomerBillOnDemand`, `AppliedCustomerBillingRate`, `BillCycle` |
| **TMF638** Service Inventory Management | v5.0.0 | `Service` |
| **TMF639** Resource Inventory Management | v5.0.0 | `Resource` (four subclasses), `ResourceGraph` |

Every CRUD resource carries the read / `Create` / `Update` triple, and every one
is member-complete against its v5 schema — the build fails otherwise. Every
client implements `HubOps` for event subscriptions; the six whose API declares
`GET /hub/{id}` also implement `HubLookup`.

Each API module also publishes the API's declared surface as `API` — every
collection, including `hub` and those this crate does not model, with the
operations declared on it. The server serves exactly that surface, the clients
offer no operation outside it, and both are checked against the specification
document in both directions.

## The loop these close

Together the fifteen cover commerce, fulfilment, assurance and monetisation:

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

### No `RatedProductUsage` in v5

TMF635 v4 had a `RatedProductUsage` resource. **v5 has none.** A rated record is
a `ProductUsage` whose status is `rated` (or `corrected`), and its prices are on
the record, in `usageProductPrice` — the same `ProductPrice` TMF622 and TMF637
use. If you are porting v4 code and looking for `RatedProductUsage`, use
`ProductUsage::usage_product_price`.

### References this crate cannot resolve

The model types every reference, including references to entities whose API has
no v5 release. Those stay unresolvable by design: there is no v5 client to resolve
them with, and this crate does not model v4. Checked against the `tmforum-apis`
repositories on 2026-11-20:

| Reference target | Its API | v5 upstream |
|---|---|---|
| `Payment` (`AppliedPayment.payment`) | TMF676 Payment | no — v4 only |
| `PaymentMethod` (`CustomerBill.paymentMethod`, accounts) | TMF670 Payment Method | no — v4 only |
| `Agreement`, `AgreementSpecification` | TMF651 Agreement | no |
| `Promotion` | TMF671 Promotion | no |
| `GeographicAddress` | TMF673 Geographic Address | no |
| `Quote` | TMF648 Quote | no |
| `ServiceCandidate`, `ServiceSpecification` | TMF633 Service Catalog | no |
| `Threshold` | TMF628 Performance | no |
| `Appointment` | TMF646 Appointment | no |
| `ServiceLevelAgreement` | TMF623 SLA | no repository |
| `ProductUsageSpecification`, `ServiceUsage` | TMF635 (referenced, not defined) | — |

The `Ref` still round-trips and carries `id`, `href` and `@referredType`, so a
system holding the v4 API can follow it with its own client.

Because TMF688 has no v5, what v5 actually ships for events is the per-API
hub/listener pattern rather than a central event bus. That is what
[the event support](@/docs/calling-an-api.md#events) models.

## Status and stability

During 0.x, minor releases may change public APIs. The conformance suite checks the published OpenAPI documents, examples and server behavior. TM Forum CTK certification is outside its scope.
