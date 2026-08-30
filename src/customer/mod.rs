//! The customer domain.
//!
//! Mirrors **TMF629 Customer Management v5.0.1**. A [`Customer`] is a *party
//! playing the customer role* — it does not duplicate the party, it points at
//! one through `engaged_party`.

use crate::account::Account;
use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    Agreement, Characteristic, CreditProfile, Party, PaymentMethod, Ref, RelatedParty, TimePeriod,
};
use crate::party::{ContactMedium, PartyRoleSpecification};

tmf_struct! {
    @name = "Customer", @ref = "PartyRoleRef";
    /// A party engaged in a customer relationship with the service provider.
    ///
    /// This is the **read model**. Use [`CustomerCreate`] for `POST` and
    /// [`CustomerUpdate`] for `PATCH`.
    ///
    /// ```
    /// use rutmf::core::{Party, Ref};
    /// use rutmf::customer::CustomerCreate;
    ///
    /// // TMF629 requires both a name and the party being engaged.
    /// let body = CustomerCreate::builder()
    ///     .name("Ada Lovelace")
    ///     .engaged_party(Ref::<Party>::new("4104"))
    ///     .build();
    /// assert_eq!(body.engaged_party.id, "4104");
    /// ```
    pub struct Customer {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this customer.
        href: String,
        /// Name the customer is known by.
        name: String,
        /// Narrative description.
        description: String,
        /// The party playing the customer role — TMF632.
        engaged_party: Ref<Party>,
        /// Role name, where the API distinguishes several.
        role: String,
        /// Lifecycle status of the relationship.
        status: String,
        /// Why the customer is in its current status.
        status_reason: String,
        /// Period during which the relationship is valid.
        valid_for: TimePeriod,
        /// The specification this customer role conforms to — TMF669.
        party_role_specification: Ref<PartyRoleSpecification>,
        /// Billing accounts held by the customer — TMF666.
        account: Vec<Ref<Account>>,
        /// Agreements the customer is party to — TMF651.
        agreement: Vec<Ref<Agreement>>,
        /// Payment methods on file — TMF670.
        payment_method: Vec<Ref<PaymentMethod>>,
        /// Credit assessments of the customer.
        credit_profile: Vec<CreditProfile>,
        /// Ways of contacting the customer in this role.
        contact_medium: Vec<ContactMedium>,
        /// Free-form characteristics.
        characteristic: Vec<Characteristic>,
        /// Other parties related to this customer.
        related_party: Vec<RelatedParty>,
    }
}

tmf_entity!(Customer);

tmf_struct! {
    @name = "Customer";
    /// Body of a `POST /customer` — the v5 `Customer_FVO`.
    ///
    /// `name` and `engagedParty` are required on create. Unusually, TMF629
    /// keeps `href` on the create schema; it is here for fidelity, but a server
    /// assigns it.
    pub struct CustomerCreate {
        @required {
            /// Name the customer is known by. **Required on create.**
            name: String,
            /// The party playing the customer role. **Required on create.**
            engaged_party: Ref<Party>,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Client-supplied URI, where the server permits one.
        href: String,
        /// Narrative description.
        description: String,
        /// Role name, where the API distinguishes several.
        role: String,
        /// Initial lifecycle status.
        status: String,
        /// Why the customer starts in that status.
        status_reason: String,
        /// Period during which the relationship is valid.
        valid_for: TimePeriod,
        /// The specification this customer role conforms to.
        party_role_specification: Ref<PartyRoleSpecification>,
        /// Billing accounts held by the customer.
        account: Vec<Ref<Account>>,
        /// Agreements the customer is party to.
        agreement: Vec<Ref<Agreement>>,
        /// Payment methods on file.
        payment_method: Vec<Ref<PaymentMethod>>,
        /// Credit assessments of the customer.
        credit_profile: Vec<CreditProfile>,
        /// Ways of contacting the customer in this role.
        contact_medium: Vec<ContactMedium>,
        /// Free-form characteristics.
        characteristic: Vec<Characteristic>,
        /// Other parties related to this customer.
        related_party: Vec<RelatedParty>,
    }
}

tmf_struct! {
    @name = "Customer";
    /// Body of a `PATCH /customer/{id}` — the v5 `Customer_MVO`.
    ///
    /// # A spec quirk worth knowing
    ///
    /// Unlike every other resource in this crate, TMF629 v5.0.1 marks `name`
    /// and `engagedParty` **required on the `_MVO` schema too**, and keeps `id`
    /// and `href` on it. That is unusual for a patch body, but it is what the
    /// specification says, so it is what this type enforces: a `PATCH` must
    /// restate the customer's name and engaged party.
    pub struct CustomerUpdate {
        @required {
            /// Name the customer is known by. **Required by the v5 patch schema.**
            name: String,
            /// The party playing the customer role. **Required by the v5 patch schema.**
            engaged_party: Ref<Party>,
        }
        /// Identifier, which the v5 patch schema unusually retains.
        id: String,
        /// URI, which the v5 patch schema unusually retains.
        href: String,
        /// New description.
        description: String,
        /// New role name.
        role: String,
        /// New lifecycle status.
        status: String,
        /// New status reason.
        status_reason: String,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement party role specification.
        party_role_specification: Ref<PartyRoleSpecification>,
        /// Replacement account list.
        account: Vec<Ref<Account>>,
        /// Replacement agreement list.
        agreement: Vec<Ref<Agreement>>,
        /// Replacement payment methods.
        payment_method: Vec<Ref<PaymentMethod>>,
        /// Replacement credit profiles.
        credit_profile: Vec<CreditProfile>,
        /// Replacement contact media.
        contact_medium: Vec<ContactMedium>,
        /// Replacement characteristics.
        characteristic: Vec<Characteristic>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
    }
}

tmf_patch_body!(CustomerUpdate);
