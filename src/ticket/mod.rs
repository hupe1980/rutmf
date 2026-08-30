//! The assurance domain: what went wrong, and who is fixing it.
//!
//! Mirrors **TMF621 Trouble Ticket v5.0.1**. A [`TroubleTicket`] is the record
//! of a reported problem — raised by a customer or by an operator, worked
//! through a lifecycle, and eventually resolved. It is the first API in this
//! crate that is about *operations* rather than about selling or inventorying,
//! and it references the domains that are: a ticket relates parties (TMF632),
//! and its `relatedEntity` points at whatever the trouble is about.
//!
//! # Three ways TMF621 differs from its siblings
//!
//! **Responses have their own schema.** TMF621 declares `TroubleTicket_RES`
//! beside `TroubleTicket`, with identical members but `id` and `href` marked
//! required — making explicit what other APIs leave implied. This crate maps
//! the read model to both, and keeps the two members optional, because
//! requiredness binds where a client *authors* a payload and relaxes where it
//! parses one.
//!
//! **The patch body keeps `id` and `href`.** Every other `_MVO` in this crate
//! drops the server-owned members; TMF621's does not, so
//! [`TroubleTicketUpdate`] carries them. Modelling the schema faithfully beats
//! modelling it consistently.
//!
//! **Two event kinds nothing else uses.** TMF621 raises `…ResolvedEvent` and
//! `…StatusChangeEvent` — the latter spelled *Status*, not *State*, as
//! everywhere else. Both are in [`EventKind`](crate::core::EventKind).

mod trouble;

pub use trouble::{
    RelatedEntity, StatusChange, TroubleTicket, TroubleTicketCreate, TroubleTicketRelationship,
    TroubleTicketSpecification, TroubleTicketSpecificationCreate, TroubleTicketSpecificationUpdate,
    TroubleTicketStatus, TroubleTicketUpdate,
};
