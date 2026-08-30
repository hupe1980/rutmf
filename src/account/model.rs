//! The `Account` family and its supporting types — TMF666.

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    FinancialAccount, Money, PaymentMethod, Ref, RelatedParty, TaxExemptionCertificate, TimePeriod,
    Timestamp,
};
use crate::party::ContactMedium;

tmf_struct! {
    @name = "Account", @ref = "AccountRef";
    /// An account a bill is issued against.
    ///
    /// # One type for the whole family
    ///
    /// TMF666 declares an abstract `Account` and four subclasses, each with its
    /// own collection. `BillingAccount`, `PartyAccount` and `SettlementAccount`
    /// add the members a bill needs — a structure, a payment plan, a default
    /// method; `FinancialAccount` adds none. Rather than four near-identical
    /// structs, this carries the union and exposes [`kind`] to recover which
    /// subclass a server sent, so an unrecognised vendor subclass never fails a
    /// parse.
    ///
    /// ```
    /// use rutmf::account::{Account, AccountKind};
    ///
    /// let json = r#"{"@type":"BillingAccount","name":"Acme Ltd","ratingType":"postpaid"}"#;
    /// let account: Account = serde_json::from_str(json).unwrap();
    ///
    /// assert_eq!(account.kind(), AccountKind::Billing);
    /// assert_eq!(account.rating_type.as_deref(), Some("postpaid"));
    /// ```
    ///
    /// [`kind`]: Account::kind
    pub struct Account {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this account.
        href: String,
        /// Name of the account.
        name: String,
        /// Narrative description.
        description: String,
        /// How the provider classifies the account.
        account_type: String,
        /// Where the account sits in its lifecycle.
        state: String,
        /// When the account was last changed.
        last_update: Timestamp,
        /// The most the account may owe.
        credit_limit: Money,
        /// Balances held against the account.
        account_balance: Vec<AccountBalance>,
        /// Links to other accounts.
        account_relationship: Vec<AccountRelationship>,
        /// People to contact about this account.
        contact: Vec<Contact>,
        /// Tax exemptions the account holder claims.
        tax_exemption: Vec<TaxExemptionCertificate>,
        /// Parties related to the account — who holds it, who pays.
        related_party: Vec<RelatedParty>,

        /// How bills are formatted and delivered — the billable subclasses.
        bill_structure: BillStructure,
        /// How the holder pays by default — the billable subclasses.
        default_payment_method: Ref<PaymentMethod>,
        /// The financial account money is posted to — the billable subclasses.
        financial_account: Ref<FinancialAccount>,
        /// Instalment arrangements — the billable subclasses.
        payment_plan: Vec<PaymentPlan>,
        /// Whether the account is paid up — the billable subclasses.
        payment_status: String,
        /// Prepaid or postpaid — `BillingAccount` alone.
        rating_type: String,
        @renamed {
            /// The concrete class an `AccountRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

impl Account {
    /// Recovers the subclass implied by `@type`.
    #[must_use]
    pub fn kind(&self) -> AccountKind {
        AccountKind::from_type_name(self.type_name())
    }
}

/// The subclass of an [`Account`], recovered from its `@type`.
///
/// Mirrors the entries of the v5 discriminator mapping, plus
/// [`AccountKind::Other`] so a vendor subclass never fails to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccountKind {
    /// The abstract base, carrying no subclass-specific member.
    Base,
    /// An account bills are issued against — adds `ratingType`.
    Billing,
    /// An account money is posted to; adds nothing of its own.
    Financial,
    /// An account held by a party.
    Party,
    /// An account used to settle between providers.
    Settlement,
    /// A subclass this crate does not know.
    Other,
}

impl AccountKind {
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
            Self::Billing,
            Self::Financial,
            Self::Party,
            Self::Settlement,
        ]
    }

    /// Maps a `@type` value to its kind; unknown names become [`Self::Other`].
    #[must_use]
    pub fn from_type_name(name: &str) -> Self {
        match name {
            "Account" => Self::Base,
            "BillingAccount" => Self::Billing,
            "FinancialAccount" => Self::Financial,
            "PartyAccount" => Self::Party,
            "SettlementAccount" => Self::Settlement,
            _ => Self::Other,
        }
    }

    /// The canonical `@type` for this kind.
    ///
    /// [`Self::Other`] has no canonical name and maps to the abstract base.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Base | Self::Other => "Account",
            Self::Billing => "BillingAccount",
            Self::Financial => "FinancialAccount",
            Self::Party => "PartyAccount",
            Self::Settlement => "SettlementAccount",
        }
    }

    /// The collection this kind is served from.
    ///
    /// The four subclasses each have their own path, which is why one Rust type
    /// still needs four sets of client methods.
    #[must_use]
    pub fn collection(self) -> &'static str {
        match self {
            Self::Base | Self::Other | Self::Party => "partyAccount",
            Self::Billing => "billingAccount",
            Self::Financial => "financialAccount",
            Self::Settlement => "settlementAccount",
        }
    }
}

