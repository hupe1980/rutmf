//! HTTP clients for the TM Forum Open APIs.
//!
//! The domain types in [`crate::core`] and the domain modules know nothing
//! about HTTP. This module adds the wire behaviour TMF630 v5 prescribes:
//!
//! - [`Query`] builds `fields`, `offset`, `limit`, `sort` and attribute filters,
//!   including the comparison operators and value lists TMF630 defines
//! - [`Patch`] pairs a `PATCH` body with the semantics it must be sent under
//! - [`Page`] surfaces `X-Total-Count`, `X-Result-Count` and the `Link`
//!   header, and [`paginate`] turns them into a [`Stream`]
//! - [`Error`] carries the parsed TMF630 error body, not a formatted string
//! - [`HubOps`] registers event listeners, uniformly across APIs — the
//!   [`TmfEvent`] envelope itself lives in `core`, so a webhook handler needs
//!   no client
//! - [`ResolveRef`] follows a typed `Ref<T>` to the resource it points at
//! - [`RetryTransport`] re-issues idempotent requests that fail transiently
//! - [`Conditional`] reads a resource with its `ETag`, so a later write can be
//!   made conditional on nobody else having edited it in between
//!
//! Requests reach the network through the [`Transport`] trait, so clients are
//! testable without a socket and `reqwest` stays optional.
//!
//! # Available clients
//!
//! Each client is behind its own feature, so a build pays for the APIs it uses.
//!
//! | Feature | Client | API |
//! |---|---|---|
//! | `api-tmf620` | [`tmf620::ProductCatalogClient`] | Product Catalog Management v5.0.0 |
//! | `api-tmf621` | [`tmf621::TroubleTicketClient`] | Trouble Ticket v5.0.1 |
//! | `api-tmf622` | [`tmf622::ProductOrderClient`] | Product Ordering v5.0.0 |
//! | `api-tmf629` | [`tmf629::CustomerClient`] | Customer Management v5.0.1 |
//! | `api-tmf632` | [`tmf632::PartyClient`] | Party Management v5.0.0 |
//! | `api-tmf634` | [`tmf634::ResourceCatalogClient`] | Resource Catalog Management v5.0.0 |
//! | `api-tmf637` | [`tmf637::ProductInventoryClient`] | Product Inventory Management v5.0.0 |
//! | `api-tmf638` | [`tmf638::ServiceInventoryClient`] | Service Inventory Management v5.0.0 |
//! | `api-tmf639` | [`tmf639::ResourceInventoryClient`] | Resource Inventory Management v5.0.0 |
//! | `api-tmf642` | [`tmf642::AlarmClient`] | Alarm Management v5.0.1 |
//! | `api-tmf666` | [`tmf666::AccountClient`] | Account Management v5.0.0 |
//! | `api-tmf669` | [`tmf669::PartyRoleClient`] | Party Role Management v5.0.0 |
//! | `api-tmf678` | [`tmf678::CustomerBillClient`] | Customer Bill Management v5.0.0 |
//! | `api-tmf679` | [`tmf679::ProductOfferingQualificationClient`] | Product Offering Qualification v5.0.0 |
//!
//! # Not every client has the same methods
//!
//! The five CRUD operations — list, get, create, patch, delete — are the common
//! case, not the rule. **Plenty of collections do not have all five.** TMF620's
//! `importJob` has no `PATCH`; TMF622's `cancelProductOrder` and TMF642's six
//! alarm tasks are `POST`-and-read; TMF678 has *no* resource with the full set
//! — `customerBill` cannot be created or deleted, and two of its collections are
//! read-only.
//!
//! Generating five methods regardless would put `create_customer_bill` and
//! `delete_bill_cycle` on a client, against endpoints no conformant server
//! serves. That is the same class of defect as sending a body with the wrong
//! content type: a request the type system invited you to make and the server
//! rejects. So each client exposes exactly the operations its specification
//! declares, and `tests/coverage.rs` checks the composition against the
//! vendored paths — a client cannot grow a method for an endpoint that does not
//! exist.
//!
//! # Credentials and server-supplied URLs
//!
//! A [`Transport`] attaches its credentials to whatever URL it is handed, and
//! TMF payloads are full of URLs the *server* wrote: the `href` of every
//! `…Ref`, and the `Link: rel="next"` header of every paged collection. Both are
//! therefore checked against the client's own origin before being followed, and
//! a URL that leaves it raises [`Error::CrossOrigin`] rather than sending a
//! bearer token somewhere new.
//!
//! Within a deployment the TM Forum APIs share a host and differ by path, so
//! this refuses nothing that ordinarily happens. Genuine federation across hosts
//! is available explicitly, through [`TmfClient::get_cross_origin`] and
//! [`ResolveRef::resolve_cross_origin`].
//!
//! # Concurrent edits
//!
//! A TMF `PATCH` is read-modify-write, so two clients editing different members
//! of one resource each overwrite the other — with `200` to both, and the losing
//! change simply gone.
//!
//! [`Conditional::fetch`] returns the resource with the `ETag` the server issued
//! for it, and [`Tagged::update`] sends that back as `If-Match`, making a stale
//! write a [`412`](Error::is_precondition_failed). The v5 documents declare no
//! request headers, so this is RFC 9110 rather than TMF; a server that ignores
//! the precondition answers as it would without one, and `fetch` reports whether
//! a tag was issued.
//!
//! [`Stream`]: futures_core::Stream

