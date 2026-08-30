//! `Individual` — a natural person, from TMF632 v5.

use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    Characteristic, ExternalIdentifier, RelatedParty, TaxExemptionCertificate, Timestamp,
};

use super::common::{
    Disability, LanguageAbility, OtherNameIndividual, PartyCreditProfile, PartyIdentification,
    Skill,
};
use super::contact::ContactMedium;

/// The lifecycle state of an [`Individual`].
///
/// The v5 OAS defines a closed enum; [`IndividualState::Other`] keeps an
/// unrecognised value intact rather than failing the parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum IndividualState {
    /// Created but not yet verified.
    #[serde(rename = "initialized")]
    Initialized,
    /// Identity has been verified.
    #[serde(rename = "validated")]
    Validated,
    /// The person has died.
    #[serde(rename = "deceased")]
    Deceased,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

tmf_struct! {
    @name = "Individual", @ref = "PartyRef";
    /// A natural person known to the service provider.
    ///
    /// This is the **read model**. Use [`IndividualCreate`] for `POST` and
    /// [`IndividualUpdate`] for `PATCH`.
    ///
    /// ```
    /// use rutmf::party::{ContactMedium, IndividualCreate};
    ///
    /// // TMF632 requires both name parts on create.
    /// let body = IndividualCreate::builder()
    ///     .given_name("Ada")
    ///     .family_name("Lovelace")
    ///     .contact_medium(vec![ContactMedium::email("ada@example.com")])
    ///     .build();
    /// assert_eq!(body.family_name, "Lovelace");
    /// ```
    pub struct Individual {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this individual.
        href: String,
        /// Given (first) name.
        given_name: String,
        /// Family (last) name.
        family_name: String,
        /// Middle name.
        middle_name: String,
        /// Prefix to the family name, e.g. `van`.
        family_name_prefix: String,
        /// The full name as one string.
        name: String,
        /// The name as it should be displayed.
        formatted_name: String,
        /// The legally registered name.
        legal_name: String,
        /// Preferred given name.
        preferred_given_name: String,
        /// Title, e.g. `Dr`.
        title: String,
        /// Aristocratic title.
        aristocratic_title: String,
        /// Generational suffix, e.g. `Jr`.
        generation: String,
        /// Gender.
        gender: String,
        /// Marital status.
        marital_status: String,
        /// Nationality.
        nationality: String,
        /// Country of birth.
        country_of_birth: String,
        /// Place of birth.
        place_of_birth: String,
        /// Date of birth.
        birth_date: Timestamp,
        /// Date of death.
        death_date: Timestamp,
        /// Where the individual is located.
        location: String,
        /// Lifecycle state.
        status: IndividualState,
        /// Ways of contacting the individual.
        contact_medium: Vec<ContactMedium>,
        /// Alternative names.
        other_name: Vec<OtherNameIndividual>,
        /// Identification documents.
        individual_identification: Vec<PartyIdentification>,
        /// Languages the individual can use.
        language_ability: Vec<LanguageAbility>,
        /// Recorded skills.
        skill: Vec<Skill>,
        /// Recorded disabilities.
        disability: Vec<Disability>,
        /// Credit ratings held about the individual.
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

tmf_entity!(Individual);

tmf_struct! {
    @name = "Individual";
    /// Body of a `POST /individual` — the v5 `Individual_FVO`.
    ///
    /// `givenName` and `familyName` are required on create; `href` is
    /// server-owned and absent.
    pub struct IndividualCreate {
        @required {
            /// Given (first) name. **Required on create.**
            given_name: String,
            /// Family (last) name. **Required on create.**
            family_name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Middle name.
        middle_name: String,
        /// Prefix to the family name, e.g. `van`.
        family_name_prefix: String,
        /// The full name as one string.
        name: String,
        /// The name as it should be displayed.
        formatted_name: String,
        /// The legally registered name.
        legal_name: String,
        /// Preferred given name.
        preferred_given_name: String,
        /// Title, e.g. `Dr`.
        title: String,
        /// Aristocratic title.
        aristocratic_title: String,
        /// Generational suffix, e.g. `Jr`.
        generation: String,
        /// Gender.
        gender: String,
        /// Marital status.
        marital_status: String,
        /// Nationality.
        nationality: String,
        /// Country of birth.
        country_of_birth: String,
        /// Place of birth.
        place_of_birth: String,
        /// Date of birth.
        birth_date: Timestamp,
        /// Date of death.
        death_date: Timestamp,
        /// Where the individual is located.
        location: String,
        /// Lifecycle state.
        status: IndividualState,
        /// Ways of contacting the individual.
        contact_medium: Vec<ContactMedium>,
        /// Alternative names.
        other_name: Vec<OtherNameIndividual>,
        /// Identification documents.
        individual_identification: Vec<PartyIdentification>,
        /// Languages the individual can use.
        language_ability: Vec<LanguageAbility>,
        /// Recorded skills.
        skill: Vec<Skill>,
        /// Recorded disabilities.
        disability: Vec<Disability>,
        /// Credit ratings held about the individual.
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
    @name = "Individual";
    /// Body of a `PATCH /individual/{id}` — the v5 `Individual_MVO`.
    ///
    /// Every member is optional, and the server-owned `id` and `href` are
    /// absent entirely.
    pub struct IndividualUpdate {
        /// New given name.
        given_name: String,
        /// New family name.
        family_name: String,
        /// New middle name.
        middle_name: String,
        /// New family-name prefix.
        family_name_prefix: String,
        /// New full name.
        name: String,
        /// New display name.
        formatted_name: String,
        /// New legal name.
        legal_name: String,
        /// New preferred given name.
        preferred_given_name: String,
        /// New title.
        title: String,
        /// New aristocratic title.
        aristocratic_title: String,
        /// New generational suffix.
        generation: String,
        /// New gender.
        gender: String,
        /// New marital status.
        marital_status: String,
        /// New nationality.
        nationality: String,
        /// New country of birth.
        country_of_birth: String,
        /// New place of birth.
        place_of_birth: String,
        /// New date of birth.
        birth_date: Timestamp,
        /// New date of death.
        death_date: Timestamp,
        /// New location.
        location: String,
        /// New lifecycle state.
        status: IndividualState,
        /// Replacement contact media.
        contact_medium: Vec<ContactMedium>,
        /// Replacement alternative names.
        other_name: Vec<OtherNameIndividual>,
        /// Replacement identification documents.
        individual_identification: Vec<PartyIdentification>,
        /// Replacement language abilities.
        language_ability: Vec<LanguageAbility>,
        /// Replacement skills.
        skill: Vec<Skill>,
        /// Replacement disabilities.
        disability: Vec<Disability>,
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

tmf_patch_body!(IndividualUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_keeps_an_unknown_value() {
        let parsed: Individual = serde_json::from_str(r#"{"status":"suspended"}"#).unwrap();
        assert_eq!(
            parsed.status,
            Some(IndividualState::Other("suspended".into()))
        );
        assert_eq!(
            serde_json::to_value(&parsed).unwrap()["status"],
            serde_json::json!("suspended")
        );
    }

    #[test]
    fn known_states_use_the_spec_spelling() {
        let parsed: Individual = serde_json::from_str(r#"{"status":"validated"}"#).unwrap();
        assert_eq!(parsed.status, Some(IndividualState::Validated));
    }

    #[test]
    fn a_patch_body_can_change_the_members_the_mvo_allows() {
        let patch = IndividualUpdate::builder().place_of_birth("Genoa").build();
        assert_eq!(
            serde_json::to_string(&patch).unwrap(),
            r#"{"placeOfBirth":"Genoa","@type":"Individual"}"#
        );
    }

    #[test]
    fn a_defaulted_patch_body_carries_a_valid_discriminator() {
        // Deriving `Default` would leave `@type` empty, which is a payload no
        // conformant server accepts.
        assert_eq!(IndividualUpdate::default().at_type, "Individual");
    }
}