tmf_struct! {
    @name = "Account";
    /// Body of a `POST` to any of the four account collections.
    ///
    /// One type for all four `_FVO` schemas, which agree on what they require:
    /// a `name` and at least one `relatedParty`. Set `@type` through the
    /// builder to say which subclass you are creating — the collection you
    /// `POST` to must agree with it.
    pub struct AccountCreate {
        @required {
            /// Name of the account. **Required on create.**
            name: String,
        }
        /// Who holds the account.
        ///
        /// Required by the four subclass `_FVO`s but not by the base, so it is
        /// optional here — the one type covers all five, and demanding it would
        /// reject a payload the base schema permits.
        related_party: Vec<RelatedParty>,
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// How the provider classifies the account.
        account_type: String,
        /// Where the account starts in its lifecycle.
        state: String,
        /// When the account was last changed.
        last_update: Timestamp,
        /// The most the account may owe.
        credit_limit: Money,
        /// Opening balances.
        account_balance: Vec<AccountBalance>,
        /// Links to other accounts.
        account_relationship: Vec<AccountRelationship>,
        /// People to contact about this account.
        contact: Vec<Contact>,
        /// Tax exemptions claimed.
        tax_exemption: Vec<TaxExemptionCertificate>,
        /// How bills are formatted and delivered.
        bill_structure: BillStructure,
        /// How the holder pays by default.
        default_payment_method: Ref<PaymentMethod>,
        /// The financial account money is posted to.
        financial_account: Ref<FinancialAccount>,
        /// Instalment arrangements.
        payment_plan: Vec<PaymentPlan>,
        /// Whether the account is paid up.
        payment_status: String,
        /// Prepaid or postpaid.
        rating_type: String,
    }
}

tmf_struct! {
    @name = "Account";
    /// Body of a `PATCH` to any of the four account collections.
    pub struct AccountUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New account type.
        account_type: String,
        /// New lifecycle state.
        state: String,
        /// New last-changed timestamp.
        last_update: Timestamp,
        /// New credit limit.
        credit_limit: Money,
        /// Replacement balances.
        account_balance: Vec<AccountBalance>,
        /// Replacement relationships.
        account_relationship: Vec<AccountRelationship>,
        /// Replacement contacts.
        contact: Vec<Contact>,
        /// Replacement tax exemptions.
        tax_exemption: Vec<TaxExemptionCertificate>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// New bill structure.
        bill_structure: BillStructure,
        /// New default payment method.
        default_payment_method: Ref<PaymentMethod>,
        /// New financial account.
        financial_account: Ref<FinancialAccount>,
        /// Replacement payment plans.
        payment_plan: Vec<PaymentPlan>,
        /// New payment status.
        payment_status: String,
        /// New rating type.
        rating_type: String,
    }
}

