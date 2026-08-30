//! `Organization` — a legal or administrative body, from TMF632 v5.

use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    Characteristic, ExternalIdentifier, Ref, RelatedParty, TaxExemptionCertificate, TimePeriod,
};

use super::common::{OtherNameOrganization, PartyCreditProfile, PartyIdentification};
use super::contact::ContactMedium;

/// The lifecycle state of an [`Organization`].
///
/// [`OrganizationState::Other`] keeps an unrecognised value intact rather than
/// failing the parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum OrganizationState {
    /// Created but not yet verified.
    #[serde(rename = "initialized")]
    Initialized,
    /// Existence has been verified.
    #[serde(rename = "validated")]
    Validated,
    /// The organization has been dissolved.
    #[serde(rename = "closed")]
    Closed,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

tmf_struct! {
    // TMF632 declares `OrganizationRef` for the two relationship members that
    // point at an organization. `PartyRef` is the class used when an
    // organization appears in the *party* union, which `PartyOrPartyRole`
    // models separately — `Individual` keeps it because TMF632 declares no
    // `IndividualRef` at all.
    @name = "Organization", @ref = "OrganizationRef";
    /// A company, department, or other body the provider deals with.
    ///
    /// This is the **read model**. Use [`OrganizationCreate`] for `POST` and
    /// [`OrganizationUpdate`] for `PATCH`.
    pub struct Organization {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this organization.
        href: String,
        /// Registered name.
        name: String,
        /// Kind of name held in `name`.
        name_type: String,
        /// Name the organization trades under.
        trading_name: String,
        /// Kind of organization, e.g. `company`, `department`.
        organization_type: String,
        /// Whether this is the head office.
        is_head_office: bool,
        /// Whether this is a legal entity in its own right.
        is_legal_entity: bool,
        /// Period during which the organization existed.
        exists_during: TimePeriod,
        /// Lifecycle state.
        status: OrganizationState,
        /// Ways of contacting the organization.
        contact_medium: Vec<ContactMedium>,
        /// Alternative names.
        other_name: Vec<OtherNameOrganization>,
        /// Registration documents.
        organization_identification: Vec<PartyIdentification>,
        /// The organization this one belongs to.
        organization_parent_relationship: OrganizationParentRelationship,
        /// Organizations belonging to this one.
        organization_child_relationship: Vec<OrganizationChildRelationship>,
        /// Credit ratings held about the organization.
        credit_rating: Vec<PartyCreditProfile>,
        /// Tax exemption certificates.
        tax_exemption_certificate: Vec<TaxExemptionCertificate>,
        /// Free-form characteristics.
        party_characteristic: Vec<Characteristic>,
        /// Other parties related to this one.
        related_party: Vec<RelatedParty>,
        /// Identifiers in external systems.
        external_reference: Vec<ExternalIdentifier>,
    }
}

tmf_entity!(Organization);

tmf_struct! {
    @name = "Organization";
    /// Body of a `POST /organization` — the v5 `Organization_FVO`.
    ///
    /// `name` is required on create.
    pub struct OrganizationCreate {
        @required {
            /// Registered name. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Kind of name held in `name`.
        name_type: String,
        /// Name the organization trades under.
        trading_name: String,
        /// Kind of organization.
        organization_type: String,
        /// Whether this is the head office.
        is_head_office: bool,
        /// Whether this is a legal entity in its own right.
        is_legal_entity: bool,
        /// Period during which the organization exists.
        exists_during: TimePeriod,
        /// Lifecycle state.
        status: OrganizationState,
        /// Ways of contacting the organization.
        contact_medium: Vec<ContactMedium>,
        /// Alternative names.
        other_name: Vec<OtherNameOrganization>,
        /// Registration documents.
        organization_identification: Vec<PartyIdentification>,
        /// The organization this one belongs to.
        organization_parent_relationship: OrganizationParentRelationship,
        /// Organizations belonging to this one.
        organization_child_relationship: Vec<OrganizationChildRelationship>,
        /// Credit ratings held about the organization.
        credit_rating: Vec<PartyCreditProfile>,
        /// Tax exemption certificates.
        tax_exemption_certificate: Vec<TaxExemptionCertificate>,
        /// Free-form characteristics.
        party_characteristic: Vec<Characteristic>,
        /// Other parties related to this one.
        related_party: Vec<RelatedParty>,
        /// Identifiers in external systems.
        external_reference: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "Organization";
    /// Body of a `PATCH /organization/{id}` — the v5 `Organization_MVO`.
    pub struct OrganizationUpdate {
        /// New registered name.
        name: String,
        /// New name type.
        name_type: String,
        /// New trading name.
        trading_name: String,
        /// New organization type.
        organization_type: String,
        /// New head-office flag.
        is_head_office: bool,
        /// New legal-entity flag.
        is_legal_entity: bool,
        /// New existence period.
        exists_during: TimePeriod,
        /// New lifecycle state.
        status: OrganizationState,
        /// Replacement contact media.
        contact_medium: Vec<ContactMedium>,
        /// Replacement alternative names.
        other_name: Vec<OtherNameOrganization>,
        /// Replacement registration documents.
        organization_identification: Vec<PartyIdentification>,
        /// Replacement parent relationship.
        organization_parent_relationship: OrganizationParentRelationship,
        /// Replacement child relationships.
        organization_child_relationship: Vec<OrganizationChildRelationship>,
        /// Replacement credit ratings.
        credit_rating: Vec<PartyCreditProfile>,
        /// Replacement tax exemption certificates.
        tax_exemption_certificate: Vec<TaxExemptionCertificate>,
        /// Replacement characteristics.
        party_characteristic: Vec<Characteristic>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement external references.
        external_reference: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "OrganizationParentRelationship";
    /// The organization this one belongs to.
    ///
    /// Identical in shape to [`OrganizationChildRelationship`]; they are
    /// separate v5 schemas because `@type` is what tells a server which
    /// direction a link points.
    pub struct OrganizationParentRelationship {
        /// The organization at the other end of the link.
        organization: Ref<Organization>,
        /// Kind of relationship, e.g. `subsidiary`.
        relationship_type: String,
    }
}

tmf_struct! {
    @name = "OrganizationChildRelationship";
    /// An organization belonging to this one.
    ///
    /// See [`OrganizationParentRelationship`] for why the two are distinct.
    pub struct OrganizationChildRelationship {
        /// The organization at the other end of the link.
        organization: Ref<Organization>,
        /// Kind of relationship, e.g. `subsidiary`.
        relationship_type: String,
    }
}

tmf_patch_body!(OrganizationUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_and_child_links_keep_their_own_discriminators() {
        assert_eq!(
            OrganizationParentRelationship::default().at_type,
            "OrganizationParentRelationship"
        );
        assert_eq!(
            OrganizationChildRelationship::default().at_type,
            "OrganizationChildRelationship"
        );
    }
}
