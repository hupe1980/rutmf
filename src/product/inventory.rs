//! `Product` — what a customer actually has.
//!
//! Mirrors **TMF637 Product Inventory Management v5.0.0**: the record of an
//! offering a customer has bought, in some lifecycle state, realised by
//! services and resources.
//!
//! This is where the commerce loop lands. A [`ProductOffering`] is what a
//! provider sells, a [`ProductOrder`] is a request to buy one, and a [`Product`]
//! is the result — which is why TMF622 and TMF637 declare the schema
//! identically and this crate has one type for both.
//!
//! [`ProductOffering`]: crate::product::ProductOffering
//! [`ProductOrder`]: crate::order::ProductOrder

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    BillingAccount, Characteristic, Duration, Intent, ItemAction, Price, Ref, RelatedParty,
    RelatedPlace, TimePeriod, Timestamp,
};
use crate::resource::Resource;
use crate::service::Service;

use super::{ProductOffering, ProductOfferingPrice, ProductSpecification};

tmf_struct! {
    @name = "Product", @ref = "ProductRef";
    /// A product a customer actually has: an instance of an offering, in the
    /// inventory, in some state.
    ///
    /// This is the **read model** of TMF637 Product Inventory. Use
    /// [`ProductCreate`] for `POST` and [`ProductUpdate`] for `PATCH`.
    ///
    /// It is also what a TMF622 order line acts on. The v5 `ProductRefOrValue`
    /// is a `oneOf` over this type and a bare reference to one, and TMF622 and
    /// TMF637 declare the `Product` schema identically — so an order line and
    /// an inventory record are the same type here, which is the point: an order
    /// is a request to change what the customer has.
    ///
    /// The discriminator is `Product`, not `ProductRefOrValue`: the v5 schema
    /// states that `@type` belongs to the entity rather than to the `oneOf`
    /// wrapper, and its own `discriminator.mapping` admits only `Product` and
    /// `ProductRef`.
    pub struct Product {
        /// Identifier of an existing product in the inventory — TMF637.
        id: String,
        /// URI of an existing product.
        href: String,
        /// Name of the product.
        name: String,
        /// Narrative description.
        description: String,
        /// Whether the product bundles others.
        is_bundle: bool,
        /// Whether the product is customer-facing.
        is_customer_visible: bool,
        /// Serial number of the delivered product.
        product_serial_number: String,
        /// Lifecycle status of the product in the inventory.
        status: ProductStatus,
        /// When the product record was created.
        creation_date: Timestamp,
        /// When the product was ordered.
        order_date: Timestamp,
        /// When the product started being provided.
        start_date: Timestamp,
        /// When provision of the product ended.
        termination_date: Timestamp,
        /// The specification this product realises.
        product_specification: Ref<ProductSpecification>,
        /// The offering this product was bought under.
        product_offering: Ref<ProductOffering>,
        /// Configured characteristics of the product.
        product_characteristic: Vec<Characteristic>,
        /// Prices being charged for the product.
        product_price: Vec<ProductPrice>,
        /// Contractual terms attached to the product.
        product_term: Vec<ProductTerm>,
        /// Relationships to other products.
        product_relationship: Vec<ProductRelationship>,
        /// Products bundled by this one.
        product: Vec<Product>,
        /// Order lines that acted on this product.
        product_order_item: Vec<RelatedOrderItem>,
        /// Agreement lines governing the product — TMF651.
        agreement_item: Vec<AgreementItemRef>,
        /// Places the product is delivered to.
        place: Vec<RelatedPlace>,
        /// Services realising the product — TMF638.
        realizing_service: Vec<Ref<Service>>,
        /// Resources realising the product — TMF639.
        realizing_resource: Vec<Ref<Resource>>,
        /// Intent governing the product — TMF921.
        intent: Ref<Intent>,
        /// Parties related to the product.
        related_party: Vec<RelatedParty>,
        /// Account the product is billed to.
        billing_account: Ref<BillingAccount>,
        @renamed {
            /// The concrete class of the product, when this is the reference
            /// form rather than an inline description.
            "@referredType" referred_type: String,
        }
    }
}

