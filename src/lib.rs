//! Ergonomic, **v5-first** Rust types and clients for the TM Forum Open APIs.
//!
//! `rutmf` models the TM Forum domain the way you would model it by hand:
//! typed references, real decimals for money, builders that only accept valid
//! shapes — with the HTTP clients layered strictly on top, so the data model is
//! usable on its own.
//!
//! > **Unofficial.** This crate is a community implementation of the publicly
//! > available, Apache-2.0-licensed TM Forum Open API specifications. It is not
//! > affiliated with, endorsed by, or certified by TM Forum. "TM Forum" is a
//! > trademark of TM Forum.
//!
//! # The domain model
//!
//! ```
//! use rutmf::product::ProductOffering;
//!
//! let offering = ProductOffering::builder()
//!     .name("Business Internet")
//!     .is_sellable(true)
//!     .build();
//! ```
//!
//! Creating and updating are *different shapes* in TMF v5, and this crate keeps
//! them distinct so the compiler catches a malformed request:
//!
//! ```
//! use rutmf::product::ProductOfferingCreate;
//! use chrono::Utc;
//!
//! // `name`, `lifecycle_status` and `last_update` are required on create —
//! // omitting one is a compile error, not a 400 from the server.
//! let body = ProductOfferingCreate::builder()
//!     .name("Business Internet")
//!     .lifecycle_status("Active")
//!     .last_update(Utc::now())
//!     .build();
//! ```
//!
//! # Round-trip fidelity
//!
//! Real TMF payloads carry vendor extensions, and a library that silently drops
//! them is unusable as integration middleware. Every entity captures unknown
//! members in [`Extensions`], preserving their relative order:
//!
//! ```
//! use rutmf::product::ProductOffering;
//!
//! let json = r#"{"id":"7655","name":"Basic Firewall","@type":"ProductOffering","x-vendor":{"tier":2}}"#;
//! let offering: ProductOffering = serde_json::from_str(json).unwrap();
//!
//! assert_eq!(offering.extensions.get("x-vendor").unwrap()["tier"], 2);
//!
//! let back = serde_json::to_value(&offering).unwrap();
//! assert_eq!(back, serde_json::from_str::<serde_json::Value>(json).unwrap());
//! ```
//!
//! ## What is guaranteed, precisely
//!
//! Decoding then re-encoding any payload is **lossless by value**:
//!
//! - every member present in the input with a value is present in the output,
//!   with an equal value — including members this crate has no typed field for;
//! - members within [`Extensions`] keep their relative order;
//! - **no member is invented.** A payload that omits the spec-mandatory `@type`
//!   comes back without one; ask the type what class it is instead, through
//!   `type_name()`. Anything the crate *builds* declares its class, because a
//!   request without `@type` is the one a server rejects;
//! - a timestamp keeps the UTC offset it arrived with, so
//!   `2020-09-23T16:42:23-04:00` does not come back as `20:42:23Z` — the one
//!   re-spelling is `+00:00`, which RFC 3339 writes as `Z`.
//!
//! It is *not* byte-for-byte. Members this crate knows about are emitted in
//! struct-declaration order rather than input order; JSON number formatting is
//! normalised; and a timestamp's fractional seconds are re-emitted in SI groups,
//! so `.96747` becomes `.967470`. Compare with [`serde_json::Value`] equality,
//! not string equality.
//!
//! ## The one exception: an explicit `null`
//!
//! `{"description": null}` on a **modelled** member reads as absence and is not
//! re-emitted: `Option<T>` has two states where that needs three, and a
//! three-state type on every field would cost every caller for a distinction
//! the v5 schemas make almost nowhere. A `null` on an unmodelled member lands in
//! [`Extensions`] and does round-trip.
//!
//! Where the distinction is real — RFC 7386 makes `null` how a **merge patch**
//! removes a member — the `…Update` types say it:
//!
//! ```
//! use rutmf::product::ProductOfferingUpdate;
//!
//! let update = ProductOfferingUpdate::builder()
//!     .name("Business Internet")
//!     .build()
//!     .deleting("description");
//!
//! assert!(update.deletes("description"));
//! ```
//!
//! This is enforced over **all 591 vendored TM Forum examples**, every one of
//! them, by `tests/conformance.rs`.
//!
//! ## Round-tripping is not coverage
//!
//! An unknown member survives in [`Extensions`] whether or not the model
//! understands it, so a payload round-trips perfectly even if the typed model
//! misses half of it. `tests/coverage.rs` is what closes that gap: it reads the
//! vendored `OpenAPI` documents and fails if any specified member has no typed
//! field, if any typed field is absent from the specification, if requiredness
//! disagrees, or if a member the specification gives a closed vocabulary is
//! typed as a `String` rather than an enumeration. It checks **215 Rust types
//! against 462 v5 schemas**. Both suites are needed, and neither substitutes for
//! the other.
//!
//! [`Extensions`]: crate::core::Extensions
//!
//! # Feature flags
//!
//! Domain models are enabled by default and pull in no I/O. Clients are opt-in,
//! so `cargo add rutmf` never drags in a TLS stack.
//!
//! | Feature | Enables |
//! |---|---|
//! | `party`, `customer`, `product`, `order` | domain models (**default**) |
//! | `ticket`, `alarm` | the assurance domain: TMF621 tickets, TMF642 alarms |
//! | `bill`, `account` | monetisation: TMF678 bills, TMF666 accounts |
//! | `service`, `resource` | the inventory-side models — also on by default, because `product` needs them for its `realizingService` and `supportingResource` references |
//! | `api` | the transport-agnostic client layer |
//! | `transport-reqwest` | a ready-made `reqwest` transport, with `OAuth2` and retries |
//! | `api-tmf620` | Product Catalog Management (TMF620 v5.0.0) |
//! | `api-tmf621` | Trouble Ticket (TMF621 v5.0.1) |
//! | `api-tmf642` | Alarm Management (TMF642 v5.0.1) |
//! | `api-tmf666` | Account Management (TMF666 v5.0.0) |
//! | `api-tmf678` | Customer Bill (TMF678 v5.0.0) |
//! | `api-tmf622` | Product Ordering (TMF622 v5.0.0) |
//! | `api-tmf629` | Customer Management (TMF629 v5.0.1) |
//! | `api-tmf632` | Party Management (TMF632 v5.0.0) |
//! | `api-tmf634` | Resource Catalog Management (TMF634 v5.0.0) |
//! | `api-tmf637` | Product Inventory (TMF637 v5.0.0) |
//! | `api-tmf638` | Service Inventory (TMF638 v5.0.0) |
//! | `api-tmf639` | Resource Inventory (TMF639 v5.0.0) |
//! | `api-tmf669` | Party Role Management (TMF669 v5.0.0) |
//! | `api-tmf679` | Product Offering Qualification (TMF679 v5.0.0) |
//! | `server` | *implement* a TMF API: a store trait under a TMF630 handler |
//! | `server-axum` | an `axum` `Router` over that handler |
//! | `mock` | an in-process TMF server for tests |
//! | `schemars` | `JsonSchema` derives on every type |
//! | `full` | everything above |

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(html_root_url = "https://docs.rs/rutmf")]

