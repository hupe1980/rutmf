//! Billing: what the customer owes — TMF678.
//!
//! A [`CustomerBill`] is an issued invoice. [`AppliedCustomerBillingRate`] is
//! the individual charge that went onto one, [`BillCycle`] is the run that
//! produced it, and [`CustomerBillOnDemand`] asks for one outside the cycle.
//!
//! # A bill is mostly read-only, and the types say so
//!
//! TMF678 is the first API in this crate where **no resource has the full CRUD
//! surface**, and that is not an oversight in the specification — it is what a
//! billing system is. You do not `POST` an invoice into existence or delete one
//! that has been issued; the billing run creates it and it stays as evidence.
//!
//! So the client offers exactly what the paths declare:
//!
//! | Resource | Operations |
//! |---|---|
//! | [`CustomerBill`] | list, get, **patch** — no create, no delete |
//! | [`CustomerBillOnDemand`] | list, get, **create** — a task |
//! | [`AppliedCustomerBillingRate`] | list, get — **read-only** |
//! | [`BillCycle`] | list, get — **read-only** |
//!
//! [`CustomerBillUpdate`] goes further. TMF678's `_MVO` drops every member
//! except `state` and `billCycle`, so the type has no `amountDue`, no
//! `taxItem`, no `billNo` — you can move a bill through its lifecycle, and you
//! cannot rewrite what it says the customer owes. A `PATCH` that tried would
//! not compile.

mod customer_bill;

pub use customer_bill::{
    AppliedBillingTaxRate, AppliedCustomerBillingRate, AppliedPayment, BillCycle, CustomerBill,
    CustomerBillOnDemand, CustomerBillOnDemandCreate, CustomerBillOnDemandState,
    CustomerBillRunType, CustomerBillState, CustomerBillUpdate,
};