tmf_entity!(Product);

tmf_struct! {
    @name = "Product";
    /// Body of a `POST /product` — the v5 `Product_FVO`.
    ///
    /// `creationDate` is server-owned and absent; every other member of the
    /// read model is accepted, and none is required — an inventory record can
    /// be created from as little as the offering it came from.
    pub struct ProductCreate {
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Name of the product.
        name: String,
        /// Narrative description.
        description: String,
        /// Whether the product bundles others.
        is_bundle: bool,
        /// Whether the product is customer-facing.
        is_customer_visible: bool,
        /// Serial number of the delivered product.
        product_serial_number: String,
        /// Lifecycle status of the product in the inventory.
        status: ProductStatus,
        /// When the product was ordered.
        order_date: Timestamp,
        /// When the product started being provided.
        start_date: Timestamp,
        /// When provision of the product ended.
        termination_date: Timestamp,
        /// The specification this product realises.
        product_specification: Ref<ProductSpecification>,
        /// The offering this product was bought under.
        product_offering: Ref<ProductOffering>,
        /// Configured characteristics of the product.
        product_characteristic: Vec<Characteristic>,
        /// Prices being charged for the product.
        product_price: Vec<ProductPrice>,
        /// Contractual terms attached to the product.
        product_term: Vec<ProductTerm>,
        /// Relationships to other products.
        product_relationship: Vec<ProductRelationship>,
        /// Products bundled by this one.
        product: Vec<Product>,
        /// Order lines that acted on this product.
        product_order_item: Vec<RelatedOrderItem>,
        /// Agreement lines governing the product — TMF651.
        agreement_item: Vec<AgreementItemRef>,
        /// Places the product is delivered to.
        place: Vec<RelatedPlace>,
        /// Services realising the product — TMF638.
        realizing_service: Vec<Ref<Service>>,
        /// Resources realising the product — TMF639.
        realizing_resource: Vec<Ref<Resource>>,
        /// Intent governing the product — TMF921.
        intent: Ref<Intent>,
        /// Parties related to the product.
        related_party: Vec<RelatedParty>,
        /// Account the product is billed to.
        billing_account: Ref<BillingAccount>,
    }
}

tmf_struct! {
    @name = "Product";
    /// Body of a `PATCH /product/{id}` — the v5 `Product_MVO`.
    ///
    /// `id`, `href` and `creationDate` are server-owned and absent entirely.
    pub struct ProductUpdate {
        /// Name of the product.
        name: String,
        /// Narrative description.
        description: String,
        /// Whether the product bundles others.
        is_bundle: bool,
        /// Whether the product is customer-facing.
        is_customer_visible: bool,
        /// Serial number of the delivered product.
        product_serial_number: String,
        /// Lifecycle status of the product in the inventory.
        status: ProductStatus,
        /// When the product was ordered.
        order_date: Timestamp,
        /// When the product started being provided.
        start_date: Timestamp,
        /// When provision of the product ended.
        termination_date: Timestamp,
        /// The specification this product realises.
        product_specification: Ref<ProductSpecification>,
        /// The offering this product was bought under.
        product_offering: Ref<ProductOffering>,
        /// Configured characteristics of the product.
        product_characteristic: Vec<Characteristic>,
        /// Prices being charged for the product.
        product_price: Vec<ProductPrice>,
        /// Contractual terms attached to the product.
        product_term: Vec<ProductTerm>,
        /// Relationships to other products.
        product_relationship: Vec<ProductRelationship>,
        /// Products bundled by this one.
        product: Vec<Product>,
        /// Order lines that acted on this product.
        product_order_item: Vec<RelatedOrderItem>,
        /// Agreement lines governing the product — TMF651.
        agreement_item: Vec<AgreementItemRef>,
        /// Places the product is delivered to.
        place: Vec<RelatedPlace>,
        /// Services realising the product — TMF638.
        realizing_service: Vec<Ref<Service>>,
        /// Resources realising the product — TMF639.
        realizing_resource: Vec<Ref<Resource>>,
        /// Intent governing the product — TMF921.
        intent: Ref<Intent>,
        /// Parties related to the product.
        related_party: Vec<RelatedParty>,
        /// Account the product is billed to.
        billing_account: Ref<BillingAccount>,
    }
}

