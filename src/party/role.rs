//! `PartyRole` — the capacity a party is acting in.
//!
//! Mirrors **TMF669 Party Role Management v5.0.0**. A [`Party`] is *who*
//! someone is; a [`PartyRole`] is *what they are to you*. The same organisation
//! can be a supplier on one contract and a consumer on another, and each role
//! carries its own accounts, agreements and payment methods.
//!
//! TMF669 describes itself as "a generalization of TMF629 Customer Management
//! where Party Roles may be any — not only a Customer", and that is exactly the
//! relationship in this crate: a [`Customer`](crate::customer::Customer) is one
//! party role with its own API, and [`PartyRole`] is the general case.
//!
//! # Why this API matters more than its size suggests
//!
//! Nearly every resource in every TM Forum API carries a `relatedParty`, and
//! each entry may name either a party or a *party role*. Until TMF669 was
//! modelled the role arm of [`PartyOrPartyRole`] pointed at a marker: the
//! reference was typed and correct, and there was nothing on the other end of
//! it. Now a role reference resolves like any other.
//!
//! # Four subclasses that add nothing
//!
//! TMF669 declares `Supplier`, `Producer`, `Consumer` and `BusinessPartner` as
//! subclasses of `PartyRole`, and **none of them adds a single member** — they
//! differ only in `@type`. So this is one type carrying the shape, with
//! [`PartyRoleKind`] recovering which subclass a server sent. Generating four
//! identical Rust structs would be four ways to spell one thing.
//!
//! [`Party`]: crate::core::Party
//! [`PartyOrPartyRole`]: crate::core::PartyOrPartyRole

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct, tmf_value};
use crate::core::{
    Account, Agreement, AgreementSpecification, AssociationSpecification, Attachment,
    Characteristic, CharacteristicSpecification, Constraint, CreditProfile, Party, PaymentMethod,
    PermissionSpecificationSet, Ref, RelatedParty, TimePeriod, Timestamp,
};

use super::ContactMedium;

tmf_struct! {
    @name = "PartyRole", @ref = "PartyRoleRef";
    /// The capacity a party is acting in — TMF669.
    ///
    /// This is the **read model**. Use [`PartyRoleCreate`] for `POST` and
    /// [`PartyRoleUpdate`] for `PATCH`.
    ///
    /// The `@type` a server sends may be `PartyRole` or one of its four
    /// subclasses; [`kind`](PartyRole::kind) reads it back.
    pub struct PartyRole {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this role.
        href: String,
        /// Name of the role.
        name: String,
        /// Narrative description.
        description: String,
        /// The role played, e.g. `supplier`.
        role: String,
        /// Lifecycle status of the role.
        status: String,
        /// Why the role is in that status.
        status_reason: String,
        /// The party acting in this role.
        engaged_party: Ref<Party>,
        /// The specification this role realises.
        party_role_specification: Ref<PartyRoleSpecification>,
        /// Accounts held in this role — TMF666.
        account: Vec<Ref<Account>>,
        /// Agreements entered into in this role — TMF651.
        agreement: Vec<Ref<Agreement>>,
        /// Payment methods usable in this role — TMF670.
        payment_method: Vec<Ref<PaymentMethod>>,
        /// Ways of contacting the party in this role.
        contact_medium: Vec<ContactMedium>,
        /// Credit assessments of the party in this role.
        credit_profile: Vec<CreditProfile>,
        /// Configured characteristics of the role.
        characteristic: Vec<Characteristic>,
        /// Parties related to this role.
        related_party: Vec<RelatedParty>,
        /// Period during which the role holds.
        valid_for: TimePeriod,
    }
}

tmf_entity!(PartyRole);

