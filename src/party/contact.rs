//! Contact media — how you reach a party.

use crate::core::macros::tmf_struct;
use crate::core::{GeographicAddress, Ref, TimePeriod};

tmf_struct! {
    @name = "ContactMedium";
    /// A way of contacting a party: an email address, a phone number, a postal
    /// address, a social handle.
    ///
    /// The v5 OAS models this as an abstract base with five `@type`-discriminated
    /// subclasses whose payload members differ — and, unlike v4, those members
    /// sit at the top level rather than under a `characteristic` object. Rather
    /// than five near-identical Rust structs, or an enum that fails to parse a
    /// vendor subclass, this keeps every member optional on one type and exposes
    /// [`kind`] to recover which subclass the server sent.
    ///
    /// ```
    /// use rutmf::party::{ContactMedium, ContactMediumKind};
    ///
    /// let email = ContactMedium::email("ada@example.com");
    /// assert_eq!(email.kind(), ContactMediumKind::Email);
    /// assert_eq!(email.at_type, "EmailContactMedium");
    /// ```
    ///
    /// [`kind`]: ContactMedium::kind
    pub struct ContactMedium {
        /// Identifier of the contact medium.
        id: String,
        /// The role of this medium, e.g. `home`, `work`.
        contact_type: String,
        /// Whether this is the party's preferred medium.
        preferred: bool,
        /// Period during which the medium is valid.
        valid_for: TimePeriod,

        /// Email address — `EmailContactMedium`.
        email_address: String,
        /// Phone number — `PhoneContactMedium`.
        phone_number: String,
        /// Fax number — `FaxContactMedium`.
        fax_number: String,
        /// Social network handle — `SocialContactMedium`.
        social_network_id: String,

        /// First line of the street address — `GeographicAddressContactMedium`.
        street1: String,
        /// Second line of the street address.
        street2: String,
        /// City.
        city: String,
        /// State or province.
        state_or_province: String,
        /// Postal code.
        post_code: String,
        /// Country.
        country: String,
        /// Reference to a structured address in TMF673.
        geographic_address: Ref<GeographicAddress>,
    }
}

impl ContactMedium {
    /// An email contact medium.
    pub fn email(address: impl Into<String>) -> Self {
        Self::builder()
            .email_address(address)
            .at_type(ContactMediumKind::Email.type_name())
            .build()
    }

    /// A phone contact medium.
    pub fn phone(number: impl Into<String>) -> Self {
        Self::builder()
            .phone_number(number)
            .at_type(ContactMediumKind::Phone.type_name())
            .build()
    }

    /// Recovers the subclass implied by `@type`.
    #[must_use]
    pub fn kind(&self) -> ContactMediumKind {
        ContactMediumKind::from_type_name(self.type_name())
    }
}

/// The subclass of a [`ContactMedium`], recovered from its `@type`.
///
/// Mirrors the entries of the v5 discriminator mapping, plus
/// [`ContactMediumKind::Other`] so a vendor subclass never fails to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContactMediumKind {
    /// The abstract base, carrying no medium-specific member.
    Base,
    /// An email address.
    Email,
    /// A phone number.
    Phone,
    /// A fax number.
    Fax,
    /// A social network handle.
    Social,
    /// A postal address.
    GeographicAddress,
    /// A subclass this crate does not know.
    Other,
}

impl ContactMediumKind {
    /// Every subclass the v5 documents declare, base first.
    ///
    /// Excludes [`Other`](Self::Other), which stands for a class the documents
    /// do *not* declare. Checked against the specification's own
    /// `discriminator.mapping` by `every_subclass_enumeration_is_the_declared_mapping`
    /// in `tests/coverage.rs`.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Base,
            Self::Email,
            Self::Phone,
            Self::Fax,
            Self::Social,
            Self::GeographicAddress,
        ]
    }

    /// Maps a `@type` value to its kind; unknown names become [`Self::Other`].
    #[must_use]
    pub fn from_type_name(name: &str) -> Self {
        match name {
            "ContactMedium" => Self::Base,
            "EmailContactMedium" => Self::Email,
            "PhoneContactMedium" => Self::Phone,
            "FaxContactMedium" => Self::Fax,
            "SocialContactMedium" => Self::Social,
            "GeographicAddressContactMedium" => Self::GeographicAddress,
            _ => Self::Other,
        }
    }

    /// The canonical `@type` for this kind.
    ///
    /// [`Self::Other`] has no canonical name and maps to the abstract base.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Base | Self::Other => "ContactMedium",
            Self::Email => "EmailContactMedium",
            Self::Phone => "PhoneContactMedium",
            Self::Fax => "FaxContactMedium",
            Self::Social => "SocialContactMedium",
            Self::GeographicAddress => "GeographicAddressContactMedium",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_subclass_round_trips_as_other() {
        let json = r#"{"contactType":"work","@type":"VendorChatContactMedium","handle":"x"}"#;
        let medium: ContactMedium = serde_json::from_str(json).unwrap();
        assert_eq!(medium.kind(), ContactMediumKind::Other);
        assert_eq!(medium.extensions.get("handle").unwrap(), "x");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(json).unwrap(),
            serde_json::to_value(&medium).unwrap()
        );
    }

    #[test]
    fn constructors_set_the_discriminator() {
        assert_eq!(
            ContactMedium::phone("+49 30 1234").kind(),
            ContactMediumKind::Phone
        );
        assert_eq!(ContactMedium::email("a@b.c").at_type, "EmailContactMedium");
    }

    #[test]
    fn v5_puts_subclass_members_at_the_top_level() {
        // v4 nested these under `characteristic`; v5 does not.
        let json = r#"{"@type":"GeographicAddressContactMedium","city":"Berlin","country":"DE"}"#;
        let medium: ContactMedium = serde_json::from_str(json).unwrap();
        assert_eq!(medium.city.as_deref(), Some("Berlin"));
        assert!(medium.extensions.is_empty());
    }
}
