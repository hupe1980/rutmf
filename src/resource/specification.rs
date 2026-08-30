//! Resource specifications — TMF634.
//!
//! A [`ResourceSpecification`] describes what a resource *is*, independently of
//! any instance: a port type, a virtual-machine flavour, a SIM model. The
//! [`Resource`](super::Resource)s in the inventory (TMF639) point back at one.

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct, tmf_value};
use crate::core::{
    Attachment, CharacteristicSpecification, ConnectionPointSpecification, EndpointSpecification,
    ExternalIdentifier, FeatureRelationshipType, IntentSpecification, Policy, Ref, RelatedParty,
    TimePeriod, Timestamp,
};
use serde::{Deserialize, Serialize};

tmf_struct! {
    @name = "ResourceSpecification", @ref = "ResourceSpecificationRef";
    /// What a class of resources is made of, and what it can be configured to do.
    ///
    /// # One type, four schemas
    ///
    /// TMF634 models this as an abstract base with three `@type`-discriminated
    /// subclasses. `PhysicalResourceSpecification` adds the members that only
    /// make sense for hardware (`model`, `part`, `sku`, `vendor`);
    /// `ResourceFunctionSpecification` adds the connectivity members that only
    /// make sense for a network function; `LogicalResourceSpecification` adds
    /// nothing at all.
    ///
    /// Rather than four near-identical Rust structs, this keeps every member
    /// optional on one type and exposes [`kind`] to recover which subclass the
    /// server sent — so an unrecognised vendor subclass never fails a parse.
    ///
    /// ```
    /// use rutmf::resource::{ResourceSpecification, ResourceSpecificationKind};
    ///
    /// let json = r#"{"@type":"PhysicalResourceSpecification","vendor":"Acme","sku":"X1"}"#;
    /// let spec: ResourceSpecification = serde_json::from_str(json).unwrap();
    ///
    /// assert_eq!(spec.kind(), ResourceSpecificationKind::Physical);
    /// assert_eq!(spec.vendor.as_deref(), Some("Acme"));
    /// ```
    ///
    /// [`kind`]: ResourceSpecification::kind
    pub struct ResourceSpecification {
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
        /// Category this specification belongs to.
        category: String,
        /// When the specification was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Whether this specification bundles others.
        is_bundle: bool,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
        /// Attachments describing the specification.
        attachment: Vec<Attachment>,
        /// Characteristics a resource of this class carries.
        resource_spec_characteristic: Vec<CharacteristicSpecification>,
        /// Relationships to other specifications.
        resource_spec_relationship: Vec<ResourceSpecificationRelationship>,
        /// Features this specification offers.
        feature_specification: Vec<FeatureSpecification>,
        /// Schema of the resource this specification targets.
        target_resource_schema: TargetResourceSchema,
        /// The intent this specification realises — TMF921.
        intent_specification: Ref<IntentSpecification>,
        /// Parties related to the specification.
        related_party: Vec<RelatedParty>,
        /// Identifiers this specification is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,

        /// Manufacturer's model number — `PhysicalResourceSpecification`.
        model: String,
        /// Part number — `PhysicalResourceSpecification`.
        part: String,
        /// Stock-keeping unit — `PhysicalResourceSpecification`.
        sku: String,
        /// Manufacturer — `PhysicalResourceSpecification`.
        vendor: String,

        /// Connection points this function exposes — `ResourceFunctionSpecification`.
        connection_point_specification: Vec<Ref<ConnectionPointSpecification>>,
        /// Internal connectivity of this function — `ResourceFunctionSpecification`.
        connectivity_specification: Vec<ResourceGraphSpecification>,
        @renamed {
            /// The concrete class a `ResourceSpecificationRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

impl ResourceSpecification {
    /// Recovers the subclass implied by `@type`.
    #[must_use]
    pub fn kind(&self) -> ResourceSpecificationKind {
        ResourceSpecificationKind::from_type_name(self.type_name())
    }
}

/// The subclass of a [`ResourceSpecification`], recovered from its `@type`.
///
/// Mirrors the entries of the v5 discriminator mapping, plus
/// [`ResourceSpecificationKind::Other`] so a vendor subclass never fails to
/// parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResourceSpecificationKind {
    /// The abstract base, carrying no subclass-specific member.
    Base,
    /// A logical resource — adds no member of its own.
    Logical,
    /// A physical resource: `model`, `part`, `sku`, `vendor`.
    Physical,
    /// A network function: connection points and internal connectivity.
    Function,
    /// A subclass this crate does not know.
    Other,
}

impl ResourceSpecificationKind {
    /// Every subclass the v5 documents declare, base first.
    ///
    /// Excludes [`Other`](Self::Other), which stands for a class the documents
    /// do *not* declare. Checked against the specification's own
    /// `discriminator.mapping` by `every_subclass_enumeration_is_the_declared_mapping`
    /// in `tests/coverage.rs`.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::Base, Self::Logical, Self::Physical, Self::Function]
    }

    /// Maps a `@type` value to its kind; unknown names become [`Self::Other`].
    #[must_use]
    pub fn from_type_name(name: &str) -> Self {
        match name {
            "ResourceSpecification" => Self::Base,
            "LogicalResourceSpecification" => Self::Logical,
            "PhysicalResourceSpecification" => Self::Physical,
            "ResourceFunctionSpecification" => Self::Function,
            _ => Self::Other,
        }
    }

    /// The canonical `@type` for this kind.
    ///
    /// [`Self::Other`] has no canonical name and maps to the abstract base.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Base | Self::Other => "ResourceSpecification",
            Self::Logical => "LogicalResourceSpecification",
            Self::Physical => "PhysicalResourceSpecification",
            Self::Function => "ResourceFunctionSpecification",
        }
    }
}

