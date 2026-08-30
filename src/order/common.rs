//! Value objects shared by orders and order items.

use crate::core::macros::tmf_struct;
use crate::core::{BillingAccount, Channel, Duration, Price, Quantity, Ref};
use crate::product::{PriceAlteration, ProductOfferingPrice};

tmf_struct! {
    @name = "OrderPrice";
    /// One priced line on an order or order item.
    pub struct OrderPrice {
        /// Name of this price line.
        name: String,
        /// Narrative description.
        description: String,
        /// Kind of price, e.g. `recurring`, `oneTime`.
        price_type: String,
        /// For recurring charges, the billing period.
        recurring_charge_period: String,
        /// Unit the price applies to.
        unit_of_measure: String,
        /// The amount itself.
        price: Price,
        /// Discounts and surcharges applied to this line.
        price_alteration: Vec<PriceAlteration>,
        /// The catalog price this line was derived from.
        product_offering_price: Ref<ProductOfferingPrice>,
        /// The account this line is billed to.
        billing_account: Ref<BillingAccount>,
    }
}

tmf_struct! {
    @name = "OrderTerm";
    /// A contractual term attached to an order item.
    pub struct OrderTerm {
        /// Name of the term.
        name: String,
        /// Narrative description.
        description: String,
        /// How long the term runs.
        duration: Duration,
    }
}

tmf_struct! {
    @name = "RelatedChannel";
    /// A sales channel in a named role on an order.
    pub struct RelatedChannel {
        /// The channel being referred to.
        channel: Ref<Channel>,
        /// The role the channel plays, e.g. `salesChannel`.
        role: String,
    }
}

tmf_struct! {
    @name = "OrderRelationship";
    /// A link from one order to another.
    pub struct OrderRelationship {
        /// Identifier of the related order.
        id: String,
        /// URI of the related order.
        href: String,
        /// Name of the related order.
        name: String,
        /// Kind of relationship, e.g. `dependency`.
        relationship_type: String,
        @renamed {
            /// The concrete class of the related order.
            "@referredType" referred_type: String,
        }
    }
}

/// A quantity of some unit, reused from the core value objects.
pub type OrderQuantity = Quantity;