tmf_struct! {
    @name = "BillFormat", @ref = "BillFormatRef";
    /// How a bill is laid out.
    pub struct BillFormat {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this format.
        href: String,
        /// Name of the format.
        name: String,
        /// Narrative description.
        description: String,
        @renamed {
            /// The concrete class a `BillFormatRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "BillFormat";
    /// Body of a `POST /billFormat` — the v5 `BillFormat_FVO`.
    pub struct BillFormatCreate {
        @required {
            /// Name of the format. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
    }
}

tmf_struct! {
    @name = "BillFormat";
    /// Body of a `PATCH /billFormat/{id}` — the v5 `BillFormat_MVO`.
    pub struct BillFormatUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
    }
}

tmf_struct! {
    @name = "BillPresentationMedia", @ref = "BillPresentationMediaRef";
    /// How a bill reaches the customer — paper, email, portal.
    pub struct BillPresentationMedia {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this medium.
        href: String,
        /// Name of the medium.
        name: String,
        /// Narrative description.
        description: String,
        @renamed {
            /// The concrete class a `BillPresentationMediaRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "BillPresentationMedia";
    /// Body of a `POST /billPresentationMedia` — the v5 `_FVO` schema.
    pub struct BillPresentationMediaCreate {
        @required {
            /// Name of the medium. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
    }
}

tmf_struct! {
    @name = "BillPresentationMedia";
    /// Body of a `PATCH /billPresentationMedia/{id}` — the v5 `_MVO` schema.
    pub struct BillPresentationMediaUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
    }
}

tmf_struct! {
    @name = "BillingCycleSpecification", @ref = "BillingCycleSpecificationRef";
    /// When bills are produced, and how the dates around them are derived.
    ///
    /// Note the name: TMF678 references a `BillCycleSpecificationRef` — four
    /// characters shorter — and no specification reconciles the two, so this
    /// crate does not either.
    pub struct BillingCycleSpecification {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this specification.
        href: String,
        /// Name of the specification.
        name: String,
        /// Narrative description.
        description: String,
        /// How often bills are produced.
        frequency: String,
        /// The period each bill covers.
        billing_period: String,
        /// Days the billing date moves by.
        billing_date_shift: i64,
        /// Days between the billing date and when charges are applied.
        charge_date_offset: i64,
        /// Days between the billing date and when credits are applied.
        credit_date_offset: i64,
        /// Days between the billing date and posting.
        mailing_date_offset: i64,
        /// Days between the billing date and when payment falls due.
        payment_due_date_offset: i64,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
        @renamed {
            /// The concrete class a `BillingCycleSpecificationRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "BillingCycleSpecification";
    /// Body of a `POST /billingCycleSpecification` — the v5 `_FVO` schema.
    pub struct BillingCycleSpecificationCreate {
        @required {
            /// Name of the specification. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// How often bills are produced.
        frequency: String,
        /// The period each bill covers.
        billing_period: String,
        /// Days the billing date moves by.
        billing_date_shift: i64,
        /// Days between the billing date and when charges are applied.
        charge_date_offset: i64,
        /// Days between the billing date and when credits are applied.
        credit_date_offset: i64,
        /// Days between the billing date and posting.
        mailing_date_offset: i64,
        /// Days between the billing date and when payment falls due.
        payment_due_date_offset: i64,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "BillingCycleSpecification";
    /// Body of a `PATCH /billingCycleSpecification/{id}` — the v5 `_MVO`.
    pub struct BillingCycleSpecificationUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New frequency.
        frequency: String,
        /// New billing period.
        billing_period: String,
        /// New billing-date shift.
        billing_date_shift: i64,
        /// New charge-date offset.
        charge_date_offset: i64,
        /// New credit-date offset.
        credit_date_offset: i64,
        /// New mailing-date offset.
        mailing_date_offset: i64,
        /// New payment-due-date offset.
        payment_due_date_offset: i64,
        /// New validity period.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "AccountBalance";
    /// A balance held against an account, for a period.
    pub struct AccountBalance {
        /// Identifier of the balance.
        id: String,
        /// What kind of balance this is, e.g. `deposit`, `disputed`.
        balance_type: String,
        /// How much.
        amount: Money,
        /// Period the balance applies to.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "AccountRelationship";
    /// A typed link from one account to another.
    pub struct AccountRelationship {
        /// Identifier of the relationship.
        id: String,
        /// URI of the relationship.
        href: String,
        /// What kind of link this is.
        relationship_type: String,
        /// The account being referred to.
        account: Ref<Account>,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "BillStructure";
    /// How an account's bills are put together and delivered.
    pub struct BillStructure {
        /// The cycle the bills run on.
        cycle_specification: BillingCycleSpecification,
        /// The layout used.
        format: BillFormat,
        /// How the bill reaches the customer.
        presentation_media: Vec<BillPresentationMedia>,
    }
}

tmf_struct! {
    @name = "Contact";
    /// Someone to contact about an account.
    pub struct Contact {
        /// Identifier of the contact.
        id: String,
        /// Their name.
        contact_name: String,
        /// What kind of contact they are, e.g. `billing`.
        contact_type: String,
        /// The role they play.
        party_role_type: String,
        /// How to reach them.
        contact_medium: Vec<ContactMedium>,
        /// The party they are.
        related_party: RelatedParty,
        /// Period during which the contact applies.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "PaymentPlan";
    /// An instalment arrangement on an account.
    pub struct PaymentPlan {
        /// Identifier of the plan.
        id: String,
        /// What kind of plan this is.
        plan_type: String,
        /// Where the plan has got to.
        status: String,
        /// How often payments fall due.
        payment_frequency: String,
        /// How many payments the plan runs to.
        number_of_payments: i64,
        /// Which plan takes precedence.
        priority: i64,
        /// The total the plan covers.
        total_amount: Money,
        /// How the payments are made.
        payment_method: Ref<PaymentMethod>,
        /// Period during which the plan applies.
        valid_for: TimePeriod,
    }
}

tmf_entity!(
    Account,
    BillFormat,
    BillPresentationMedia,
    BillingCycleSpecification
);
tmf_patch_body!(
    AccountUpdate,
    BillFormatUpdate,
    BillPresentationMediaUpdate,
    BillingCycleSpecificationUpdate
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_subclass_is_recovered_from_its_discriminator() {
        for (name, kind, collection) in [
            ("Account", AccountKind::Base, "partyAccount"),
            ("BillingAccount", AccountKind::Billing, "billingAccount"),
            (
                "FinancialAccount",
                AccountKind::Financial,
                "financialAccount",
            ),
            ("PartyAccount", AccountKind::Party, "partyAccount"),
            (
                "SettlementAccount",
                AccountKind::Settlement,
                "settlementAccount",
            ),
        ] {
            assert_eq!(AccountKind::from_type_name(name), kind);
            assert_eq!(kind.collection(), collection);
        }
    }

    #[test]
    fn subclass_members_sit_on_the_one_type() {
        // `ratingType` belongs to `BillingAccount` alone; `financialAccount` to
        // the three billable subclasses. Both are typed here, so neither ends
        // up in `extensions`.
        let json = r#"{"@type":"BillingAccount","name":"Acme","ratingType":"postpaid","paymentStatus":"paid"}"#;
        let account: Account = serde_json::from_str(json).unwrap();

        assert_eq!(account.kind(), AccountKind::Billing);
        assert_eq!(account.rating_type.as_deref(), Some("postpaid"));
        assert!(account.extensions.is_empty());
    }

    #[test]
    fn an_unknown_subclass_round_trips_as_other() {
        let json = r#"{"@type":"VendorAccount","name":"n","quirk":1}"#;
        let account: Account = serde_json::from_str(json).unwrap();

        assert_eq!(account.kind(), AccountKind::Other);
        assert_eq!(account.extensions.get("quirk").unwrap(), 1);
        assert_eq!(
            serde_json::to_value(&account).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn creating_an_account_carries_its_subclass_and_holder() {
        // `relatedParty` is required by the four subclass `_FVO`s but not by
        // the base, so the one create type leaves it optional — see the field's
        // documentation.
        let account = AccountCreate::builder()
            .name("Acme Ltd")
            .related_party(vec![RelatedParty::default()])
            .at_type(AccountKind::Billing.type_name())
            .build();

        assert_eq!(account.at_type, "BillingAccount");
        assert_eq!(account.related_party.as_ref().map(Vec::len), Some(1));
    }
}