tmf_struct! {
    @name = "PartyRole";
    /// Body of a `POST /partyRole` — the v5 `PartyRole_FVO`.
    ///
    /// `engagedParty` and `name` are **required**: a role with nobody playing
    /// it, or with no name to play it under, is not a role.
    pub struct PartyRoleCreate {
        @required {
            /// The party acting in this role. **Required on create.**
            engaged_party: Ref<Party>,
            /// Name of the role. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// The role played, e.g. `supplier`.
        role: String,
        /// Lifecycle status of the role.
        status: String,
        /// Why the role is in that status.
        status_reason: String,
        /// The specification this role realises.
        party_role_specification: Ref<PartyRoleSpecification>,
        /// Accounts held in this role — TMF666.
        account: Vec<Ref<Account>>,
        /// Agreements entered into in this role — TMF651.
        agreement: Vec<Ref<Agreement>>,
        /// Payment methods usable in this role — TMF670.
        payment_method: Vec<Ref<PaymentMethod>>,
        /// Ways of contacting the party in this role.
        contact_medium: Vec<ContactMedium>,
        /// Credit assessments of the party in this role.
        credit_profile: Vec<CreditProfile>,
        /// Configured characteristics of the role.
        characteristic: Vec<Characteristic>,
        /// Parties related to this role.
        related_party: Vec<RelatedParty>,
        /// Period during which the role holds.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "PartyRole";
    /// Body of a `PATCH /partyRole/{id}` — the v5 `PartyRole_MVO`.
    ///
    /// `id` and `href` are server-owned and absent entirely.
    pub struct PartyRoleUpdate {
        /// Name of the role.
        name: String,
        /// Narrative description.
        description: String,
        /// The role played.
        role: String,
        /// Lifecycle status of the role.
        status: String,
        /// Why the role is in that status.
        status_reason: String,
        /// The party acting in this role.
        engaged_party: Ref<Party>,
        /// The specification this role realises.
        party_role_specification: Ref<PartyRoleSpecification>,
        /// Accounts held in this role — TMF666.
        account: Vec<Ref<Account>>,
        /// Agreements entered into in this role — TMF651.
        agreement: Vec<Ref<Agreement>>,
        /// Payment methods usable in this role — TMF670.
        payment_method: Vec<Ref<PaymentMethod>>,
        /// Ways of contacting the party in this role.
        contact_medium: Vec<ContactMedium>,
        /// Credit assessments of the party in this role.
        credit_profile: Vec<CreditProfile>,
        /// Configured characteristics of the role.
        characteristic: Vec<Characteristic>,
        /// Parties related to this role.
        related_party: Vec<RelatedParty>,
        /// Period during which the role holds.
        valid_for: TimePeriod,
    }
}

tmf_patch_body!(PartyRoleUpdate);

/// Which subclass of `PartyRole` a server sent.
///
/// The four subclasses TMF669 declares are pure `@type` markers — not one of
/// them adds a member — so this recovers the distinction without splitting the
/// shape four ways.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PartyRoleKind {
    /// The base class, sent when a server draws no finer distinction.
    PartyRole,
    /// A party that supplies goods or services.
    Supplier,
    /// A party that produces them.
    Producer,
    /// A party that consumes them.
    Consumer,
    /// A commercial partner.
    BusinessPartner,
    /// A `@type` outside the v5 mapping, preserved verbatim.
    Other(String),
}

impl PartyRoleKind {
    /// Every subclass the v5 documents declare, base first.
    ///
    /// Excludes [`Other`](Self::Other), which stands for a class the documents
    /// do *not* declare. Checked against the specification's own
    /// `discriminator.mapping` by `every_subclass_enumeration_is_the_declared_mapping`
    /// in `tests/coverage.rs`.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::PartyRole,
            Self::Supplier,
            Self::Producer,
            Self::Consumer,
            Self::BusinessPartner,
        ]
    }

    /// Maps a `@type` value to its kind.
    ///
    /// An unrecognised value is kept in [`Other`](Self::Other) rather than
    /// flattened into the base class, because a vendor subclass is information
    /// a caller may want — and losing it would make a round trip lossy.
    #[must_use]
    pub fn from_type_name(name: &str) -> Self {
        match name {
            "Supplier" => Self::Supplier,
            "Producer" => Self::Producer,
            "Consumer" => Self::Consumer,
            "BusinessPartner" => Self::BusinessPartner,
            "PartyRole" => Self::PartyRole,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The `@type` this kind is written as.
    ///
    /// The write direction, so creating a supplier does not mean spelling
    /// `"Supplier"` at the call site:
    ///
    /// ```
    /// use rutmf::core::{Party, Ref};
    /// use rutmf::party::{PartyRoleCreate, PartyRoleKind};
    ///
    /// let supplier = PartyRoleCreate::builder()
    ///     .name("Acme Ltd")
    ///     .engaged_party(Ref::<Party>::new("4104"))
    ///     .at_type(PartyRoleKind::Supplier.type_name())
    ///     .build();
    /// assert_eq!(supplier.at_type, "Supplier");
    /// ```
    #[must_use]
    pub fn type_name(&self) -> &str {
        match self {
            Self::PartyRole => "PartyRole",
            Self::Supplier => "Supplier",
            Self::Producer => "Producer",
            Self::Consumer => "Consumer",
            Self::BusinessPartner => "BusinessPartner",
            Self::Other(name) => name,
        }
    }
}

impl PartyRole {
    /// Which subclass the server said this is.
    ///
    /// Reads the `@type` the payload carried; see
    /// [`PartyRoleKind::from_type_name`].
    #[must_use]
    pub fn kind(&self) -> PartyRoleKind {
        PartyRoleKind::from_type_name(self.type_name())
    }
}

tmf_struct! {
    @name = "PartyRoleSpecification", @ref = "PartyRoleSpecificationRef";
    /// The template a [`PartyRole`] is created from — TMF669.
    ///
    /// What a `ProductSpecification` is to a product, this is to a role: it
    /// names the characteristics a role of this kind carries and the
    /// permissions it grants.
    pub struct PartyRoleSpecification {
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
        /// Whether the specification bundles others.
        is_bundle: bool,
        /// When the specification was last changed.
        last_update: Timestamp,
        /// Where the specification is in its own lifecycle.
        lifecycle_status: String,
        /// Lifecycle status of the specification.
        status: String,
        /// Characteristics a role built from this carries.
        spec_characteristic: Vec<CharacteristicSpecification>,
        /// Links to other entity specifications.
        entity_spec_relationship: Vec<EntitySpecificationRelationship>,
        /// Agreement specifications governing roles of this kind — TMF651.
        agreement_specification: Vec<Ref<AgreementSpecification>>,
        /// Permission sets granted to roles of this kind — TMF672.
        permission_specification_set: Vec<Ref<PermissionSpecificationSet>>,
        /// Constraints on the specification.
        constraint: Vec<Ref<Constraint>>,
        /// Documents attached to the specification.
        attachment: Vec<Attachment>,
        /// Schema describing the entity this specification targets.
        target_entity_schema: TargetEntitySchema,
        /// Parties related to the specification.
        related_party: Vec<RelatedParty>,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
    }
}

tmf_entity!(PartyRoleSpecification);

tmf_struct! {
    @name = "PartyRoleSpecification";
    /// Body of a `POST /partyRoleSpecification` — the v5
    /// `PartyRoleSpecification_FVO`.
    pub struct PartyRoleSpecificationCreate {
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
        /// Whether the specification bundles others.
        is_bundle: bool,
        /// When the specification was last changed.
        last_update: Timestamp,
        /// Where the specification is in its own lifecycle.
        lifecycle_status: String,
        /// Lifecycle status of the specification.
        status: String,
        /// Characteristics a role built from this carries.
        spec_characteristic: Vec<CharacteristicSpecification>,
        /// Links to other entity specifications.
        entity_spec_relationship: Vec<EntitySpecificationRelationship>,
        /// Agreement specifications governing roles of this kind — TMF651.
        agreement_specification: Vec<Ref<AgreementSpecification>>,
        /// Permission sets granted to roles of this kind — TMF672.
        permission_specification_set: Vec<Ref<PermissionSpecificationSet>>,
        /// Constraints on the specification.
        constraint: Vec<Ref<Constraint>>,
        /// Documents attached to the specification.
        attachment: Vec<Attachment>,
        /// Schema describing the entity this specification targets.
        target_entity_schema: TargetEntitySchema,
        /// Parties related to the specification.
        related_party: Vec<RelatedParty>,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "PartyRoleSpecification";
    /// Body of a `PATCH /partyRoleSpecification/{id}` — the v5
    /// `PartyRoleSpecification_MVO`.
    pub struct PartyRoleSpecificationUpdate {
        /// Name of the specification.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the specification.
        version: String,
        /// Whether the specification bundles others.
        is_bundle: bool,
        /// When the specification was last changed.
        last_update: Timestamp,
        /// Where the specification is in its own lifecycle.
        lifecycle_status: String,
        /// Lifecycle status of the specification.
        status: String,
        /// Characteristics a role built from this carries.
        spec_characteristic: Vec<CharacteristicSpecification>,
        /// Links to other entity specifications.
        entity_spec_relationship: Vec<EntitySpecificationRelationship>,
        /// Agreement specifications governing roles of this kind — TMF651.
        agreement_specification: Vec<Ref<AgreementSpecification>>,
        /// Permission sets granted to roles of this kind — TMF672.
        permission_specification_set: Vec<Ref<PermissionSpecificationSet>>,
        /// Constraints on the specification.
        constraint: Vec<Ref<Constraint>>,
        /// Documents attached to the specification.
        attachment: Vec<Attachment>,
        /// Schema describing the entity this specification targets.
        target_entity_schema: TargetEntitySchema,
        /// Parties related to the specification.
        related_party: Vec<RelatedParty>,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
    }
}

tmf_patch_body!(PartyRoleSpecificationUpdate);

tmf_value! {
    /// A link between two entity specifications.
    ///
    /// A value object rather than an entity: TMF669 gives it no `@type` of its
    /// own, and no `id` either — the other end is identified by `href` and
    /// `name` alone.
    pub struct EntitySpecificationRelationship {
        /// URI of the referenced specification.
        href: String,
        /// Name of the referenced specification.
        name: String,
        /// What kind of link this is.
        relationship_type: String,
        /// The role the referenced specification plays.
        role: String,
        /// The specification describing the association itself.
        association_spec: Ref<AssociationSpecification>,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
        @renamed {
            /// Immediate base class, when a server sends one.
            ///
            /// TMF669 declares `@baseType` and `@schemaLocation` here but *not*
            /// `@type`, so the two are members rather than the usual
            /// entity plumbing.
            "@baseType" base_type: String,
            /// URI of a schema describing this relationship.
            "@schemaLocation" schema_location: String,
        }
    }
}

tmf_value! {
    /// A pointer to the schema describing the entity a specification targets.
    pub struct TargetEntitySchema {
        @renamed {
            /// URI of the schema document.
            "@schemaLocation" schema_location: String,
            /// Class the schema describes.
            "@type" ty: String,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subclass_survives_both_directions() {
        for kind in PartyRoleKind::all() {
            assert_eq!(
                &PartyRoleKind::from_type_name(kind.type_name()),
                kind,
                "{} does not read back as itself",
                kind.type_name()
            );
        }
    }

    #[test]
    fn a_vendor_subclass_keeps_its_name_rather_than_becoming_the_base() {
        // Flattening it into `PartyRole` would make the round trip lossy: the
        // class a server sent would be replaced by the base on the way out.
        let kind = PartyRoleKind::from_type_name("WholesalePartner");
        assert_eq!(kind, PartyRoleKind::Other("WholesalePartner".to_owned()));
        assert_eq!(kind.type_name(), "WholesalePartner");
    }

    #[test]
    fn a_role_reports_the_subclass_its_payload_declared() {
        let supplier: PartyRole = serde_json::from_str(r#"{"id":"7","@type":"Supplier"}"#).unwrap();
        assert_eq!(supplier.kind(), PartyRoleKind::Supplier);

        // A payload with no `@type` is still a party role, because the schema
        // this type models says so.
        let bare: PartyRole = serde_json::from_str(r#"{"id":"7"}"#).unwrap();
        assert_eq!(bare.kind(), PartyRoleKind::PartyRole);
    }
}
