//! `ProductOfferingPrice` and the terms attached to an offering.

use rust_decimal::Decimal;

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    CharacteristicValueUse, Duration, ExternalIdentifier, Money, Place, Policy, Quantity, Ref,
    TaxItem, TimePeriod, Timestamp,
};

tmf_struct! {
    @name = "ProductOfferingPrice", @ref = "ProductOfferingPriceRef";
    /// A price at which an offering is sold.
    ///
    /// This is the **read model**. Use [`ProductOfferingPriceCreate`] for `POST`
    /// and [`ProductOfferingPriceUpdate`] for `PATCH`.
    pub struct ProductOfferingPrice {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this price.
        href: String,
        /// Name of the price.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the price.
        version: String,
        /// When the price was last changed.
        last_update: Timestamp,
        /// Lifecycle status, e.g. `Active`.
        lifecycle_status: String,
        /// Kind of price, e.g. `recurring`, `oneTime`, `usage`.
        price_type: String,
        /// For recurring prices, the billing frequency, e.g. `monthly`.
        recurring_charge_period_type: String,
        /// For recurring prices, the number of periods per charge.
        recurring_charge_period_length: i64,
        /// Whether this price bundles others.
        is_bundle: bool,
        /// The amount charged.
        price: Money,
        /// Period during which the price is valid.
        valid_for: TimePeriod,
        /// Unit of measure the price applies to.
        unit_of_measure: Quantity,
        /// Places where the price applies.
        place: Vec<Ref<Place>>,
        /// Policies constraining the price.
        policy: Vec<Ref<Policy>>,
        /// Prices bundled by this one.
        bundled_pop_relationship: Vec<BundledPriceRelationship>,
        /// Prices that alter this one, e.g. discounts.
        pop_relationship: Vec<PriceRelationship>,
        /// Characteristic uses narrowing when the price applies.
        prod_spec_char_value_use: Vec<CharacteristicValueUse>,
        /// Terms attached to this price.
        product_offering_term: Vec<ProductOfferingTerm>,
        /// Algorithms computing the price where a fixed amount will not do.
        pricing_logic_algorithm: Vec<PricingLogicAlgorithm>,
        /// Tax applied to the price.
        tax: Vec<TaxItem>,
        /// Identifiers for this price in external systems.
        external_identifier: Vec<ExternalIdentifier>,
        @decimal {
            /// Percentage applied instead of an absolute amount.
            percentage: Decimal,
        }
    }
}

tmf_entity!(ProductOfferingPrice);

