//! `CustomerBill` and the rest of TMF678.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct, tmf_value};
use crate::core::{
    Attachment, BillCycleSpecification, BillingAccount, Characteristic, FinancialAccount, Money,
    Payment, PaymentMethod, Ref, RelatedParty, TaxItem, TimePeriod, Timestamp,
};
use crate::product::Product;

/// Where a [`CustomerBill`] has got to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum CustomerBillState {
    /// Produced but not yet checked.
    #[serde(rename = "new")]
    New,
    /// Held back from the customer, pending a query.
    #[serde(rename = "onHold")]
    OnHold,
    /// Checked and correct.
    #[serde(rename = "validated")]
    Validated,
    /// Delivered to the customer.
    #[serde(rename = "sent")]
    Sent,
    /// Paid in full.
    #[serde(rename = "settled")]
    Settled,
    /// Some of the amount has been paid.
    #[serde(rename = "partiallyPaid")]
    PartiallyPaid,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl CustomerBillState {
    /// Whether the bill still has money outstanding.
    ///
    /// An unrecognised state counts as **outstanding**: a collections process
    /// that writes off what it does not understand is worse than one that asks.
    #[must_use]
    pub fn is_outstanding(&self) -> bool {
        !matches!(self, Self::Settled)
    }
}

/// Whether a bill came from the regular run or was asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum CustomerBillRunType {
    /// Produced by the scheduled billing cycle.
    #[serde(rename = "onCycle")]
    OnCycle,
    /// Produced outside the cycle, on request.
    #[serde(rename = "offCycle")]
    OffCycle,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// How an on-demand bill request is progressing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum CustomerBillOnDemandState {
    /// The bill is being produced.
    #[serde(rename = "inProgress")]
    InProgress,
    /// The request was refused.
    #[serde(rename = "rejected")]
    Rejected,
    /// The bill is ready.
    #[serde(rename = "done")]
    Done,
    /// Production failed.
    #[serde(rename = "terminatedWithError")]
    TerminatedWithError,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl CustomerBillOnDemandState {
    /// Whether the request has stopped moving.
    ///
    /// An unrecognised state is **not** terminal, so a client polling for the
    /// bill keeps polling rather than giving up on a state it does not know.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            Self::Done | Self::Rejected | Self::TerminatedWithError
        )
    }
}