tmf_struct! {
    @name = "ResourceSpecification";
    /// Body of a `POST /resourceSpecification` — the v5 `_FVO` schema.
    pub struct ResourceSpecificationCreate {
        @required {
            /// Name of the specification. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the specification.
        version: String,
        /// Category this specification belongs to.
        category: String,
        /// When the specification was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Whether this specification bundles others.
        is_bundle: bool,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
        /// Attachments describing the specification.
        attachment: Vec<Attachment>,
        /// Characteristics a resource of this class carries.
        resource_spec_characteristic: Vec<CharacteristicSpecification>,
        /// Relationships to other specifications.
        resource_spec_relationship: Vec<ResourceSpecificationRelationship>,
        /// Features this specification offers.
        feature_specification: Vec<FeatureSpecification>,
        /// Schema of the resource this specification targets.
        target_resource_schema: TargetResourceSchema,
        /// The intent this specification realises.
        intent_specification: Ref<IntentSpecification>,
        /// Parties related to the specification.
        related_party: Vec<RelatedParty>,
        /// Identifiers this specification is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ResourceSpecification";
    /// Body of a `PATCH /resourceSpecification/{id}` — the v5 `_MVO` schema.
    pub struct ResourceSpecificationUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New category.
        category: String,
        /// New last-changed timestamp.
        last_update: Timestamp,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New bundle flag.
        is_bundle: bool,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement attachments.
        attachment: Vec<Attachment>,
        /// Replacement characteristics.
        resource_spec_characteristic: Vec<CharacteristicSpecification>,
        /// Replacement relationships.
        resource_spec_relationship: Vec<ResourceSpecificationRelationship>,
        /// Replacement features.
        feature_specification: Vec<FeatureSpecification>,
        /// New target schema.
        target_resource_schema: TargetResourceSchema,
        /// New intent specification.
        intent_specification: Ref<IntentSpecification>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ResourceSpecificationRelationship";
    /// A typed link from one resource specification to another.
    pub struct ResourceSpecificationRelationship {
        /// Identifier of the referenced specification.
        id: String,
        /// URI of the referenced specification.
        href: String,
        /// Name of the referenced specification.
        name: String,
        /// What kind of link this is, e.g. `dependency`.
        relationship_type: String,
        /// The role the target plays in this relationship.
        role: String,
        /// Default number of targets, where the link is quantified.
        default_quantity: i64,
        /// Maximum number of targets.
        maximum_quantity: i64,
        /// Minimum number of targets.
        minimum_quantity: i64,
        /// Characteristics qualifying the relationship.
        characteristic: Vec<CharacteristicSpecification>,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "FeatureSpecification";
    /// A configurable feature a resource specification offers.
    pub struct FeatureSpecification {
        /// Identifier of the feature.
        id: String,
        /// Name of the feature.
        name: String,
        /// Version of the feature.
        version: String,
        /// Whether this feature bundles others.
        is_bundle: bool,
        /// Whether the feature is enabled by default.
        is_enabled: bool,
        /// Characteristics the feature carries.
        feature_spec_characteristic: Vec<CharacteristicSpecification>,
        /// Relationships to other features.
        feature_spec_relationship: Vec<FeatureSpecificationRelationship>,
        /// Policies constraining the feature.
        policy_constraint: Vec<Ref<Policy>>,
        /// Period during which the feature is valid.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "FeatureSpecificationRelationship";
    /// A link between two feature specifications.
    pub struct FeatureSpecificationRelationship {
        /// Identifier of the referenced feature.
        feature_id: String,
        /// Name of the referenced feature.
        name: String,
        /// Identifier of the specification owning the referenced feature.
        parent_specification_id: String,
        /// URI of the specification owning the referenced feature.
        parent_specification_href: String,
        /// What kind of link this is.
        relationship_type: FeatureRelationshipType,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "ResourceGraphSpecification";
    /// The internal connectivity of a resource function.
    pub struct ResourceGraphSpecification {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this graph specification.
        href: String,
        /// Name of the graph.
        name: String,
        /// Narrative description.
        description: String,
        /// Connections making up the graph.
        connection_specification: Vec<ConnectionSpecification>,
        /// Links to other graph specifications.
        graph_specification_relationship: Vec<ResourceGraphSpecificationRelationship>,
    }
}

/// How a [`ConnectionSpecification`] joins its endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ConnectionAssociationType {
    /// Exactly two endpoints.
    #[serde(rename = "pointtoPoint")]
    PointToPoint,
    /// One endpoint to many.
    #[serde(rename = "pointtoMultipoint")]
    PointToMultipoint,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

tmf_struct! {
    @name = "ConnectionSpecification";
    /// One connection within a [`ResourceGraphSpecification`].
    pub struct ConnectionSpecification {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this connection specification.
        href: String,
        /// Name of the connection.
        name: String,
        /// How the endpoints associate.
        association_type: ConnectionAssociationType,
        /// The endpoints this connection joins.
        endpoint_specification: Vec<Ref<EndpointSpecification>>,
    }
}

/// How two resource graph specifications relate.
///
/// Not the same vocabulary as [`FeatureRelationshipType`] despite the shared
/// member name — TMF634 gives graph relationships their own two values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceGraphRelationshipType {
    /// The two graphs touch.
    #[serde(rename = "adjacency")]
    Adjacency,
    /// The two graphs are joined.
    #[serde(rename = "connectivity")]
    Connectivity,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

tmf_value! {
    /// A link between two resource graph specifications.
    ///
    /// Declared by TMF634 with no `@type` of its own, which is why it is a
    /// value object rather than an entity.
    pub struct ResourceGraphSpecificationRelationship {
        /// What kind of link this is.
        relationship_type: ResourceGraphRelationshipType,
        /// The graph specification being referenced.
        resource_graph: Ref<ResourceGraphSpecification>,
    }
}

tmf_value! {
    /// A pointer to the schema describing the resource a specification targets.
    pub struct TargetResourceSchema {
        @renamed {
            /// The class the schema describes.
            "@type" at_type: String,
            /// URI of the schema document.
            "@schemaLocation" at_schema_location: String,
        }
    }
}

tmf_entity!(ResourceSpecification);
tmf_patch_body!(ResourceSpecificationUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subclass_members_sit_at_the_top_level() {
        let json =
            r#"{"@type":"PhysicalResourceSpecification","vendor":"Acme","sku":"X1","part":"P9"}"#;
        let spec: ResourceSpecification = serde_json::from_str(json).unwrap();

        assert_eq!(spec.kind(), ResourceSpecificationKind::Physical);
        assert_eq!(spec.vendor.as_deref(), Some("Acme"));
        assert!(spec.extensions.is_empty(), "subclass members must be typed");
    }

    #[test]
    fn an_unknown_subclass_round_trips_as_other() {
        let json = r#"{"@type":"VendorResourceSpecification","name":"n","quirk":1}"#;
        let spec: ResourceSpecification = serde_json::from_str(json).unwrap();

        assert_eq!(spec.kind(), ResourceSpecificationKind::Other);
        assert_eq!(spec.extensions.get("quirk").unwrap(), 1);
        assert_eq!(
            serde_json::to_value(&spec).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn every_declared_kind_round_trips_through_its_name() {
        for kind in [
            ResourceSpecificationKind::Base,
            ResourceSpecificationKind::Logical,
            ResourceSpecificationKind::Physical,
            ResourceSpecificationKind::Function,
        ] {
            assert_eq!(
                ResourceSpecificationKind::from_type_name(kind.type_name()),
                kind
            );
        }
    }
}
