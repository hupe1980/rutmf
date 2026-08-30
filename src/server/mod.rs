//! Implement a TM Forum API, rather than call one.
//!
//! The client side of this crate answers "how do I talk to a TMF server". This
//! is the other direction: you have the data, and you need to expose it as a
//! conformant TMF API.
//!
//! # The shape of the problem
//!
//! Every TM Forum API is the same API. A collection or two, each with
//! `GET`/`POST` on the collection and `GET`/`PATCH`/`DELETE` on an item, plus a
//! notification `hub` — and TMF630 defines, once, what all of that *means*:
//! which resources a filter selects, how `sort=` orders them, what `fields=`
//! returns, how `offset`/`limit` page and what the count headers say, which of
//! four content types a `PATCH` arrived as, and what an error body looks like.
//!
//! The usual generated-server approach gives you a trait with a method per
//! operation per API, and leaves all of that to you — so every implementation
//! re-derives the same semantics, and each gets them slightly wrong.
//!
//! This module inverts it. You implement [`ResourceStore`] — five methods, all
//! about *storage*, none about HTTP — and [`TmfHandler`] supplies TMF630.
//!
//! ```
//! use rutmf::server::{MemoryStore, TmfHandler};
//!
//! let handler = TmfHandler::new(
//!     "https://mycsp.com/tmf-api/productCatalogManagement/v5",
//!     MemoryStore::new(),
//! );
//! # let _ = handler;
//! ```
//!
//! Nothing in [`ResourceStore`] can get the wire format wrong, because nothing
//! in it is about the wire. That is the whole design.
//!
//! # Concurrency is the one thing a store has to help with
//!
//! Everything above is free. `If-Match` is not, and it is the one place the
//! handler cannot be correct on its own: a `PATCH` is read-modify-write, and
//! checking a tag and then writing is two steps with a lost update between
//! them.
//!
//! So [`ResourceStore`] adds [`replace_if_unchanged`] and
//! [`delete_if_unchanged`], both **defaulted** — five methods still gets you a
//! working server. The defaults narrow the window; overriding them with whatever
//! your backend calls a compare-and-swap closes it, and that is where the
//! guarantee comes from. [`MemoryStore`] does it under its own lock.
//!
//! The handler issues an `ETag` on every read and write, honours `If-Match` on
//! `PATCH` and `DELETE`, and answers `304` to a `GET` whose `If-None-Match`
//! still holds. [`Conditional`](crate::api::Conditional) is the client half of
//! the same exchange, and `tests/server.rs` asserts the loop over a socket.
//!
//! [`replace_if_unchanged`]: ResourceStore::replace_if_unchanged
//! [`delete_if_unchanged`]: ResourceStore::delete_if_unchanged
//!
//! # Framework-agnostic, with one adapter provided
//!
//! [`TmfHandler::handle`] takes a [`TmfRequest`] and returns a [`TmfResponse`]
//! — the same pair the client side uses. Adapting it to an HTTP server is a
//! small function, and the crate ships one for `axum` behind the `server-axum`
//! feature. This mirrors the client layer, which is transport-agnostic with one
//! ready-made `reqwest` implementation.
//!
//! [`MockTmfServer`](crate::mock::MockTmfServer) is a third adapter: the same
//! handler wired straight into the client's [`Transport`], with no socket at
//! all. The mock and a real server therefore run *the same semantics code* —
//! the 591-fixture conformance corpus exercises one and vouches for both.
//!
//! # Notifications
//!
//! Every v5 API has a `/hub` collection, and serving it is only half the job: a
//! conformant server then `POST`s to the registered callback whenever a resource
//! changes. Getting there means naming the event `{Resource}{Kind}Event`,
//! wrapping the resource under the right payload member, reading each hub's
//! `query` as a TMF630 filter to decide who wants it, and telling a lifecycle
//! move (`…StateChangeEvent`) from an ordinary edit
//! (`…AttributeValueChangeEvent`).
//!
//! That is all TMF630 semantics, so the handler does it, and a `POST`, `PATCH`
//! or `DELETE` through the API raises the right event by itself. What is left is
//! the one part only a deployment can decide — whether delivery is a blocking
//! `POST`, a queue publish or a retry loop — and that is the [`Notifier`] seam.
//! Without one, subscriptions are still stored and read back; nothing is sent.
//!
//! # What this is not
//!
//! It does not validate bodies against the v5 schemas, enforce lifecycle
//! transitions, or authenticate anyone. Those are decisions only your
//! implementation can make, and a layer that guessed at them would be in the
//! way. Deserialise the body into the crate's own `…Create` type if you want
//! schema validation — that is what those types are for.
//!
//! [`TmfRequest`]: crate::api::TmfRequest
//! [`TmfResponse`]: crate::api::TmfResponse
//! [`Transport`]: crate::api::Transport

mod handler;
mod memory;
mod notify;
mod semantics;
mod store;

#[cfg(feature = "server-axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "server-axum")))]
mod axum_adapter;

#[cfg(feature = "server-axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "server-axum")))]
pub use axum_adapter::router;

pub use handler::{IdGenerator, Notifier, RandomId, TmfHandler, entity_tag};
pub use memory::MemoryStore;
pub use notify::{
    HUB_COLLECTION, Listener, change_event, event_type_for, matching_listeners, state_change_kind,
};
pub use semantics::{
    apply_json_patch, apply_merge_patch, is_reserved, matches_filters, project_fields,
    sort_resources,
};
pub use store::{Matched, Replaced, ResourceStore, Selection, StoreError, StoreResult};
