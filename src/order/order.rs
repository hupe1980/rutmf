//! `ProductOrder` — a request to provide, change or cease products.

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    Agreement, BillingAccount, ExternalIdentifier, Note, Payment, ProductOfferingQualification,
    Quote, Ref, RelatedParty, Timestamp,
};
use serde::{Deserialize, Serialize};

use super::common::{OrderPrice, OrderRelationship, RelatedChannel};
use super::item::{ProductOrderItem, ProductOrderItemCreate, ProductOrderItemRef};
use super::state::{InitialProductOrderState, ProductOrderState};

tmf_struct! {
    @name = "ProductOrder", @ref = "ProductOrderRef";
    /// A request to provide, change or cease one or more products.
    ///
    /// This is the **read model**. Use [`ProductOrderCreate`] for `POST` and
    /// [`ProductOrderUpdate`] for `PATCH`.
    pub struct ProductOrder {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this order.
        href: String,
        /// Narrative description.
        description: String,
        /// Category the order falls under.
        category: String,
        /// Priority, conventionally `0` (highest) to `4`.
        priority: String,
        /// Current fulfilment state.
        state: ProductOrderState,
        /// When the order was created.
        creation_date: Timestamp,
        /// When fulfilment finished.
        completion_date: Timestamp,
        /// When the provider expects to finish.
        expected_completion_date: Timestamp,
        /// When the buyer asked for fulfilment to finish.
        requested_completion_date: Timestamp,
        /// When the buyer asked for fulfilment to start.
        requested_start_date: Timestamp,
        /// When the order was cancelled.
        cancellation_date: Timestamp,
        /// Why the order was cancelled.
        cancellation_reason: String,
        /// Where to send progress notifications.
        notification_contact: String,
        /// The state the buyer asked the order to start in.
        requested_initial_state: InitialProductOrderState,
        /// The lines making up the order.
        product_order_item: Vec<ProductOrderItem>,
        /// Roll-up totals across the order.
        order_total_price: Vec<OrderPrice>,
        /// The account the order is billed to.
        billing_account: Ref<BillingAccount>,
        /// Payments settling the order — TMF676.
        payment: Vec<Ref<Payment>>,
        /// The quote the order was raised from — TMF648.
        quote: Vec<Ref<Quote>>,
        /// Qualifications establishing the order is deliverable — TMF679.
        product_offering_qualification: Vec<Ref<ProductOfferingQualification>>,
        /// Agreements the order is placed under.
        agreement: Vec<Ref<Agreement>>,
        /// Channels the order came through.
        channel: Vec<RelatedChannel>,
        /// Parties involved: buyer, seller, contact.
        related_party: Vec<RelatedParty>,
        /// Links to other orders.
        order_relationship: Vec<OrderRelationship>,
        /// Notes attached to the order.
        note: Vec<Note>,
        /// Identifiers in external systems.
        external_id: Vec<ExternalIdentifier>,
        /// Errors raised during fulfilment.
        product_order_error_message: Vec<OrderErrorMessage>,
        /// Milestones reached during fulfilment.
        product_order_milestone: Vec<OrderMilestone>,
        /// Warnings that fulfilment may miss its dates.
        product_order_jeopardy_alert: Vec<OrderJeopardyAlert>,
    }
}

tmf_entity!(ProductOrder);

