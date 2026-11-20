//! Usage: what a customer consumed, and what it was rated at — TMF635.
//!
//! A [`ProductUsage`] is one usage record against a product: a call, a data
//! session, an event. Mediation creates it, rating prices it, and billing puts
//! it on a bill. Its [`ProductUsageStatus`] says how far it has got:
//!
//! ```text
//! initialized ──▶ validated ──▶ rated ──▶ (corrected record) …
//!      │              │           └──────▶ superseded
//!      └─▶ rejected   └─▶ notRated / failed
//! ```
//!
//! # Prices live on the record
//!
//! TMF635 v4 had a separate `RatedProductUsage`. **v5 has none**: a rated record
//! carries its prices in `usage_product_price`, the same
//! [`ProductPrice`](crate::product::ProductPrice) TMF622 and TMF637 use, with
//! money as an exact decimal.
//!
//! # Correcting rated usage
//!
//! TMF635 declares `PATCH` and `DELETE` on a usage record, so the protocol lets
//! a client rewrite one. A billing system usually must not: a rated record is
//! the evidence a bill was built from. Correct it by recording a new one with
//! status [`Corrected`](ProductUsageStatus::Corrected) and moving the old one to
//! [`Superseded`](ProductUsageStatus::Superseded) — the only change the old
//! record then needs. A server that wants that enforced refuses the rest in its
//! store; the serving guide shows how.
//!
//! # Referring to a usage record
//!
//! TMF635 declares no `ProductUsageRef`, so `Ref<ProductUsage>` is the generic v5
//! reference: `{"@type": "EntityRef", "@referredType": "ProductUsage", …}`. With
//! the `bill` feature, [`AppliedCustomerBillingRate::product_usages`] reads the
//! usage a bill line charges for.
//!
//! [`AppliedCustomerBillingRate::product_usages`]: crate::bill::AppliedCustomerBillingRate::product_usages

mod product_usage;

pub use product_usage::{
    ProductUsage, ProductUsageCreate, ProductUsageExportJobCreate, ProductUsageStatus,
    ProductUsageUpdate,
};
