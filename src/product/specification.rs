//! `ProductSpecification` — the technical definition behind an offering.

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    Attachment, CharacteristicSpecification, ExternalIdentifier, IntentSpecification, Policy, Ref,
    RelatedParty, ServiceSpecification, TimePeriod, Timestamp,
};
use crate::resource::ResourceSpecification;

use super::Category;

tmf_struct! {
    @name = "ProductSpecification", @ref = "ProductSpecificationRef";
    /// The definition of a product: what it is, independent of how it is sold.
    ///
    /// This is the **read model**. Use [`ProductSpecificationCreate`] for `POST`
    /// and [`ProductSpecificationUpdate`] for `PATCH`.
    pub struct ProductSpecification {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this specification.
        href: String,
        /// Name of the specification.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the specification.
        version: String,
        /// When the specification was last changed.
        last_update: Timestamp,
        /// Lifecycle status, e.g. `Active`, `Retired`.
        lifecycle_status: String,
        /// Whether this specification bundles others.
        is_bundle: bool,
        /// Brand under which the product is sold.
        brand: String,
        /// Product number used within the enterprise.
        product_number: String,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
        /// Characteristics that products built from this spec may carry.
        ///
        /// Note the v5 member name: `productSpecCharacteristic`, not v4's
        /// `productSpecificationCharacteristic`.
        product_spec_characteristic: Vec<CharacteristicSpecification>,
        /// Relationships to other specifications.
        product_specification_relationship: Vec<SpecificationRelationship>,
        /// Specifications bundled by this one.
        bundled_product_specification: Vec<BundledProductSpecification>,
        /// Categories this specification is filed under.
        category: Vec<Ref<Category>>,
        /// Policies constraining the specification.
        policy: Vec<Ref<Policy>>,
        /// Service specifications this product is realised by — TMF633.
        service_specification: Vec<Ref<ServiceSpecification>>,
        /// Resource specifications this product is realised by — TMF634.
        resource_specification: Vec<Ref<ResourceSpecification>>,
        /// Intent specification governing this product — TMF921.
        intent_specification: Ref<IntentSpecification>,
        /// A JSON Schema describing products built from this specification.
        target_product_schema: TargetProductSchema,
        /// Parties related to this specification, e.g. the owner.
        related_party: Vec<RelatedParty>,
        /// Attachments such as datasheets.
        attachment: Vec<Attachment>,
        /// Identifiers for this specification in external systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_entity!(ProductSpecification);

tmf_struct! {
    @name = "ProductSpecification";
    /// Body of a `POST /productSpecification` — the v5 `ProductSpecification_FVO`.
    pub struct ProductSpecificationCreate {
        @required {
            /// Name of the specification. **Required on create.**
            name: String,
            /// Lifecycle status. **Required on create.**
            lifecycle_status: String,
            /// When the specification was last changed. **Required on create.**
            last_update: Timestamp,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the specification.
        version: String,
        /// Whether this specification bundles others.
        is_bundle: bool,
        /// Brand under which the product is sold.
        brand: String,
        /// Product number used within the enterprise.
        product_number: String,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
        /// Characteristics that products built from this spec may carry.
        product_spec_characteristic: Vec<CharacteristicSpecification>,
        /// Relationships to other specifications.
        product_specification_relationship: Vec<SpecificationRelationship>,
        /// Specifications bundled by this one.
        bundled_product_specification: Vec<BundledProductSpecification>,
        /// Categories this specification is filed under.
        category: Vec<Ref<Category>>,
        /// Policies constraining the specification.
        policy: Vec<Ref<Policy>>,
        /// Service specifications this product is realised by.
        service_specification: Vec<Ref<ServiceSpecification>>,
        /// Resource specifications this product is realised by.
        resource_specification: Vec<Ref<ResourceSpecification>>,
        /// Intent specification governing this product.
        intent_specification: Ref<IntentSpecification>,
        /// A JSON Schema describing products built from this specification.
        target_product_schema: TargetProductSchema,
        /// Parties related to this specification.
        related_party: Vec<RelatedParty>,
        /// Attachments such as datasheets.
        attachment: Vec<Attachment>,
        /// Identifiers for this specification in external systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ProductSpecification";
    /// Body of a `PATCH /productSpecification/{id}` — the v5 `_MVO` schema.
    pub struct ProductSpecificationUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New bundle flag.
        is_bundle: bool,
        /// New brand.
        brand: String,
        /// New product number.
        product_number: String,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement characteristics.
        product_spec_characteristic: Vec<CharacteristicSpecification>,
        /// Replacement relationships.
        product_specification_relationship: Vec<SpecificationRelationship>,
        /// Replacement bundled specifications.
        bundled_product_specification: Vec<BundledProductSpecification>,
        /// Replacement categories.
        category: Vec<Ref<Category>>,
        /// Replacement policies.
        policy: Vec<Ref<Policy>>,
        /// Replacement service specifications.
        service_specification: Vec<Ref<ServiceSpecification>>,
        /// Replacement resource specifications.
        resource_specification: Vec<Ref<ResourceSpecification>>,
        /// Replacement intent specification.
        intent_specification: Ref<IntentSpecification>,
        /// Replacement target product schema.
        target_product_schema: TargetProductSchema,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement attachments.
        attachment: Vec<Attachment>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ProductSpecificationRelationship";
    /// A relationship from one product specification to another.
    pub struct SpecificationRelationship {
        /// Identifier of the related specification.
        id: String,
        /// URI of the related specification.
        href: String,
        /// Name of the related specification.
        name: String,
        /// Kind of relationship, e.g. `dependency`, `exclusivity`.
        relationship_type: String,
        /// Version of the related specification.
        version: String,
        /// Characteristics that qualify the relationship.
        characteristic: Vec<CharacteristicSpecification>,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
        @renamed {
            /// The concrete class of the related specification.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "BundledProductSpecification";
    /// A specification included in a bundled specification.
    pub struct BundledProductSpecification {
        /// Identifier of the bundled specification.
        id: String,
        /// URI of the bundled specification.
        href: String,
        /// Name of the bundled specification.
        name: String,
        /// Version of the bundled specification.
        version: String,
        /// Lifecycle status of the bundled specification.
        lifecycle_status: String,
    }
}

/// A JSON Schema describing the products built from a specification.
///
/// The v5 schema defines this with `@type` and `@schemaLocation` and nothing
/// else: the point of the object is the schema URI it carries. That is why it
/// is written out rather than declared with the usual macro — it is the one
/// entity with no `@baseType`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, bon::Builder)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct TargetProductSchema {
    /// The `@type` of the schema being pointed at.
    #[serde(rename = "@type", default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub at_type: Option<String>,
    /// A URI to the JSON-Schema file.
    #[serde(
        rename = "@schemaLocation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    #[builder(into)]
    pub at_schema_location: Option<String>,
    /// Members not covered by the typed model, kept in document order.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "crate::core::Extensions::is_empty"
    )]
    #[builder(default)]
    pub extensions: crate::core::Extensions,
}

tmf_patch_body!(ProductSpecificationUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_v5_characteristic_member_is_product_spec_characteristic() {
        // v4 spelled it `productSpecificationCharacteristic`. Carrying that
        // name into a v5 type meant every characteristic a v5 server sent was
        // swept into `extensions` and no typed access existed.
        let json = r#"{"@type":"ProductSpecification","productSpecCharacteristic":[{"name":"Colour","@type":"CharacteristicSpecification"}]}"#;
        let spec: ProductSpecification = serde_json::from_str(json).unwrap();
        assert_eq!(spec.product_spec_characteristic.as_ref().unwrap().len(), 1);
        assert!(spec.extensions.is_empty());
    }
}
