+++
title = "rutmf"
description = "Ergonomic, v5-first Rust types and clients for the TM Forum Open APIs. Typed references, real decimals, guaranteed round-trip fidelity, and a conformance suite over 610 official specification examples."
template = "index.html"

[extra]
lede = "Model resources, call partner APIs and serve TM Forum endpoints in Rust. Enable the domain models and clients your application needs."

[[extra.features]]
title = "Operation-specific types"
body = "Read models and create and update builders follow the fields each TMF operation accepts."

[[extra.features]]
title = "Round-trip fidelity"
body = "Vendor extensions retain their order, timestamps keep their UTC offset, and omitted type discriminators stay absent."

[[extra.features]]
title = "Typed references"
body = "A `Ref<ProductOffering>` resolves to a product offering through its client."

[[extra.features]]
title = "Conditional writes"
body = "Use an ETag with `If-Match` to detect concurrent edits. A stale write returns HTTP 412."

[[extra.features]]
title = "Exact monetary amounts"
body = "Read invoices and credits as decimals and emit exact JSON numbers, including amounts above the binary floating-point precision limit."

[[extra.features]]
title = "Client transports and errors"
body = "Use reqwest with OAuth2 or supply a transport. Read TMF630 errors and RFC 9457 problem details directly from the client error."

[[extra.features]]
title = "Server support"
body = "Connect a resource store to TMF630 routing, filtering, paging, JSON Patch and hub notifications."

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
id = "TMF635"
name = "Product Usage Management"
version = "v5.0.0"
resources = "ProductUsage (rated usage, prices on the record), ExportJob"

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

## Create a product offering

```rust
use rutmf::api::tmf620::ProductCatalogClient;
use rutmf::core::Timestamp;
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
        .last_update(Timestamp::now())
        .build(),
).await?;
```