tmf_patch_body!(ProductUpdate);

/// The lifecycle status of a product in the inventory.
///
/// The v5 `ProductStatusType` enumeration, with [`ProductStatus::Other`]
/// preserving a value outside it rather than failing the parse.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ProductStatus {
    /// Recorded but not yet being provisioned.
    #[serde(rename = "created")]
    Created,
    /// Provisioning is under way.
    #[serde(rename = "pendingActive")]
    PendingActive,
    /// In service.
    #[serde(rename = "active")]
    Active,
    /// Temporarily out of service.
    #[serde(rename = "suspended")]
    Suspended,
    /// Cessation is under way.
    #[serde(rename = "pendingTerminate")]
    PendingTerminate,
    /// Out of service for good.
    #[serde(rename = "terminated")]
    Terminated,
    /// Never came into service.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Provisioning was abandoned.
    ///
    /// Note the trailing space in the v5 enumeration value; it is reproduced
    /// verbatim, because a server matching the specification will send it.
    #[serde(rename = "aborted ")]
    Aborted,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

tmf_struct! {
    @name = "ProductPrice";
    /// A price being charged for a product in the inventory.
    pub struct ProductPrice {
        /// Name of the price.
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
        /// Discounts and surcharges applied.
        price_alteration: Vec<PriceAlteration>,
        /// The catalog price this was derived from.
        product_offering_price: Ref<ProductOfferingPrice>,
    }
}

tmf_struct! {
    @name = "ProductTerm";
    /// A contractual term attached to a product.
    pub struct ProductTerm {
        /// Name of the term.
        name: String,
        /// Narrative description.
        description: String,
        /// How long the term runs.
        duration: Duration,
        /// Period during which the term is valid.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "ProductRelationship";
    /// A relationship between two products.
    pub struct ProductRelationship {
        /// Identifier of the related product.
        id: String,
        /// URI of the related product.
        href: String,
        /// Name of the related product.
        name: String,
        /// Kind of relationship, e.g. `reliesOn`.
        relationship_type: String,
        @renamed {
            /// The concrete class of the related product.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "RelatedOrderItem";
    /// An order line that acted on a product.
    pub struct RelatedOrderItem {
        /// Identifier of the order the line belongs to.
        order_id: String,
        /// URI of that order.
        order_href: String,
        /// Identifier of the line within the order.
        order_item_id: String,
        /// What the line did to the product.
        order_item_action: ItemAction,
        /// The role the line plays in relation to the product.
        role: String,
        @renamed {
            /// The concrete class of the related order.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "AgreementItemRef";
    /// A line of an agreement governing a product — TMF651.
    pub struct AgreementItemRef {
        /// Identifier of the agreement.
        agreement_id: String,
        /// URI of the agreement.
        agreement_href: String,
        /// Name of the agreement.
        agreement_name: String,
        /// Identifier of the line within the agreement.
        agreement_item_id: String,
        @renamed {
            /// The concrete class of the agreement.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "PriceAlteration";
    /// A discount or surcharge modifying a [`ProductPrice`] or an
    /// [`OrderPrice`](crate::order::OrderPrice).
    pub struct PriceAlteration {
        /// Name of the alteration.
        name: String,
        /// Narrative description.
        description: String,
        /// Kind of alteration, e.g. `discount`.
        price_type: String,
        /// Where in the sequence of alterations this one applies.
        priority: i64,
        /// For recurring charges, the billing period.
        recurring_charge_period: String,
        /// How many charge periods the alteration lasts for.
        application_duration: i64,
        /// Unit the alteration applies to.
        unit_of_measure: String,
        /// The alteration amount.
        price: Price,
        /// The catalog price this alteration was derived from.
        product_offering_price: Ref<ProductOfferingPrice>,
    }
}
