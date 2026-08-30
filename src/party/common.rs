//! Value objects shared by [`Individual`] and [`Organization`].
//!
//! [`Individual`]: super::Individual
//! [`Organization`]: super::Organization

use crate::core::macros::{tmf_struct, tmf_value};
use crate::core::{Attachment, TimePeriod, Timestamp};

tmf_struct! {
    @name = "PartyCreditProfile";
    /// A credit rating held about a party by a credit agency.
    pub struct PartyCreditProfile {
        /// Identifier of the profile.
        id: String,
        /// URI of the profile.
        href: String,
        /// Name of the credit agency.
        credit_agency_name: String,
        /// Kind of credit agency.
        credit_agency_type: String,
        /// Reference for the rating.
        rating_reference: String,
        /// The score itself.
        rating_score: i64,
        /// Period during which the rating is valid.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "IndividualIdentification";
    /// An identification document held by a party.
    ///
    /// Serves both `IndividualIdentification` and `OrganizationIdentification`,
    /// which are structurally identical in v5; `at_type` distinguishes them, and
    /// [`PartyIdentification::for_organization`] sets it.
    pub struct PartyIdentification {
        /// The identifier itself, e.g. a passport number.
        identification_id: String,
        /// Kind of identification, e.g. `passport`, `nationalId`.
        identification_type: String,
        /// Authority that issued the document.
        issuing_authority: String,
        /// When the document was issued.
        issuing_date: Timestamp,
        /// Period during which the document is valid.
        valid_for: TimePeriod,
        /// A scan or photo of the document.
        attachment: Attachment,
    }
}

impl PartyIdentification {
    /// The `@type` an organization's identification carries.
    pub const ORGANIZATION_TYPE: &'static str = "OrganizationIdentification";

    /// Re-labels this identification as an organization's.
    ///
    /// The two schemas are identical apart from `@type`, and a server matches
    /// on the discriminator.
    #[must_use]
    pub fn for_organization(mut self) -> Self {
        Self::ORGANIZATION_TYPE.clone_into(&mut self.at_type);
        self
    }
}

tmf_value! {
    /// An alternative name an individual is known by.
    pub struct OtherNameIndividual {
        /// Family name.
        family_name: String,
        /// Prefix to the family name, e.g. `van`.
        family_name_prefix: String,
        /// Given name.
        given_name: String,
        /// Middle name.
        middle_name: String,
        /// The full name as one string.
        full_name: String,
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
        /// Period during which the name applies.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "OtherNameOrganization";
    /// An alternative name an organization is known by.
    pub struct OtherNameOrganization {
        /// The name.
        name: String,
        /// Kind of name, e.g. `trading`.
        name_type: String,
        /// The trading name.
        trading_name: String,
        /// Period during which the name applies.
        valid_for: TimePeriod,
    }
}

tmf_value! {
    /// A disability recorded against an individual.
    pub struct Disability {
        /// Code identifying the disability.
        disability_code: String,
        /// Human-readable name.
        disability_name: String,
        /// Period during which the record applies.
        valid_for: TimePeriod,
    }
}

tmf_value! {
    /// A language an individual can use, with proficiency levels.
    pub struct LanguageAbility {
        /// ISO language code.
        language_code: String,
        /// Human-readable language name.
        language_name: String,
        /// Whether this is the individual's preferred language.
        is_favourite_language: bool,
        /// Listening proficiency.
        listening_proficiency: String,
        /// Reading proficiency.
        reading_proficiency: String,
        /// Speaking proficiency.
        speaking_proficiency: String,
        /// Writing proficiency.
        writing_proficiency: String,
        /// Period during which the ability applies.
        valid_for: TimePeriod,
    }
}

tmf_value! {
    /// A skill attributed to an individual.
    pub struct Skill {
        /// Code identifying the skill.
        skill_code: String,
        /// Human-readable skill name.
        skill_name: String,
        /// Assessed level.
        evaluated_level: String,
        /// Free-text comment on the assessment.
        comment: String,
        /// Period during which the assessment applies.
        valid_for: TimePeriod,
    }
}
