//! The ordering domain: product orders and their cancellation.
//!
//! Mirrors **TMF622 Product Ordering v5.0.0**, the API that closes the loop
//! from catalog to fulfilment: a [`ProductOrder`] references the
//! [`ProductOffering`](crate::product::ProductOffering)s being bought, and the
//! provider turns each [`ProductOrderItem`] into a product in the inventory.
//!
//! # States are types
//!
//! TMF622 defines three overlapping state enumerations, and this module keeps
//! them apart:
//!
//! - [`ProductOrderState`] — the whole order, thirteen values
//! - [`ProductOrderItemState`] — one line, eleven values
//! - [`InitialProductOrderState`] — the two states a *client* may request
//!
//! The third is the interesting one: `requestedInitialState` accepts only
//! `draft` or `acknowledged`, so asking a server to create an order in
//! `completed` is a compile error rather than a rejected request.
//!
//! # Cancellation is a resource
//!
//! You do not cancel an order by patching its state — see
//! [`CancelProductOrder`].
//!
//! # What an order line acts on lives in `product`
//!
//! A [`ProductOrderItem`] carries a [`Product`](crate::product::Product): the
//! thing the customer will have once the order completes. TMF622 and TMF637
//! declare that schema identically, so it is one type, and it lives with the
//! inventory API that owns it.

mod cancel;
mod common;
mod item;
#[allow(
    clippy::module_inception,
    reason = "the order module's primary resource"
)]
mod order;
mod state;

pub use cancel::{CancelProductOrder, CancelProductOrderCreate};
pub use common::{OrderPrice, OrderQuantity, OrderRelationship, OrderTerm, RelatedChannel};
// TMF621, TMF622, TMF638 and TMF639 declare `Note` identically, so it is one
// type in `core` rather than one per domain.
pub use crate::core::Note;
pub use item::{
    OrderItemRelationship, ProductOfferingQualificationItemRef, ProductOrderItem,
    ProductOrderItemCreate, ProductOrderItemRef, QuoteItemRef,
};
pub use order::{
    OrderErrorMessage, OrderJeopardyAlert, OrderMilestone, OrderMilestoneStatus, ProductOrder,
    ProductOrderCreate, ProductOrderUpdate,
};
pub use state::{InitialProductOrderState, ProductOrderItemState, ProductOrderState};
