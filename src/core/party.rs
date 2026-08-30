//! Party references shared across every TM Forum domain.
//!
//! In v5 a related party points at *either* a party or a party role, modelled
//! in the OAS as a `oneOf` discriminated by `@type`.

use serde::{Deserialize, Serialize};

use super::extensible::TmfType;
use super::macros::tmf_struct;
use super::reference::Ref;

/// Marker for a party (an individual or organization) — TMF632.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Party;

impl TmfType for Party {
    const TYPE_NAME: &'static str = "Party";
    const REF_TYPE_NAME: &'static str = "PartyRef";
}

/// Marker for a party role (customer, supplier, …) — TMF669.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyRole;

impl TmfType for PartyRole {
    const TYPE_NAME: &'static str = "PartyRole";
    const REF_TYPE_NAME: &'static str = "PartyRoleRef";
}

/// Either a party reference or a party role reference.
///
/// The v5 OAS models this as a `oneOf` discriminated by `@type`. Both arms have
/// an identical wire shape, so this cannot be a serde `untagged` enum — that
/// would always select the first arm. Deserialisation reads `@type` and
/// dispatches on it, defaulting to [`PartyOrPartyRole::Party`] and preserving
/// any unrecognised value verbatim.
#[derive(Debug, Clone, PartialEq)]
pub enum PartyOrPartyRole {
    /// A reference to a party.
    Party(Ref<Party>),
    /// A reference to a party role.
    Role(Ref<PartyRole>),
}

impl Serialize for PartyOrPartyRole {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Party(r) => r.serialize(serializer),
            Self::Role(r) => r.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PartyOrPartyRole {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Both arms share one wire shape, so parse once and dispatch on `@type`.
        let parsed = Ref::<Party>::deserialize(deserializer)?;
        if parsed.type_name() == PartyRole::REF_TYPE_NAME {
            Ok(Self::Role(parsed.retarget()))
        } else {
            Ok(Self::Party(parsed))
        }
    }
}

impl From<Ref<Party>> for PartyOrPartyRole {
    fn from(reference: Ref<Party>) -> Self {
        Self::Party(reference)
    }
}

impl From<Ref<PartyRole>> for PartyOrPartyRole {
    fn from(reference: Ref<PartyRole>) -> Self {
        Self::Role(reference)
    }
}

impl PartyOrPartyRole {
    /// The identifier of the referred party or role.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Party(r) => &r.id,
            Self::Role(r) => &r.id,
        }
    }

    /// The class the underlying reference declares, or the arm's default.
    #[must_use]
    pub fn type_name(&self) -> &str {
        match self {
            Self::Party(r) => r.type_name(),
            Self::Role(r) => r.type_name(),
        }
    }

    /// The `@referredType`: the concrete class of the target, when the server
    /// disambiguated it (`Individual` rather than merely `Party`).
    #[must_use]
    pub fn referred_type(&self) -> Option<&str> {
        match self {
            Self::Party(r) => r.referred_type.as_deref(),
            Self::Role(r) => r.referred_type.as_deref(),
        }
    }
}

tmf_struct! {
    @name = "RelatedPartyRefOrPartyRoleRef";
    /// A party (or party role) linked to an entity in a named role.
    ///
    /// Almost every TM Forum resource carries one: an order has a customer, an
    /// alarm an assignee, a bill a payer.
    ///
    /// ```
    /// use rutmf::core::{Party, Ref, RelatedParty};
    ///
    /// let customer = RelatedParty::new("customer", Ref::<Party>::new("4104").with_name("Ada"));
    /// assert_eq!(customer.party_or_party_role.as_ref().unwrap().id(), "4104");
    /// ```
    ///
    /// A [`Ref<Party>`](Ref) and a [`Ref<PartyRole>`](Ref) both convert, so which
    /// arm of the v5 `oneOf` this is follows from the reference's type.
    pub struct RelatedParty {
        /// Role played by the related party, e.g. `customer`, `salesAgent`.
        role: String,
        /// The party or party role being referred to.
        party_or_party_role: PartyOrPartyRole,
    }
}

impl RelatedParty {
    /// A party or party role in a named role.
    ///
    /// ```
    /// use rutmf::core::{Party, PartyRole, Ref, RelatedParty};
    ///
    /// let buyer = RelatedParty::new("customer", Ref::<Party>::new("4104"));   // TMF632
    /// let agent = RelatedParty::new("salesAgent", Ref::<PartyRole>::new("77")); // TMF669
    ///
    /// assert_eq!(buyer.party_or_party_role.as_ref().unwrap().type_name(), "PartyRef");
    /// assert_eq!(agent.party_or_party_role.as_ref().unwrap().type_name(), "PartyRoleRef");
    /// ```
    pub fn new(role: impl Into<String>, target: impl Into<PartyOrPartyRole>) -> Self {
        Self::builder()
            .role(role)
            .party_or_party_role(target.into())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn related_party_round_trips() {
        let json = r#"{"role":"customer","partyOrPartyRole":{"id":"42","@referredType":"Individual","@type":"PartyRef"},"@type":"RelatedPartyRefOrPartyRoleRef"}"#;
        let rp: RelatedParty = serde_json::from_str(json).unwrap();
        assert_eq!(rp.role.as_deref(), Some("customer"));

        let target = rp.party_or_party_role.as_ref().unwrap();
        assert_eq!(target.id(), "42");
        assert_eq!(target.referred_type(), Some("Individual"));
        assert!(matches!(target, PartyOrPartyRole::Party(_)));

        assert_eq!(serde_json::to_string(&rp).unwrap(), json);
    }

    #[test]
    fn a_related_party_is_short_to_write_and_keeps_its_discriminator() {
        let buyer = RelatedParty::new("customer", Ref::<Party>::new("4104").with_name("Ada"));
        assert_eq!(
            serde_json::to_value(&buyer).unwrap(),
            serde_json::json!({
                "role": "customer",
                "partyOrPartyRole": {
                    "id": "4104",
                    "name": "Ada",
                    "@referredType": "Party",
                    "@type": "PartyRef",
                },
                "@type": "RelatedPartyRefOrPartyRoleRef",
            })
        );

        // The arm follows from the reference's type, so a role reference cannot
        // be written out under the party discriminator by mistake.
        let agent = RelatedParty::new("salesAgent", Ref::<PartyRole>::new("77"));
        assert_eq!(
            serde_json::to_value(&agent).unwrap()["partyOrPartyRole"]["@type"],
            "PartyRoleRef"
        );
    }

    #[test]
    fn a_role_reference_dispatches_on_its_discriminator() {
        let json = r#"{"id":"7","@type":"PartyRoleRef"}"#;
        let target: PartyOrPartyRole = serde_json::from_str(json).unwrap();
        assert!(matches!(target, PartyOrPartyRole::Role(_)));
        assert_eq!(serde_json::to_string(&target).unwrap(), json);
    }
}
