//! The party domain: individuals and organizations.
//!
//! Mirrors **TMF632 Party Management v5.0**. A *party* is anyone the service
//! provider deals with, before any role is attached: [`Individual`] for a
//! natural person, [`Organization`] for a body. The role they play — customer,
//! supplier, partner — is a separate concept, carried by
//! [`RelatedParty`](crate::core::RelatedParty) and by the customer domain.
//!
//! As in every domain here, each top-level resource appears three times,
//! matching the v5 `_FVO` / `_MVO` schema triple.

mod common;
mod contact;
mod individual;
mod organization;
mod role;

pub use common::{
    Disability, LanguageAbility, OtherNameIndividual, OtherNameOrganization, PartyCreditProfile,
    PartyIdentification, Skill,
};
pub use contact::{ContactMedium, ContactMediumKind};
pub use individual::{Individual, IndividualCreate, IndividualState, IndividualUpdate};
pub use organization::{
    Organization, OrganizationChildRelationship, OrganizationCreate,
    OrganizationParentRelationship, OrganizationState, OrganizationUpdate,
};
pub use role::{
    EntitySpecificationRelationship, PartyRole, PartyRoleCreate, PartyRoleKind,
    PartyRoleSpecification, PartyRoleSpecificationCreate, PartyRoleSpecificationUpdate,
    PartyRoleUpdate, TargetEntitySchema,
};