tmf_struct! {
    @name = "ProductOrder";
    /// Body of a `POST /productOrder` — the v5 `ProductOrder_FVO`.
    ///
    /// This is the strictest create body in the crate, and deliberately so.
    /// TMF622 removes every server-owned member from the create schema —
    /// `state`, `creationDate`, `completionDate`, `expectedCompletionDate`,
    /// `cancellationDate` and `cancellationReason` are all absent, so they
    /// cannot be sent by accident — and requires `productOrderItem`.
    ///
    /// ```
    /// use rutmf::core::Ref;
    /// use rutmf::order::{ProductOrderCreate, ProductOrderItemCreate};
    /// use rutmf::product::ProductOffering;
    ///
    /// let order = ProductOrderCreate::builder()
    ///     .product_order_item(vec![ProductOrderItemCreate::add(
    ///         "1",
    ///         Ref::<ProductOffering>::new("7655"),
    ///     )])
    ///     .description("Firewall for the Berlin office")
    ///     .build();
    /// assert_eq!(order.product_order_item.len(), 1);
    /// ```
    pub struct ProductOrderCreate {
        @required {
            /// The lines making up the order. **Required on create.**
            product_order_item: Vec<ProductOrderItemCreate>,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Category the order falls under.
        category: String,
        /// Priority, conventionally `0` (highest) to `4`.
        priority: String,
        /// The state to submit the order in — only `draft` or `acknowledged`.
        requested_initial_state: InitialProductOrderState,
        /// When the buyer asks fulfilment to start.
        requested_start_date: Timestamp,
        /// When the buyer asks fulfilment to finish.
        requested_completion_date: Timestamp,
        /// Where to send progress notifications.
        notification_contact: String,
        /// Roll-up totals across the order.
        order_total_price: Vec<OrderPrice>,
        /// The account to bill.
        billing_account: Ref<BillingAccount>,
        /// Payments settling the order.
        payment: Vec<Ref<Payment>>,
        /// The quote the order was raised from.
        quote: Vec<Ref<Quote>>,
        /// Qualifications establishing the order is deliverable.
        product_offering_qualification: Vec<Ref<ProductOfferingQualification>>,
        /// Agreements the order is placed under.
        agreement: Vec<Ref<Agreement>>,
        /// Channels the order came through.
        channel: Vec<RelatedChannel>,
        /// Parties involved: buyer, seller, contact.
        related_party: Vec<RelatedParty>,
        /// Links to other orders.
        order_relationship: Vec<OrderRelationship>,
        /// Notes attached to the order.
        note: Vec<Note>,
        /// Identifiers in external systems.
        external_id: Vec<ExternalIdentifier>,
        /// Errors, where a server accepts them on create.
        product_order_error_message: Vec<OrderErrorMessage>,
        /// Milestones, where a server accepts them on create.
        product_order_milestone: Vec<OrderMilestone>,
        /// Jeopardy alerts, where a server accepts them on create.
        product_order_jeopardy_alert: Vec<OrderJeopardyAlert>,
    }
}

tmf_struct! {
    @name = "ProductOrder";
    /// Body of a `PATCH /productOrder/{id}` — the v5 `ProductOrder_MVO`.
    ///
    /// `id`, `href`, `creationDate` and `requestedInitialState` are absent: the
    /// first three are server-owned and the fourth only makes sense at creation.
    pub struct ProductOrderUpdate {
        /// New description.
        description: String,
        /// New category.
        category: String,
        /// New priority.
        priority: String,
        /// New state.
        state: ProductOrderState,
        /// New completion date.
        completion_date: Timestamp,
        /// New expected completion date.
        expected_completion_date: Timestamp,
        /// New requested start date.
        requested_start_date: Timestamp,
        /// New requested completion date.
        requested_completion_date: Timestamp,
        /// New cancellation date.
        cancellation_date: Timestamp,
        /// New cancellation reason.
        cancellation_reason: String,
        /// New notification contact.
        notification_contact: String,
        /// Replacement lines.
        product_order_item: Vec<ProductOrderItem>,
        /// Replacement order totals.
        order_total_price: Vec<OrderPrice>,
        /// Replacement billing account.
        billing_account: Ref<BillingAccount>,
        /// Replacement payments.
        payment: Vec<Ref<Payment>>,
        /// Replacement quotes.
        quote: Vec<Ref<Quote>>,
        /// Replacement qualifications.
        product_offering_qualification: Vec<Ref<ProductOfferingQualification>>,
        /// Replacement agreements.
        agreement: Vec<Ref<Agreement>>,
        /// Replacement channels.
        channel: Vec<RelatedChannel>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement order relationships.
        order_relationship: Vec<OrderRelationship>,
        /// Replacement notes.
        note: Vec<Note>,
        /// Replacement external identifiers.
        external_id: Vec<ExternalIdentifier>,
        /// Replacement error messages.
        product_order_error_message: Vec<OrderErrorMessage>,
        /// Replacement milestones.
        product_order_milestone: Vec<OrderMilestone>,
        /// Replacement jeopardy alerts.
        product_order_jeopardy_alert: Vec<OrderJeopardyAlert>,
    }
}

tmf_struct! {
    @name = "ProductOrderErrorMessage";
    /// An error raised while fulfilling an order.
    pub struct OrderErrorMessage {
        /// Application-relevant error code.
        code: String,
        /// Explanation safe to show to a user.
        reason: String,
        /// Further detail and corrective actions.
        message: String,
        /// HTTP-style status, carried as a string.
        status: String,
        /// URI of documentation describing the error.
        reference_error: String,
        /// When the error was raised.
        timestamp: Timestamp,
        /// The order lines this error relates to.
        product_order_item: Vec<ProductOrderItemRef>,
    }
}

/// Whether a milestone has been met.
///
/// TMF622 writes these values with a leading capital and a hyphen — unlike
/// every other vocabulary in the API — so the wire names are spelled out rather
/// than derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum OrderMilestoneStatus {
    /// Not yet met, and still within its window.
    #[serde(rename = "Yet-To-Reach")]
    YetToReach,
    /// Met.
    #[serde(rename = "Completed")]
    Completed,
    /// Missed.
    #[serde(rename = "Violated")]
    Violated,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

tmf_struct! {
    @name = "ProductOrderMilestone";
    /// A milestone reached while fulfilling an order.
    pub struct OrderMilestone {
        /// Identifier of the milestone.
        id: String,
        /// Name of the milestone.
        name: String,
        /// Narrative description.
        description: String,
        /// Status of the milestone.
        status: OrderMilestoneStatus,
        /// Free-text message.
        message: String,
        /// When the milestone was reached.
        milestone_date: Timestamp,
        /// The order lines this milestone relates to.
        product_order_item: Vec<ProductOrderItemRef>,
    }
}

tmf_struct! {
    @name = "ProductOrderJeopardyAlert";
    /// A warning that an order may miss its committed dates.
    pub struct OrderJeopardyAlert {
        /// Identifier of the alert.
        id: String,
        /// Name of the alert.
        name: String,
        /// Kind of jeopardy.
        jeopardy_type: String,
        /// The exception that triggered the alert.
        exception: String,
        /// Free-text message.
        message: String,
        /// When the alert was raised.
        alert_date: Timestamp,
        /// The order lines this alert relates to.
        product_order_item: Vec<ProductOrderItemRef>,
    }
}

tmf_patch_body!(ProductOrderUpdate);