pub mod core;

#[cfg(feature = "party")]
#[cfg_attr(docsrs, doc(cfg(feature = "party")))]
pub mod party;

#[cfg(feature = "customer")]
#[cfg_attr(docsrs, doc(cfg(feature = "customer")))]
pub mod customer;

#[cfg(feature = "product")]
#[cfg_attr(docsrs, doc(cfg(feature = "product")))]
pub mod product;

#[cfg(feature = "resource")]
#[cfg_attr(docsrs, doc(cfg(feature = "resource")))]
pub mod resource;

#[cfg(feature = "service")]
#[cfg_attr(docsrs, doc(cfg(feature = "service")))]
pub mod service;

#[cfg(feature = "account")]
#[cfg_attr(docsrs, doc(cfg(feature = "account")))]
pub mod account;

#[cfg(feature = "bill")]
#[cfg_attr(docsrs, doc(cfg(feature = "bill")))]
pub mod bill;

#[cfg(feature = "alarm")]
#[cfg_attr(docsrs, doc(cfg(feature = "alarm")))]
pub mod alarm;

#[cfg(feature = "ticket")]
#[cfg_attr(docsrs, doc(cfg(feature = "ticket")))]
pub mod ticket;

#[cfg(feature = "order")]
#[cfg_attr(docsrs, doc(cfg(feature = "order")))]
pub mod order;

#[cfg(feature = "api")]
#[cfg_attr(docsrs, doc(cfg(feature = "api")))]
pub mod api;

#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub mod server;

#[cfg(feature = "mock")]
#[cfg_attr(docsrs, doc(cfg(feature = "mock")))]
pub mod mock;

/// The traits whose methods are otherwise invisible.
///
/// `reference()` on a resource, `resolve()` on a `Ref`, `fetch()` on a client,
/// `register_listener()` on any of the fourteen — each lives on a trait, and
/// Rust does not offer a trait method until the trait is in scope.
///
/// **Traits, not types.** [`Query`] comes too, because those calls take one.
///
/// ```
/// use rutmf::prelude::*;
/// use rutmf::product::ProductOffering;
///
/// // `reference()` is `core::Entity`.
/// let offering = ProductOffering::builder().id("7655").name("Firewall").build();
/// assert_eq!(offering.reference().unwrap().id, "7655");
/// ```
///
/// Concrete types stay in their domain module: `rutmf::product::ProductOffering`
/// says which API it belongs to, and several of the fourteen declare a `Product`
/// or a `Category` of their own.
///
/// [`Query`]: crate::api::Query
pub mod prelude {
    pub use crate::core::{Entity, TmfType};

    #[cfg(feature = "api")]
    #[cfg_attr(docsrs, doc(cfg(feature = "api")))]
    pub use crate::api::{Conditional, HubOps, Query, ResolveRef};
}

/// The TM Forum Open API **major** version this crate targets.
///
/// This is the `v5` that appears in every API's URL path, and it is the whole
/// of the crate's version story at the major level: there is no v4 support and
/// no build flag that changes what a type means.
///
/// It is deliberately *not* the patch version. The covered APIs sit on
/// different patch releases — TMF621 and TMF629 are at 5.0.1 while the rest are
/// at 5.0.0 — so the precise version each client was modelled from is a
/// per-API constant, `SPEC_VERSION`, next to that client's `API_PATH`:
///
/// ```
/// # #[cfg(feature = "api-tmf621")] {
/// assert_eq!(rutmf::api::tmf621::SPEC_VERSION, "5.0.1");
/// assert_eq!(rutmf::TMF_VERSION, "v5");
/// # }
/// ```
pub const TMF_VERSION: &str = "v5";
