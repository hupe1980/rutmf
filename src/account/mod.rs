//! Accounts: who is billed, and how — TMF666.
//!
//! An [`Account`] is the thing a bill is issued against. TMF666 completes the
//! monetisation picture that TMF678 starts: a
//! [`CustomerBill`](crate::bill::CustomerBill) names a `billingAccount` and a
//! `financialAccount`, and this is the API that defines them.
//!
//! # One type, five schemas, four collections
//!
//! TMF666 models `Account` as an abstract base with four `@type`-discriminated
//! subclasses — `BillingAccount`, `FinancialAccount`, `PartyAccount`,
//! `SettlementAccount` — and then exposes each as its *own collection*. So
//! `GET /billingAccount` and `GET /financialAccount` return the same shape
//! differing only in `@type` and which members are populated.
//!
//! This crate keeps that as one [`Account`] type carrying the union, with
//! [`AccountKind`] recovering which subclass a server sent — the same treatment
//! `ContactMedium` and `ResourceSpecification` get. The client still has four
//! sets of methods, because there are four paths.
//!
//! # Two names for one idea
//!
//! TMF666 calls its billing-cycle template `BillingCycleSpecification`; TMF678
//! references `BillCycleSpecificationRef`. The two names differ by four
//! characters and no specification reconciles them, so this crate models
//! TMF666's as a real type and leaves TMF678's reference pointing at a marker.
//! Merging them would assert an equivalence TM Forum has not.

mod model;

pub use model::{
    Account, AccountBalance, AccountCreate, AccountKind, AccountRelationship, AccountUpdate,
    BillFormat, BillFormatCreate, BillFormatUpdate, BillPresentationMedia,
    BillPresentationMediaCreate, BillPresentationMediaUpdate, BillStructure,
    BillingCycleSpecification, BillingCycleSpecificationCreate, BillingCycleSpecificationUpdate,
    Contact, PaymentPlan,
};
