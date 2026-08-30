//! `ProductOrderItem` — one line on a product order.

use crate::core::macros::tmf_struct;
use crate::core::{
    Appointment, BillingAccount, ItemAction, Note, Payment, ProductOfferingQualification, Ref,
};
use crate::product::{Product, ProductOffering};

use super::common::{OrderPrice, OrderTerm};
use super::state::ProductOrderItemState;

tmf_struct! {
    @name = "ProductOrderItem";
    /// One line on a [`ProductOrder`](super::ProductOrder): an action to take on
    /// one product.
    ///
    /// Items nest — a bundle offering produces a parent item with children —
    /// which is why `product_order_item` is a `Vec` of the same type.
    pub struct ProductOrderItem {
        /// Identifier of the line, unique within the order.
        id: String,
        /// What to do with the product.
        action: ItemAction,
        /// Fulfilment state of this line.
        state: ProductOrderItemState,
        /// How many.
        quantity: i64,
        /// The catalog offering being ordered.
        product_offering: Ref<ProductOffering>,
        /// The product being created or changed.
        product: Product,
        /// Prices for this line.
        item_price: Vec<OrderPrice>,
        /// Roll-up totals for this line and its children.
        item_total_price: Vec<OrderPrice>,
        /// Contractual terms for this line.
        item_term: Vec<OrderTerm>,
        /// Account this line is billed to.
        billing_account: Ref<BillingAccount>,
        /// Payments settling this line — TMF676.
        payment: Vec<Ref<Payment>>,
        /// Appointment booked to fulfil this line — TMF646.
        appointment: Ref<Appointment>,
        /// The qualification that established this line is deliverable — TMF679.
        qualification: Vec<Ref<ProductOfferingQualification>>,
        /// The qualification line this order line came from — TMF679.
        product_offering_qualification_item: ProductOfferingQualificationItemRef,
        /// The quote line this order line came from — TMF648.
        quote_item: QuoteItemRef,
        /// Nested lines, for a bundle.
        product_order_item: Vec<ProductOrderItem>,
        /// Relationships to other lines on the same order.
        product_order_item_relationship: Vec<OrderItemRelationship>,
        /// Notes attached to this line.
        note: Vec<Note>,
    }
}

tmf_struct! {
    @name = "ProductOrderItem";
    /// Body of one line in a `POST /productOrder` — the v5 `ProductOrderItem_FVO`.
    ///
    /// `id` and `action` are required on create: a line the provider cannot
    /// identify or act on is meaningless.
    ///
    /// ```
    /// use rutmf::core::{ItemAction, Ref};
    /// use rutmf::order::ProductOrderItemCreate;
    /// use rutmf::product::ProductOffering;
    ///
    /// let line = ProductOrderItemCreate::builder()
    ///     .id("1")
    ///     .action(ItemAction::Add)
    ///     .product_offering(Ref::<ProductOffering>::new("7655"))
    ///     .quantity(2)
    ///     .build();
    /// assert_eq!(line.id, "1");
    /// ```
    pub struct ProductOrderItemCreate {
        @required {
            /// Identifier of the line, unique within the order. **Required on create.**
            id: String,
            /// What to do with the product. **Required on create.**
            action: ItemAction,
        }
        /// Requested initial state of this line.
        state: ProductOrderItemState,
        /// How many.
        quantity: i64,
        /// The catalog offering being ordered.
        product_offering: Ref<ProductOffering>,
        /// The product being created or changed.
        product: Product,
        /// Prices for this line.
        item_price: Vec<OrderPrice>,
        /// Roll-up totals for this line and its children.
        item_total_price: Vec<OrderPrice>,
        /// Contractual terms for this line.
        item_term: Vec<OrderTerm>,
        /// Account this line is billed to.
        billing_account: Ref<BillingAccount>,
        /// Payments settling this line.
        payment: Vec<Ref<Payment>>,
        /// Appointment booked to fulfil this line.
        appointment: Ref<Appointment>,
        /// The qualification that established this line is deliverable.
        qualification: Vec<Ref<ProductOfferingQualification>>,
        /// The qualification line this order line came from.
        product_offering_qualification_item: ProductOfferingQualificationItemRef,
        /// The quote line this order line came from.
        quote_item: QuoteItemRef,
        /// Nested lines, for a bundle.
        product_order_item: Vec<ProductOrderItemCreate>,
        /// Relationships to other lines on the same order.
        product_order_item_relationship: Vec<OrderItemRelationship>,
        /// Notes attached to this line.
        note: Vec<Note>,
    }
}

impl ProductOrderItemCreate {
    /// A line adding the given offering.
    pub fn add(id: impl Into<String>, offering: Ref<ProductOffering>) -> Self {
        Self::builder()
            .id(id)
            .action(ItemAction::Add)
            .product_offering(offering)
            .build()
    }
}

tmf_struct! {
    @name = "QuoteItemRef";
    /// A line of a quote an order line came from — TMF648.
    ///
    /// One of the four v5 "item reference" shapes, which address a line *within*
    /// a parent resource and so carry no `id`/`href` of their own. That is why
    /// they are structs rather than [`Ref<T>`](crate::core::Ref), whose `id` is
    /// mandatory.
    pub struct QuoteItemRef {
        /// Identifier of the quote.
        quote_id: String,
        /// URI of the quote.
        quote_href: String,
        /// Identifier of the line within the quote.
        quote_item_id: String,
        @renamed {
            /// The concrete class of the quote.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "ProductOfferingQualificationItemRef";
    /// A line of a product offering qualification — TMF679.
    ///
    /// See [`QuoteItemRef`] for why this is a struct rather than a `Ref<T>`.
    pub struct ProductOfferingQualificationItemRef {
        /// Identifier of the qualification.
        product_offering_qualification_id: String,
        /// URI of the qualification.
        product_offering_qualification_href: String,
        /// Name of the qualification.
        product_offering_qualification_name: String,
        /// Identifier of the line within the qualification.
        item_id: String,
        @renamed {
            /// The concrete class of the qualification.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "ProductOrderItemRef";
    /// A line of a product order, addressed from outside it.
    ///
    /// See [`QuoteItemRef`] for why this is a struct rather than a `Ref<T>`.
    pub struct ProductOrderItemRef {
        /// Identifier of the order.
        product_order_id: String,
        /// Identifier of the line within the order.
        product_order_item_id: String,
        @renamed {
            /// URI of the order.
            ///
            /// The v5 schema spells this member with a leading capital, unlike
            /// every other member in the document. It is reproduced verbatim
            /// because that is what a conformant server sends.
            "ProductOrderHref" product_order_href: String,
            /// The concrete class of the order.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "OrderItemRelationship";
    /// A dependency between two lines on the same order.
    pub struct OrderItemRelationship {
        /// Identifier of the related line.
        id: String,
        /// Kind of relationship, e.g. `reliesOn`.
        relationship_type: String,
    }
}
