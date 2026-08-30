//! Configurable features of a resource.

use crate::core::macros::tmf_struct;
use crate::core::{Characteristic, FeatureRelationshipType, Policy, Ref, TimePeriod};

tmf_struct! {
    @name = "Feature";
    /// A configurable capability of a resource, switched on or off.
    ///
    /// # Why this is not `service::Feature`
    ///
    /// TMF638 and TMF639 both declare a schema called `Feature`, and they are
    /// *not* the same: a service feature is constrained by a `ConstraintRef`
    /// (TMF632 constraints), a resource feature by a `PolicyRef` (TMF638-side
    /// policy). Modelling them as one type would let you set a member the
    /// server will silently drop, so this crate keeps them apart — see
    /// [`crate::service::Feature`].
    pub struct Feature {
        /// Identifier of the feature within its resource.
        id: String,
        /// Name the feature is known by.
        name: String,
        /// Whether the feature is currently switched on.
        is_enabled: bool,
        /// Whether the feature groups others.
        is_bundle: bool,
        /// Characteristics configuring the feature.
        feature_characteristic: Vec<Characteristic>,
        /// Relationships to other features.
        feature_relationship: Vec<FeatureRelationship>,
        /// Policies constraining the feature.
        policy_constraint: Vec<Ref<Policy>>,
    }
}

tmf_struct! {
    @name = "FeatureRelationship";
    /// A typed link between two features of the same resource.
    ///
    /// # Why this is not `service::FeatureRelationship`
    ///
    /// TMF639's `FeatureRelationship` extends `EntityRef`, so it is addressable
    /// — `href`, `@type`, `@referredType`. TMF638's is a bare object with none
    /// of that. Same name, different schema; see
    /// [`crate::service::FeatureRelationship`].
    pub struct FeatureRelationship {
        /// Identifier of the feature at the other end.
        id: String,
        /// URI of the feature at the other end.
        href: String,
        /// Name of the feature at the other end.
        name: String,
        /// Kind of relationship: `excluded`, `includes`, `may include`,
        /// `requires`.
        relationship_type: FeatureRelationshipType,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
        @renamed {
            /// The concrete class of the target feature.
            "@referredType" referred_type: String,
        }
    }
}