tmf_struct! {
    @name = "CustomerBill", @ref = "CustomerBillRef";
    /// An issued invoice: what the customer owes, and for what period.
    ///
    /// ```
    /// use rutmf::bill::{CustomerBill, CustomerBillState};
    ///
    /// let json = r#"{"@type":"CustomerBill","state":"sent"}"#;
    /// let bill: CustomerBill = serde_json::from_str(json).unwrap();
    ///
    /// assert!(bill.state.unwrap().is_outstanding());
    /// ```
    pub struct CustomerBill {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this bill.
        href: String,
        /// Name of the bill, as a `CustomerBillRef` carries it.
        name: String,
        /// The bill number the customer sees.
        bill_no: String,
        /// How the provider classifies this bill.
        category: String,
        /// Where the bill has got to.
        state: CustomerBillState,
        /// Whether it came from the cycle or was asked for.
        run_type: CustomerBillRunType,
        /// When the bill was produced.
        bill_date: Timestamp,
        /// When the next one is due to be produced.
        next_bill_date: Timestamp,
        /// When payment is due.
        payment_due_date: Timestamp,
        /// When the bill was last changed.
        last_update: Timestamp,
        /// The period the charges cover.
        billing_period: TimePeriod,
        /// The total the customer owes.
        amount_due: Money,
        /// What is still unpaid.
        remaining_amount: Money,
        /// The total before tax.
        tax_excluded_amount: Money,
        /// The total including tax.
        tax_included_amount: Money,
        /// The tax applied, broken down.
        tax_item: Vec<TaxItem>,
        /// Payments already applied to this bill.
        applied_payment: Vec<AppliedPayment>,
        /// The account this bill is issued against.
        billing_account: Ref<BillingAccount>,
        /// The financial account the money lands in.
        financial_account: Ref<FinancialAccount>,
        /// How the customer is set up to pay.
        payment_method: Ref<PaymentMethod>,
        /// The billing run that produced this bill.
        bill_cycle: Ref<BillCycle>,
        /// The bill itself, as a document.
        bill_document: Vec<Attachment>,
        /// Parties related to the bill.
        related_party: Vec<RelatedParty>,
        @renamed {
            /// The concrete class a `CustomerBillRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "CustomerBill";
    /// Body of a `PATCH /customerBill/{id}` — the v5 `CustomerBill_MVO`.
    ///
    /// Deliberately almost empty. TMF678's `_MVO` declares only `state` and
    /// `billCycle`, so this type cannot express a change to `amountDue`,
    /// `taxItem` or `billNo` — an issued invoice is evidence, and the
    /// specification does not let a client rewrite what it says.
    ///
    /// There is no `CustomerBillCreate` for the same reason: `POST
    /// /customerBill` does not exist. Bills are produced by a billing run, or
    /// requested through [`CustomerBillOnDemandCreate`].
    pub struct CustomerBillUpdate {
        /// The state to move the bill to.
        state: CustomerBillState,
        /// The billing run to attribute it to.
        bill_cycle: Ref<BillCycle>,
    }
}

tmf_struct! {
    // TMF678 defines no `CustomerBillOnDemandRef`: an on-demand request is
    // addressed, never referenced.
    @name = "CustomerBillOnDemand";
    /// A request for a bill outside the regular cycle.
    pub struct CustomerBillOnDemand {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this request.
        href: String,
        /// Name of the request.
        name: String,
        /// Narrative description.
        description: String,
        /// How the request is progressing.
        state: CustomerBillOnDemandState,
        /// When the request was last changed.
        last_update: Timestamp,
        /// The account to bill.
        billing_account: Ref<BillingAccount>,
        /// The bill produced, once there is one.
        customer_bill: Ref<CustomerBill>,
        /// The party the request is for.
        related_party: RelatedParty,
    }
}

tmf_struct! {
    @name = "CustomerBillOnDemand";
    /// Body of a `POST /customerBillOnDemand` — the v5 `_FVO` schema.
    ///
    /// There is no matching update type: TMF678 defines `POST` and `GET` on
    /// this collection and nothing else. Poll the returned request with
    /// [`CustomerBillOnDemandState::is_finished`].
    pub struct CustomerBillOnDemandCreate {
        @required {
            /// The account to bill. **Required on create.**
            billing_account: Ref<BillingAccount>,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Name of the request.
        name: String,
        /// Narrative description.
        description: String,
        /// The state to open the request in, where the server allows one.
        state: CustomerBillOnDemandState,
        /// When the request was last changed.
        last_update: Timestamp,
        /// The bill produced.
        customer_bill: Ref<CustomerBill>,
        /// The party the request is for.
        related_party: RelatedParty,
    }
}

tmf_struct! {
    // Likewise, no `AppliedCustomerBillingRateRef` is declared.
    @name = "AppliedCustomerBillingRate";
    /// One charge that went onto a bill.
    ///
    /// Read-only: TMF678 declares `GET` on this collection and nothing else.
    /// Charges are produced by rating, not authored by a client.
    pub struct AppliedCustomerBillingRate {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this charge.
        href: String,
        /// Name of the charge.
        name: String,
        /// Narrative description.
        description: String,
        /// How the provider classifies the charge.
        applied_billing_rate_type: String,
        /// When the charge was applied.
        date: Timestamp,
        /// Whether it has appeared on a bill yet.
        is_billed: bool,
        /// The period the charge covers.
        period_coverage: TimePeriod,
        /// The amount before tax.
        tax_excluded_amount: Money,
        /// The amount including tax.
        tax_included_amount: Money,
        /// The tax applied.
        applied_tax: Vec<AppliedBillingTaxRate>,
        /// The bill this charge appears on.
        bill: Ref<CustomerBill>,
        /// The account it is charged to.
        billing_account: Ref<BillingAccount>,
        /// The product being charged for.
        product: Ref<Product>,
        /// Provider-defined attributes of the charge.
        characteristic: Vec<Characteristic>,
    }
}

tmf_struct! {
    @name = "BillCycle", @ref = "BillCycleRef";
    /// One run of the billing process.
    ///
    /// Read-only: cycles are scheduled by the billing system, not created by a
    /// client.
    pub struct BillCycle {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this cycle.
        href: String,
        /// Name of the cycle.
        name: String,
        /// Narrative description.
        description: String,
        /// The period the cycle covers.
        billing_period: String,
        /// When bills are produced.
        billing_date: Timestamp,
        /// When charges are applied.
        charge_date: Timestamp,
        /// When credits are applied.
        credit_date: Timestamp,
        /// When bills are posted.
        mailing_date: Timestamp,
        /// When payment falls due.
        payment_due_date: Timestamp,
        /// Period during which the cycle is valid.
        valid_for: TimePeriod,
        @renamed {
            /// The specification this cycle runs to.
            ///
            /// # The schema and the example disagree about this name
            ///
            /// TMF678's `BillCycle` schema declares the member as
            /// `BillCycleSpecification`, capitalised, against the `camelCase`
            /// of every other v5 member — while its own retrieve example sends
            /// `billCycleSpecification`. One of the two is a typo and the
            /// specification does not say which.
            ///
            /// This types the **schema** spelling, because that is what
            /// `tests/coverage.rs` checks a conformant server against. A server
            /// that follows the example instead still round-trips losslessly:
            /// the lowercase member is captured in `extensions` rather than
            /// dropped. Read both if you are integrating against a real
            /// deployment.
            "BillCycleSpecification" bill_cycle_specification: Ref<BillCycleSpecification>,
            /// The concrete class a `BillCycleRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "AppliedBillingTaxRate";
    /// Tax applied to one charge.
    pub struct AppliedBillingTaxRate {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI.
        href: String,
        /// The kind of tax.
        tax_category: String,
        /// The amount of tax.
        tax_amount: Money,
        @decimal {
            /// The rate applied, as a percentage.
            ///
            /// TMF678 spells this `number/float`; it is a [`Decimal`] here for
            /// the same reason [`Money::value`](crate::core::Money) is — this
            /// rate is multiplied into a monetary amount, and a binary float
            /// carries its rounding error into the result.
            tax_rate: Decimal,
        }
    }
}

tmf_value! {
    /// A payment set against a bill.
    ///
    /// A plain object: TMF678 gives it no `@type`.
    pub struct AppliedPayment {
        /// How much of the payment was applied here.
        applied_amount: Money,
        /// The payment it came from.
        payment: Ref<Payment>,
    }
}

tmf_entity!(
    CustomerBill,
    CustomerBillOnDemand,
    AppliedCustomerBillingRate,
    BillCycle
);
tmf_patch_body!(CustomerBillUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bill_patch_can_move_the_state_and_nothing_else() {
        // The whole point of the `_MVO`: an issued invoice is evidence. There
        // is no `amount_due` field to set, so this cannot compile into a
        // request that rewrites what the customer owes.
        let patch = CustomerBillUpdate::builder()
            .state(CustomerBillState::Settled)
            .build();

        let json = serde_json::to_value(&patch).unwrap();
        assert_eq!(json["state"], "settled");
        assert!(json.get("amountDue").is_none());
        assert!(json.get("billNo").is_none());
    }

    #[test]
    fn an_unknown_state_leaves_the_bill_outstanding() {
        let json = r#"{"@type":"CustomerBill","state":"disputed"}"#;
        let bill: CustomerBill = serde_json::from_str(json).unwrap();
        let state = bill.state.clone().expect("a state");

        assert_eq!(state, CustomerBillState::Other("disputed".into()));
        assert!(
            state.is_outstanding(),
            "writing off an unrecognised state is worse than asking"
        );
        assert_eq!(
            serde_json::to_value(&bill).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn the_example_spelling_survives_even_though_the_schema_differs() {
        // TM Forum's own example lowercases the member its schema capitalises.
        // The typed field follows the schema; the example's spelling is not
        // lost, it is captured — which is the whole point of `extensions`.
        let json = r#"{"@type":"BillCycle","billCycleSpecification":{"id":"BCSPEC-M"}}"#;
        let cycle: BillCycle = serde_json::from_str(json).unwrap();

        assert!(cycle.bill_cycle_specification.is_none());
        assert!(cycle.extensions.get("billCycleSpecification").is_some());
        assert_eq!(
            serde_json::to_value(&cycle).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap(),
        );
    }

    #[test]
    fn the_capitalised_wire_name_is_preserved() {
        // TMF678 spells this member `BillCycleSpecification`, against the
        // camelCase of every other v5 member.
        let json = r#"{"@type":"BillCycle","BillCycleSpecification":{"id":"spec-1","@type":"BillCycleSpecificationRef"}}"#;
        let cycle: BillCycle = serde_json::from_str(json).unwrap();

        assert_eq!(
            cycle
                .bill_cycle_specification
                .as_ref()
                .map(|r| r.id.as_str()),
            Some("spec-1")
        );
        assert!(cycle.extensions.is_empty(), "the member must be typed");
    }

    #[test]
    fn an_on_demand_request_polls_until_finished() {
        assert!(!CustomerBillOnDemandState::InProgress.is_finished());
        assert!(CustomerBillOnDemandState::Done.is_finished());
        assert!(
            !CustomerBillOnDemandState::Other("queued".into()).is_finished(),
            "an unrecognised state must not stop a poller"
        );
    }
}