tmf_struct! {
    @name = "ProductOfferingPrice";
    /// Body of a `POST /productOfferingPrice` — the v5 `ProductOfferingPrice_FVO`.
    ///
    /// `name`, `lifecycleStatus`, `lastUpdate` and `priceType` are required on
    /// create, so they are non-optional here.
    pub struct ProductOfferingPriceCreate {
        @required {
            /// Name of the price. **Required on create.**
            name: String,
            /// Lifecycle status. **Required on create.**
            lifecycle_status: String,
            /// Kind of price. **Required on create.**
            price_type: String,
            /// When the price was last changed. **Required on create.**
            last_update: Timestamp,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the price.
        version: String,
        /// For recurring prices, the billing frequency.
        recurring_charge_period_type: String,
        /// For recurring prices, the number of periods per charge.
        recurring_charge_period_length: i64,
        /// Whether this price bundles others.
        is_bundle: bool,
        /// The amount charged.
        price: Money,
        /// Period during which the price is valid.
        valid_for: TimePeriod,
        /// Unit of measure the price applies to.
        unit_of_measure: Quantity,
        /// Places where the price applies.
        place: Vec<Ref<Place>>,
        /// Policies constraining the price.
        policy: Vec<Ref<Policy>>,
        /// Prices bundled by this one.
        bundled_pop_relationship: Vec<BundledPriceRelationship>,
        /// Prices that alter this one.
        pop_relationship: Vec<PriceRelationship>,
        /// Characteristic uses narrowing when the price applies.
        prod_spec_char_value_use: Vec<CharacteristicValueUse>,
        /// Terms attached to this price.
        product_offering_term: Vec<ProductOfferingTerm>,
        /// Algorithms computing the price.
        pricing_logic_algorithm: Vec<PricingLogicAlgorithm>,
        /// Tax applied to the price.
        tax: Vec<TaxItem>,
        /// Identifiers for this price in external systems.
        external_identifier: Vec<ExternalIdentifier>,
        @decimal {
            /// Percentage applied instead of an absolute amount.
            percentage: Decimal,
        }
    }
}

tmf_struct! {
    @name = "ProductOfferingPrice";
    /// Body of a `PATCH /productOfferingPrice/{id}` — the v5 `_MVO` schema.
    pub struct ProductOfferingPriceUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New price type.
        price_type: String,
        /// New recurring billing frequency.
        recurring_charge_period_type: String,
        /// New number of periods per charge.
        recurring_charge_period_length: i64,
        /// New bundle flag.
        is_bundle: bool,
        /// New amount.
        price: Money,
        /// New validity period.
        valid_for: TimePeriod,
        /// New unit of measure.
        unit_of_measure: Quantity,
        /// Replacement place list.
        place: Vec<Ref<Place>>,
        /// Replacement policy list.
        policy: Vec<Ref<Policy>>,
        /// Replacement bundled prices.
        bundled_pop_relationship: Vec<BundledPriceRelationship>,
        /// Replacement price relationships.
        pop_relationship: Vec<PriceRelationship>,
        /// Replacement characteristic value uses.
        prod_spec_char_value_use: Vec<CharacteristicValueUse>,
        /// Replacement terms.
        product_offering_term: Vec<ProductOfferingTerm>,
        /// Replacement pricing algorithms.
        pricing_logic_algorithm: Vec<PricingLogicAlgorithm>,
        /// Replacement tax items.
        tax: Vec<TaxItem>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
        @decimal {
            /// New percentage.
            percentage: Decimal,
        }
    }
}

tmf_struct! {
    @name = "ProductOfferingPriceRelationship";
    /// A relationship between two offering prices, e.g. a discount that alters
    /// another price.
    pub struct PriceRelationship {
        /// Identifier of the related price.
        id: String,
        /// URI of the related price.
        href: String,
        /// Name of the related price.
        name: String,
        /// The role the related price plays.
        role: String,
        /// Kind of relationship, e.g. `discount`, `alteration`.
        relationship_type: String,
        /// Version of the related price.
        version: String,
        @renamed {
            /// The concrete class of the related price.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "BundledProductOfferingPriceRelationship";
    /// A price included in a bundled price.
    pub struct BundledPriceRelationship {
        /// Identifier of the bundled price.
        id: String,
        /// URI of the bundled price.
        href: String,
        /// Name of the bundled price.
        name: String,
        /// Version of the bundled price.
        version: String,
        @renamed {
            /// The concrete class of the bundled price.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "PricingLogicAlgorithm";
    /// An algorithm that computes a price where a fixed amount will not do —
    /// tiered usage rating, for example.
    pub struct PricingLogicAlgorithm {
        /// Identifier of the algorithm instance.
        id: String,
        /// URI of the algorithm instance.
        href: String,
        /// Name of the algorithm.
        name: String,
        /// Narrative description.
        description: String,
        /// Identifier of the algorithm specification in the rating engine.
        pla_spec_id: String,
        /// Period during which the algorithm applies.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "ProductOfferingTerm";
    /// A contractual term attached to an offering, e.g. a 24-month commitment.
    pub struct ProductOfferingTerm {
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

tmf_patch_body!(ProductOfferingPriceUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_v5_relationship_member_is_pop_not_popr() {
        // v4 spelled it `poprRelationship`; carrying that name into a v5 type
        // silently dropped every price relationship a v5 server sent.
        let json = r#"{"@type":"ProductOfferingPrice","popRelationship":[{"id":"1","@type":"ProductOfferingPriceRelationship"}]}"#;
        let price: ProductOfferingPrice = serde_json::from_str(json).unwrap();
        assert_eq!(price.pop_relationship.as_ref().unwrap().len(), 1);
        assert!(price.extensions.is_empty());
    }
}
