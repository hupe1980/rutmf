//! `ProductOffering` — what a service provider actually sells.
//!
//! Mirrors the TMF620 v5 `ProductOffering`, `ProductOffering_FVO` and
//! `ProductOffering_MVO` schemas.

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    Agreement, Attachment, Channel, CharacteristicSpecification, CharacteristicValueUse,
    ExternalIdentifier, MarketSegment, Place, Policy, Ref, ServiceCandidate, ServiceLevelAgreement,
    TimePeriod, Timestamp,
};
use crate::resource::ResourceCandidate;

use super::{Category, ProductOfferingPrice, ProductOfferingTerm, ProductSpecification};

tmf_struct! {
    @name = "ProductOffering", @ref = "ProductOfferingRef";
    /// A product offering: a product specification made sellable through a
    /// channel, at a price, in a market.
    ///
    /// This is the **read model**, returned by `GET`. To create one use
    /// [`ProductOfferingCreate`]; to modify one use [`ProductOfferingUpdate`].
    ///
    /// ```
    /// use rutmf::product::ProductOffering;
    ///
    /// let offering = ProductOffering::builder()
    ///     .name("Business Internet")
    ///     .is_sellable(true)
    ///     .build();
    /// assert_eq!(offering.at_type, "ProductOffering");
    /// ```
    pub struct ProductOffering {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this offering.
        href: String,
        /// Name of the offering.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the offering.
        version: String,
        /// When the offering was last changed.
        last_update: Timestamp,
        /// Lifecycle status, e.g. `Active`, `Retired`.
        lifecycle_status: String,
        /// Reason for the current status.
        status_reason: String,
        /// Period during which the offering is valid.
        valid_for: TimePeriod,
        /// Whether this offering bundles others.
        is_bundle: bool,
        /// Whether the offering can be sold on its own.
        is_sellable: bool,
        /// The product specification realised by this offering.
        product_specification: Ref<ProductSpecification>,
        /// Prices at which the offering is sold.
        ///
        /// The v5 schema types this as `ProductOfferingPriceRefOrValue`: an
        /// entry may be the whole price or a bare reference to one. Both are a
        /// subset of [`ProductOfferingPrice`], so one type covers each.
        product_offering_price: Vec<ProductOfferingPrice>,
        /// Categories the offering is filed under.
        category: Vec<Ref<Category>>,
        /// Sales channels through which the offering is available.
        channel: Vec<Ref<Channel>>,
        /// Geographic places where the offering applies.
        place: Vec<Ref<Place>>,
        /// Market segments targeted by the offering.
        market_segment: Vec<Ref<MarketSegment>>,
        /// Policies constraining the offering.
        policy: Vec<Ref<Policy>>,
        /// Agreements the offering is sold under — TMF651.
        agreement: Vec<Ref<Agreement>>,
        /// Service level agreement attached to the offering.
        service_level_agreement: Ref<ServiceLevelAgreement>,
        /// The service catalog entry this offering is realised by — TMF633.
        service_candidate: Ref<ServiceCandidate>,
        /// The resource catalog entry this offering is realised by — TMF634.
        resource_candidate: Ref<ResourceCandidate>,
        /// Terms of sale.
        product_offering_term: Vec<ProductOfferingTerm>,
        /// Characteristics defined directly on the offering.
        product_offering_characteristic: Vec<CharacteristicSpecification>,
        /// Narrowed uses of characteristics defined on the specification.
        prod_spec_char_value_use: Vec<CharacteristicValueUse>,
        /// Relationships to other offerings.
        product_offering_relationship: Vec<OfferingRelationship>,
        /// Offerings bundled by this one.
        bundled_product_offering: Vec<BundledProductOffering>,
        /// Groups of alternative bundled offerings.
        bundled_group_product_offering: Vec<BundledGroupProductOffering>,
        /// Actions permitted on products created from this offering.
        allowed_action: Vec<AllowedProductAction>,
        /// Attachments such as brochures or images.
        attachment: Vec<Attachment>,
        /// Identifiers for this offering in external systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_entity!(ProductOffering);

tmf_struct! {
    @name = "ProductOffering";
    /// Body of a `POST /productOffering` — the TMF620 v5 `ProductOffering_FVO`.
    ///
    /// The members the spec marks required on create are non-optional here, so
    /// a request a conformant server would reject does not compile.
    ///
    /// ```
    /// use rutmf::product::ProductOfferingCreate;
    /// use chrono::Utc;
    ///
    /// let body = ProductOfferingCreate::builder()
    ///     .name("Business Internet")
    ///     .lifecycle_status("Active")
    ///     .last_update(Utc::now())
    ///     .is_sellable(true)
    ///     .build();
    /// assert_eq!(body.name, "Business Internet");
    /// ```
    pub struct ProductOfferingCreate {
        @required {
            /// Name of the offering. **Required on create.**
            name: String,
            /// Lifecycle status. **Required on create.**
            lifecycle_status: String,
            /// When the offering was last changed. **Required on create.**
            last_update: Timestamp,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the offering.
        version: String,
        /// Reason for the initial status.
        status_reason: String,
        /// Period during which the offering is valid.
        valid_for: TimePeriod,
        /// Whether this offering bundles others.
        is_bundle: bool,
        /// Whether the offering can be sold on its own.
        is_sellable: bool,
        /// The product specification realised by this offering.
        product_specification: Ref<ProductSpecification>,
        /// Prices at which the offering is sold.
        product_offering_price: Vec<ProductOfferingPrice>,
        /// Categories the offering is filed under.
        category: Vec<Ref<Category>>,
        /// Sales channels through which the offering is available.
        channel: Vec<Ref<Channel>>,
        /// Geographic places where the offering applies.
        place: Vec<Ref<Place>>,
        /// Market segments targeted by the offering.
        market_segment: Vec<Ref<MarketSegment>>,
        /// Policies constraining the offering.
        policy: Vec<Ref<Policy>>,
        /// Agreements the offering is sold under.
        agreement: Vec<Ref<Agreement>>,
        /// Service level agreement attached to the offering.
        service_level_agreement: Ref<ServiceLevelAgreement>,
        /// The service catalog entry this offering is realised by.
        service_candidate: Ref<ServiceCandidate>,
        /// The resource catalog entry this offering is realised by.
        resource_candidate: Ref<ResourceCandidate>,
        /// Terms of sale.
        product_offering_term: Vec<ProductOfferingTerm>,
        /// Characteristics defined directly on the offering.
        product_offering_characteristic: Vec<CharacteristicSpecification>,
        /// Narrowed uses of characteristics defined on the specification.
        prod_spec_char_value_use: Vec<CharacteristicValueUse>,
        /// Relationships to other offerings.
        product_offering_relationship: Vec<OfferingRelationship>,
        /// Offerings bundled by this one.
        bundled_product_offering: Vec<BundledProductOffering>,
        /// Groups of alternative bundled offerings.
        bundled_group_product_offering: Vec<BundledGroupProductOffering>,
        /// Actions permitted on products created from this offering.
        allowed_action: Vec<AllowedProductAction>,
        /// Attachments such as brochures or images.
        attachment: Vec<Attachment>,
        /// Identifiers for this offering in external systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ProductOffering";
    /// Body of a `PATCH /productOffering/{id}` — the v5 `ProductOffering_MVO`.
    ///
    /// Every member is optional (a patch changes only what it names) and the
    /// server-owned members — `id`, `href`, `lastUpdate` — are absent entirely,
    /// so they cannot be sent by accident.
    ///
    /// ```
    /// use rutmf::product::ProductOfferingUpdate;
    ///
    /// let patch = ProductOfferingUpdate::builder()
    ///     .lifecycle_status("Retired")
    ///     .build();
    ///
    /// // Only what you named is sent — plus the mandatory discriminator.
    /// assert_eq!(
    ///     serde_json::to_string(&patch).unwrap(),
    ///     r#"{"lifecycleStatus":"Retired","@type":"ProductOffering"}"#,
    /// );
    /// ```
    pub struct ProductOfferingUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New status reason.
        status_reason: String,
        /// New validity period.
        valid_for: TimePeriod,
        /// New bundle flag.
        is_bundle: bool,
        /// New sellable flag.
        is_sellable: bool,
        /// Replacement product specification.
        product_specification: Ref<ProductSpecification>,
        /// Replacement price list.
        product_offering_price: Vec<ProductOfferingPrice>,
        /// Replacement category list.
        category: Vec<Ref<Category>>,
        /// Replacement channel list.
        channel: Vec<Ref<Channel>>,
        /// Replacement place list.
        place: Vec<Ref<Place>>,
        /// Replacement market segment list.
        market_segment: Vec<Ref<MarketSegment>>,
        /// Replacement policy list.
        policy: Vec<Ref<Policy>>,
        /// Replacement agreement list.
        agreement: Vec<Ref<Agreement>>,
        /// Replacement service level agreement.
        service_level_agreement: Ref<ServiceLevelAgreement>,
        /// Replacement service catalog entry.
        service_candidate: Ref<ServiceCandidate>,
        /// Replacement resource catalog entry.
        resource_candidate: Ref<ResourceCandidate>,
        /// Replacement terms.
        product_offering_term: Vec<ProductOfferingTerm>,
        /// Replacement offering characteristics.
        product_offering_characteristic: Vec<CharacteristicSpecification>,
        /// Replacement characteristic value uses.
        prod_spec_char_value_use: Vec<CharacteristicValueUse>,
        /// Replacement relationships.
        product_offering_relationship: Vec<OfferingRelationship>,
        /// Replacement bundled offerings.
        bundled_product_offering: Vec<BundledProductOffering>,
        /// Replacement bundled offering groups.
        bundled_group_product_offering: Vec<BundledGroupProductOffering>,
        /// Replacement allowed actions.
        allowed_action: Vec<AllowedProductAction>,
        /// Replacement attachments.
        attachment: Vec<Attachment>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ProductOfferingRelationship";
    /// A relationship from one offering to another.
    pub struct OfferingRelationship {
        /// Identifier of the related offering.
        id: String,
        /// URI of the related offering.
        href: String,
        /// Name of the related offering.
        name: String,
        /// The role the related offering plays.
        role: String,
        /// Kind of relationship, e.g. `substitute`, `dependency`.
        relationship_type: String,
        /// Version of the related offering.
        version: String,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
        @renamed {
            /// The concrete class of the related offering.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "BundledProductOffering";
    /// An offering included in a bundle, with the cardinality it is included at.
    ///
    /// Distinct from a plain `ProductOfferingRef`: the v5 schema adds
    /// `bundledProductOfferingOption`, which is how a bundle expresses "pick
    /// two to four of these".
    pub struct BundledProductOffering {
        /// Identifier of the bundled offering.
        id: String,
        /// URI of the bundled offering.
        href: String,
        /// Name of the bundled offering.
        name: String,
        /// Version of the bundled offering.
        version: String,
        /// How many instances of this offering the bundle admits.
        bundled_product_offering_option: BundledProductOfferingOption,
        @renamed {
            /// The concrete class of the bundled offering.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "BundledProductOfferingOption";
    /// Cardinality bounds on one offering inside a bundle.
    pub struct BundledProductOfferingOption {
        /// How many instances are selected unless the buyer says otherwise.
        number_rel_offer_default: i64,
        /// Fewest instances the bundle admits.
        number_rel_offer_lower_limit: i64,
        /// Most instances the bundle admits.
        number_rel_offer_upper_limit: i64,
    }
}

tmf_struct! {
    @name = "BundledGroupProductOffering";
    /// A group of offerings within a bundle, chosen between as a unit.
    pub struct BundledGroupProductOffering {
        /// Identifier of the group.
        id: String,
        /// Name of the group.
        name: String,
        /// Offerings in this group.
        bundled_product_offering: Vec<BundledProductOffering>,
        /// Nested groups.
        bundled_group_product_offering: Vec<BundledGroupProductOffering>,
        /// How many members of the group the bundle admits.
        bundled_group_product_offering_option: BundledGroupProductOfferingOption,
    }
}

tmf_struct! {
    @name = "BundledGroupProductOfferingOption";
    /// Cardinality bounds on a group of offerings inside a bundle.
    pub struct BundledGroupProductOfferingOption {
        /// Fewest members the bundle admits.
        number_rel_offer_lower_limit: i64,
        /// Most members the bundle admits.
        number_rel_offer_upper_limit: i64,
    }
}

tmf_struct! {
    @name = "AllowedProductAction";
    /// An action permitted on products created from an offering, e.g. `resume`.
    pub struct AllowedProductAction {
        /// The permitted action.
        action: String,
        /// Channels the action is permitted through.
        channel: Vec<Ref<Channel>>,
        /// Period during which the action is permitted.
        valid_for: TimePeriod,
    }
}

tmf_patch_body!(ProductOfferingUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patch_body_defaults_to_a_valid_discriminator() {
        let patch = ProductOfferingUpdate::default();
        assert_eq!(patch.at_type, "ProductOffering");
        assert_eq!(
            serde_json::to_string(&patch).unwrap(),
            r#"{"@type":"ProductOffering"}"#
        );
    }

    #[test]
    fn a_bundle_keeps_its_cardinality_option() {
        let json = r#"{"id":"1","@type":"BundledProductOffering","bundledProductOfferingOption":{"numberRelOfferLowerLimit":2,"numberRelOfferUpperLimit":4,"@type":"BundledProductOfferingOption"}}"#;
        let bundled: BundledProductOffering = serde_json::from_str(json).unwrap();
        let option = bundled.bundled_product_offering_option.as_ref().unwrap();
        assert_eq!(option.number_rel_offer_lower_limit, Some(2));
        assert!(
            bundled.extensions.is_empty(),
            "the option must be typed, not swept into extensions"
        );
        assert_eq!(
            serde_json::to_value(&bundled).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }
}