mod client;
mod conditional;
mod error;
mod hub;
mod ops;
mod page;
mod patch;
mod query;
mod resolve;
mod retry;
mod transport;

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
mod reqwest_transport;

#[cfg(feature = "api-tmf620")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf620")))]
pub mod tmf620;

#[cfg(feature = "api-tmf621")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf621")))]
pub mod tmf621;

#[cfg(feature = "api-tmf622")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf622")))]
pub mod tmf622;

#[cfg(feature = "api-tmf629")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf629")))]
pub mod tmf629;

#[cfg(feature = "api-tmf632")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf632")))]
pub mod tmf632;

#[cfg(feature = "api-tmf634")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf634")))]
pub mod tmf634;

#[cfg(feature = "api-tmf637")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf637")))]
pub mod tmf637;

#[cfg(feature = "api-tmf638")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf638")))]
pub mod tmf638;

#[cfg(feature = "api-tmf639")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf639")))]
pub mod tmf639;

#[allow(
    unused_imports,
    reason = "no per-API client is enabled in an `api`-only build"
)]
#[cfg(feature = "api-tmf642")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf642")))]
pub mod tmf642;

#[cfg(feature = "api-tmf666")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf666")))]
pub mod tmf666;
#[cfg(feature = "api-tmf669")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf669")))]
pub mod tmf669;
#[cfg(feature = "api-tmf679")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf679")))]
pub mod tmf679;

#[cfg(feature = "api-tmf678")]
#[cfg_attr(docsrs, doc(cfg(feature = "api-tmf678")))]
pub mod tmf678;

// Which of these a build needs depends on which per-API clients are enabled:
// `op_delete` is only reached by the two catalog clients, `readonly_ops` only
// by TMF678. An unused one is a warning that CI escalates to an error.
#[allow(
    unused_imports,
    reason = "each client composes only the operations its API declares"
)]
pub(crate) use ops::{
    op_delete, op_get, op_list, op_patch, op_stream, readonly_ops, resource_ops, task_ops,
};

// Re-exported so client-side code has one import path, though both are
// I/O-free wire data and belong to the model — see `crate::core`.
pub use crate::core::{JsonPatchOp, PatchOperation, TmfEvent};
// The mock reuses the client's own reading of a failure response, so
// `expect_error` cannot drift from what a real client would report. Nothing else
// needs it, so it is not compiled into a build without the mock.
#[cfg(feature = "mock")]
pub(crate) use client::interpret_failure;
pub use client::{TmfClient, same_origin};
pub use conditional::{Conditional, EntityTag, Tagged};
pub use error::{Error, Result};
pub use hub::{Hub, HubCreate, HubOps};
pub use page::{DEFAULT_PAGE_SIZE, Page, PageRequest, PageStream, next_link, paginate};
pub use patch::Patch;
pub use query::{FilterOp, Query};
pub use resolve::{Resolvable, ResolveRef};
pub use retry::{RetryPolicy, RetryTransport, Sleeper};

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub use retry::TokioSleeper;
pub use transport::{TmfRequest, TmfResponse, Transport, TransportError};

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub use reqwest_transport::{Auth, ClientCredentials, ReqwestTransport, ReqwestTransportBuilder};
