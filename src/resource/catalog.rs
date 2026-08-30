//! The resource catalog — TMF634.
//!
//! What TMF620 is to products, TMF634 is to resources: a [`ResourceCatalog`]
//! publishes [`ResourceCategory`] trees, each holding [`ResourceCandidate`]s
//! that make a [`ResourceSpecification`](super::ResourceSpecification)
//! available for use.
//!
//! Note the shape difference from TMF620. A `Category` there carries a typed
//! `parent` reference; a [`ResourceCategory`] here carries a bare `parentId`
//! string, and lists its children under `category` rather than `subCategory`.
//! The two catalogs are analogous, not identical, so the types are not shared.

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{ExternalIdentifier, Ref, RelatedParty, TimePeriod, Timestamp};

use super::ResourceSpecification;

tmf_struct! {
    @name = "ResourceCatalog";
    /// A collection of resource categories published together.
    pub struct ResourceCatalog {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this catalog.
        href: String,
        /// Name of the catalog.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the catalog.
        version: String,
        /// Kind of catalog, e.g. `ResourceCatalog`.
        catalog_type: String,
        /// When the catalog was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Period during which the catalog is valid.
        valid_for: TimePeriod,
        /// Top-level categories in this catalog.
        category: Vec<Ref<ResourceCategory>>,
        /// Parties related to the catalog, e.g. the publisher.
        related_party: Vec<RelatedParty>,
        /// Identifiers this catalog is known by in other systems.
        ///
        /// Note that TMF634 declares no `ResourceCatalogRef`: a catalog is
        /// addressed, never referenced, so this type carries no
        /// `@referredType` where its siblings do.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ResourceCatalog";
    /// Body of a `POST /resourceCatalog` — the v5 `ResourceCatalog_FVO`.
    pub struct ResourceCatalogCreate {
        @required {
            /// Name of the catalog. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the catalog.
        version: String,
        /// Kind of catalog.
        catalog_type: String,
        /// When the catalog was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Period during which the catalog is valid.
        valid_for: TimePeriod,
        /// Top-level categories in this catalog.
        category: Vec<Ref<ResourceCategory>>,
        /// Parties related to the catalog.
        related_party: Vec<RelatedParty>,
        /// Identifiers this catalog is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ResourceCatalog";
    /// Body of a `PATCH /resourceCatalog/{id}` — the v5 `_MVO` schema.
    pub struct ResourceCatalogUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New catalog type.
        catalog_type: String,
        /// New last-changed timestamp.
        last_update: Timestamp,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement category list.
        category: Vec<Ref<ResourceCategory>>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ResourceCategory", @ref = "ResourceCategoryRef";
    /// A node in the resource catalog hierarchy.
    pub struct ResourceCategory {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this category.
        href: String,
        /// Name of the category.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the category.
        version: String,
        /// When the category was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Whether this is a root category of its catalog.
        is_root: bool,
        /// Identifier of the parent category.
        ///
        /// A bare identifier, not a reference: TMF634 models the parent link
        /// differently from TMF620, which carries a typed `CategoryRef`.
        parent_id: String,
        /// Period during which the category is valid.
        valid_for: TimePeriod,
        /// Child categories.
        category: Vec<Ref<ResourceCategory>>,
        /// Candidates filed directly under this category.
        resource_candidate: Vec<Ref<ResourceCandidate>>,
        /// Specifications filed directly under this category.
        resource_specification: Vec<Ref<ResourceSpecification>>,
        /// Parties related to the category.
        related_party: Vec<RelatedParty>,
        /// Identifiers this category is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
        @renamed {
            /// The concrete class a `ResourceCategoryRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "ResourceCategory";
    /// Body of a `POST /resourceCategory` — the v5 `ResourceCategory_FVO`.
    pub struct ResourceCategoryCreate {
        @required {
            /// Name of the category. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the category.
        version: String,
        /// When the category was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Whether this is a root category.
        is_root: bool,
        /// Identifier of the parent category.
        parent_id: String,
        /// Period during which the category is valid.
        valid_for: TimePeriod,
        /// Child categories.
        category: Vec<Ref<ResourceCategory>>,
        /// Candidates filed directly under this category.
        resource_candidate: Vec<Ref<ResourceCandidate>>,
        /// Specifications filed directly under this category.
        resource_specification: Vec<Ref<ResourceSpecification>>,
        /// Parties related to the category.
        related_party: Vec<RelatedParty>,
        /// Identifiers this category is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ResourceCategory";
    /// Body of a `PATCH /resourceCategory/{id}` — the v5 `_MVO` schema.
    pub struct ResourceCategoryUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New last-changed timestamp.
        last_update: Timestamp,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New root flag.
        is_root: bool,
        /// New parent identifier.
        parent_id: String,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement child categories.
        category: Vec<Ref<ResourceCategory>>,
        /// Replacement candidate list.
        resource_candidate: Vec<Ref<ResourceCandidate>>,
        /// Replacement specification list.
        resource_specification: Vec<Ref<ResourceSpecification>>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ResourceCandidate", @ref = "ResourceCandidateRef";
    /// A resource specification made available through a catalog.
    ///
    /// A candidate and its specification may be published in any number of
    /// resource catalogs, or in none — which is the distinction between the
    /// specification (what a resource *is*) and the candidate (that it is
    /// *offered*).
    ///
    /// # `name` is on the wire but not in the schema
    ///
    /// TMF634 v5 defines no `name` member on `ResourceCandidate`, yet
    /// `ResourceCandidate_FVO` marks `name` **required** and every vendored
    /// response example carries one. The schema contradicts itself and the
    /// examples, so this crate follows the wire and types it. See
    /// `WIRE_ONLY` in `tests/coverage.rs`, which records the exception and
    /// fails if no fixture justifies it.
    pub struct ResourceCandidate {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this candidate.
        href: String,
        /// Name of the candidate.
        ///
        /// Not declared by the v5 schema; see the type documentation.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the candidate.
        version: String,
        /// When the candidate was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Period during which the candidate is valid.
        valid_for: TimePeriod,
        /// Categories this candidate is filed under.
        category: Vec<Ref<ResourceCategory>>,
        /// The specification this candidate makes available.
        resource_specification: Ref<ResourceSpecification>,
        /// Identifiers this candidate is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
        @renamed {
            /// The concrete class a `ResourceCandidateRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "ResourceCandidate";
    /// Body of a `POST /resourceCandidate` — the v5 `ResourceCandidate_FVO`.
    pub struct ResourceCandidateCreate {
        @required {
            /// Name of the candidate. **Required on create**, even though the
            /// read schema does not declare the member — see
            /// [`ResourceCandidate`].
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the candidate.
        version: String,
        /// When the candidate was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Period during which the candidate is valid.
        valid_for: TimePeriod,
        /// Categories this candidate is filed under.
        category: Vec<Ref<ResourceCategory>>,
        /// The specification this candidate makes available.
        resource_specification: Ref<ResourceSpecification>,
        /// Identifiers this candidate is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ResourceCandidate";
    /// Body of a `PATCH /resourceCandidate/{id}` — the v5 `_MVO` schema.
    pub struct ResourceCandidateUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New last-changed timestamp.
        last_update: Timestamp,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement category list.
        category: Vec<Ref<ResourceCategory>>,
        /// New specification reference.
        resource_specification: Ref<ResourceSpecification>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_entity!(ResourceCatalog, ResourceCategory, ResourceCandidate);
tmf_patch_body!(
    ResourceCatalogUpdate,
    ResourceCategoryUpdate,
    ResourceCandidateUpdate
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_category_parent_is_an_id_not_a_reference() {
        // The mirror-image of TMF620's `Category`, which carries a typed
        // `CategoryRef`. Modelling both the same way would put one of them in
        // `extensions`.
        let json = r#"{"@type":"ResourceCategory","parentId":"12","isRoot":false}"#;
        let category: ResourceCategory = serde_json::from_str(json).unwrap();

        assert_eq!(category.parent_id.as_deref(), Some("12"));
        assert!(category.extensions.is_empty());
    }

    #[test]
    fn a_candidate_keeps_the_name_the_schema_forgot() {
        // TMF634 declares no `name` on `ResourceCandidate`, requires it on the
        // create body, and sends it in every example. Typing it is what keeps
        // it out of `extensions`.
        let json = r#"{"@type":"ResourceCandidate","name":"Virtual Storage Medium"}"#;
        let candidate: ResourceCandidate = serde_json::from_str(json).unwrap();

        assert_eq!(candidate.name.as_deref(), Some("Virtual Storage Medium"));
        assert!(candidate.extensions.is_empty(), "name must be typed");
        assert_eq!(
            serde_json::to_value(&candidate).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap(),
        );
    }
}
