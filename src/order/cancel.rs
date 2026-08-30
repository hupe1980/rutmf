//! `CancelProductOrder` — cancellation as a first-class resource.
//!
//! TMF622 does not cancel an order by patching its state. Cancellation is a
//! *task*: you `POST` a request, the provider assesses it, and the order moves
//! through `assessingCancellation` → `pendingCancellation` → `cancelled` under
//! the provider's control.

use crate::core::macros::{tmf_entity, tmf_struct};
use crate::core::{Ref, TaskState, Timestamp};

use super::order::ProductOrder;

tmf_struct! {
    @name = "CancelProductOrder";
    /// A request to cancel a product order.
    pub struct CancelProductOrder {
        /// Server-assigned identifier of the request.
        id: String,
        /// Canonical URI of the request.
        href: String,
        /// The order to cancel.
        product_order: Ref<ProductOrder>,
        /// Why the buyer wants to cancel.
        cancellation_reason: String,
        /// When the buyer wants the cancellation to take effect.
        requested_cancellation_date: Timestamp,
        /// When the cancellation actually took effect.
        effective_cancellation_date: Timestamp,
        /// When the request was created.
        creation_date: Timestamp,
        /// Progress of the request itself, not of the order.
        state: TaskState,
    }
}

tmf_entity!(CancelProductOrder);

tmf_struct! {
    @name = "CancelProductOrder";
    /// Body of a `POST /cancelProductOrder` — the v5 `CancelProductOrder_FVO`.
    ///
    /// `productOrder` is required; `state`, `creationDate` and
    /// `effectiveCancellationDate` are server-owned and absent from this type.
    ///
    /// ```
    /// use rutmf::core::Ref;
    /// use rutmf::order::{CancelProductOrderCreate, ProductOrder};
    ///
    /// let request = CancelProductOrderCreate::builder()
    ///     .product_order(Ref::<ProductOrder>::new("42"))
    ///     .cancellation_reason("Ordered in error")
    ///     .build();
    /// assert_eq!(request.product_order.id, "42");
    /// ```
    pub struct CancelProductOrderCreate {
        @required {
            /// The order to cancel. **Required on create.**
            product_order: Ref<ProductOrder>,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Why the buyer wants to cancel.
        cancellation_reason: String,
        /// When the buyer wants the cancellation to take effect.
        requested_cancellation_date: Timestamp,
    }
}

impl CancelProductOrderCreate {
    /// A cancellation request for the given order.
    #[must_use]
    pub fn for_order(order: Ref<ProductOrder>) -> Self {
        Self::builder().product_order(order).build()
    }
}
