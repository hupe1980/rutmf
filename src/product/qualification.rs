//! Product offering qualification — can this customer actually buy this?
//!
//! Mirrors **TMF679 Product Offering Qualification v5.0.0**. It sits between
//! the catalog and the order: a catalog says what a provider sells, and this
//! says what *this* customer, at *this* address, is eligible to buy.
//!
//! # Two questions, two resources
//!
//! The API asks eligibility in two directions, and gives each its own
//! collection:
//!
//! | Resource | Question |
//! |---|---|
//! | [`CheckProductOfferingQualification`] | "Can I have *these* offerings?" — you name them, the provider answers per item |
//! | [`QueryProductOfferingQualification`] | "What *can* I have?" — you give search criteria, the provider returns the eligible set |
//!
//! Both are **tasks**, not records: you `POST` a request and read the answer
//! back, and [`TaskState`] says how far it has got. Both nonetheless carry the
//! full five operations, because TMF679 declares `PATCH` and `DELETE` on each.
//!
//! # What a "no" carries
//!
//! An ineligible item is more useful than a rejection. A
//! [`CheckProductOfferingQualificationItem`] can come back with
//! [`EligibilityResultReason`]s explaining *why*, and with
//! [`AlternateProductOfferingProposal`]s naming what the customer could have
//! instead — which is what turns a failed eligibility check into a sale.

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{Channel, Note, Promotion, Ref, RelatedParty, TaskState, Timestamp};

use super::{Category, Product, ProductOffering};

tmf_struct! {
    @name = "CheckProductOfferingQualification";
    /// A request to confirm eligibility for offerings the client names.
    ///
    /// This is the **read model**. Use
    /// [`CheckProductOfferingQualificationCreate`] for `POST` and
    /// [`CheckProductOfferingQualificationUpdate`] for `PATCH`.
    pub struct CheckProductOfferingQualification {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this qualification.
        href: String,
        /// Narrative description.
        description: String,
        /// Sales channel the request came through.
        channel: Ref<Channel>,
        /// The offerings being checked, one entry each.
        check_product_offering_qualification_item:
            Vec<CheckProductOfferingQualificationItem>,
        /// Whether the provider should answer synchronously.
        instant_sync_qualification: bool,
        /// Whether to propose alternatives for ineligible items.
        provide_alternative: bool,
        /// Whether to return only the items the customer may have.
        provide_only_available: bool,
        /// Whether to explain each ineligible item.
        provide_result_reason: bool,
        /// The overall answer.
        qualification_result: String,
        /// How far the task has got.
        state: TaskState,
        /// When the request was created.
        creation_date: Timestamp,
        /// When the answer takes effect.
        effective_qualification_date: Timestamp,
        /// When the provider expects to finish.
        expected_qualification_completion_date: Timestamp,
        /// When the answer stops being valid.
        expiration_date: Timestamp,
        /// When the client asked for it to be finished.
        requested_qualification_completion_date: Timestamp,
        /// Free-form notes.
        note: Vec<Note>,
        /// Parties related to the request.
        related_party: Vec<RelatedParty>,
    }
}

tmf_entity!(CheckProductOfferingQualification);

tmf_struct! {
    @name = "CheckProductOfferingQualification";
    /// Body of a `POST /checkProductOfferingQualification` — the v5
    /// `CheckProductOfferingQualification_FVO`.
    ///
    /// TMF679 requires nothing but `@type` on create — but it also **removes
    /// every member that holds the answer**. There is no `state`, no
    /// `qualificationResult`, no `effectiveQualificationDate` and no `id`: a
    /// client asks the question, and the provider is the only one that may
    /// write down the reply. A request that tried to pre-fill it would not
    /// compile.
    pub struct CheckProductOfferingQualificationCreate {
        /// Narrative description.
        description: String,
        /// Sales channel the request came through.
        channel: Ref<Channel>,
        /// The offerings being checked, one entry each.
        check_product_offering_qualification_item: Vec<CheckProductOfferingQualificationItem>,
        /// Whether the provider should answer synchronously.
        instant_sync_qualification: bool,
        /// Whether to propose alternatives for ineligible items.
        provide_alternative: bool,
        /// Whether to return only the items the customer may have.
        provide_only_available: bool,
        /// Whether to explain each ineligible item.
        provide_result_reason: bool,
        /// When the client asked for it to be finished.
        requested_qualification_completion_date: Timestamp,
        /// Free-form notes.
        note: Vec<Note>,
        /// Parties related to the request.
        related_party: Vec<RelatedParty>,
    }
}

tmf_struct! {
    @name = "CheckProductOfferingQualification";
    /// Body of a `PATCH /checkProductOfferingQualification/{id}` — the v5
    /// `CheckProductOfferingQualification_MVO`.
    pub struct CheckProductOfferingQualificationUpdate {
        /// Narrative description.
        description: String,
        /// Sales channel the request came through.
        channel: Ref<Channel>,
        /// The offerings being checked, one entry each.
        check_product_offering_qualification_item:
            Vec<CheckProductOfferingQualificationItem>,
        /// Whether the provider should answer synchronously.
        instant_sync_qualification: bool,
        /// Whether to propose alternatives for ineligible items.
        provide_alternative: bool,
        /// Whether to return only the items the customer may have.
        provide_only_available: bool,
        /// Whether to explain each ineligible item.
        provide_result_reason: bool,
        /// The overall answer.
        qualification_result: String,
        /// How far the task has got.
        state: TaskState,
        /// When the answer takes effect.
        effective_qualification_date: Timestamp,
        /// When the provider expects to finish.
        expected_qualification_completion_date: Timestamp,
        /// When the answer stops being valid.
        expiration_date: Timestamp,
        /// When the client asked for it to be finished.
        requested_qualification_completion_date: Timestamp,
        /// Free-form notes.
        note: Vec<Note>,
        /// Parties related to the request.
        related_party: Vec<RelatedParty>,
    }
}

tmf_patch_body!(CheckProductOfferingQualificationUpdate);

tmf_struct! {
    @name = "QueryProductOfferingQualification";
    /// A request for the offerings a customer is eligible for.
    ///
    /// The open-ended counterpart to
    /// [`CheckProductOfferingQualification`]: instead of naming offerings, the
    /// client gives `searchCriteria` and the provider returns what fits.
    pub struct QueryProductOfferingQualification {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this qualification.
        href: String,
        /// Narrative description.
        description: String,
        /// Sales channel the request came through.
        channel: Ref<Channel>,
        /// The offerings the customer is eligible for.
        qualified_product_offering_item: Vec<QueryProductOfferingQualificationItem>,
        /// Whether the provider should answer synchronously.
        instant_sync_qualification: bool,
        /// How far the task has got.
        state: TaskState,
        /// When the request was created.
        creation_date: Timestamp,
        /// When the answer takes effect.
        effective_qualification_date: Timestamp,
        /// When the provider expects to finish.
        expected_qualification_completion_date: Timestamp,
        /// When the answer stops being valid.
        expiration_date: Timestamp,
        /// When the client asked for it to be finished.
        requested_qualification_completion_date: Timestamp,
        /// Free-form notes.
        note: Vec<Note>,
        /// Parties related to the request.
        related_party: Vec<RelatedParty>,
        @renamed {
            /// What to search for.
            ///
            /// TMF679 types this as a bare `object` with no members of its own,
            /// so it stays untyped here rather than being given a shape the
            /// specification does not define.
            "searchCriteria" search_criteria: serde_json::Value,
        }
    }
}

tmf_entity!(QueryProductOfferingQualification);

tmf_struct! {
    @name = "QueryProductOfferingQualification";
    /// Body of a `POST /queryProductOfferingQualification` — the v5
    /// `QueryProductOfferingQualification_FVO`.
    pub struct QueryProductOfferingQualificationCreate {
        /// Narrative description.
        description: String,
        /// Sales channel the request came through.
        channel: Ref<Channel>,
        /// Whether the provider should answer synchronously.
        instant_sync_qualification: bool,
        /// When the client asked for it to be finished.
        requested_qualification_completion_date: Timestamp,
        /// Free-form notes.
        note: Vec<Note>,
        /// Parties related to the request.
        related_party: Vec<RelatedParty>,
        @renamed {
            /// What to search for.
            "searchCriteria" search_criteria: serde_json::Value,
        }
    }
}

tmf_struct! {
    @name = "QueryProductOfferingQualification";
    /// Body of a `PATCH /queryProductOfferingQualification/{id}` — the v5
    /// `QueryProductOfferingQualification_MVO`.
    pub struct QueryProductOfferingQualificationUpdate {
        /// Narrative description.
        description: String,
        /// Sales channel the request came through.
        channel: Ref<Channel>,
        /// The offerings the customer is eligible for.
        qualified_product_offering_item: Vec<QueryProductOfferingQualificationItem>,
        /// Whether the provider should answer synchronously.
        instant_sync_qualification: bool,
        /// How far the task has got.
        state: TaskState,
        /// When the answer takes effect.
        effective_qualification_date: Timestamp,
        /// When the provider expects to finish.
        expected_qualification_completion_date: Timestamp,
        /// When the answer stops being valid.
        expiration_date: Timestamp,
        /// When the client asked for it to be finished.
        requested_qualification_completion_date: Timestamp,
        /// Free-form notes.
        note: Vec<Note>,
        /// Parties related to the request.
        related_party: Vec<RelatedParty>,
        @renamed {
            /// What to search for.
            "searchCriteria" search_criteria: serde_json::Value,
        }
    }
}

tmf_patch_body!(QueryProductOfferingQualificationUpdate);

tmf_struct! {
    @name = "CheckProductOfferingQualificationItem";
    /// One offering being checked, and the answer for it.
    pub struct CheckProductOfferingQualificationItem {
        /// Identifier of the item within the request.
        id: String,
        /// What the customer wants to do — `add`, `modify`, `delete`.
        action: String,
        /// The offering being checked.
        product_offering: Ref<ProductOffering>,
        /// The catalog category it sits under.
        category: Ref<Category>,
        /// An existing product the item acts on.
        product: Product,
        /// A promotion applied to the item — TMF671.
        promotion: Ref<Promotion>,
        /// The answer for this item.
        qualification_item_result: String,
        /// How far this item has got.
        state: String,
        /// When the customer wants it active.
        expected_activation_date: Timestamp,
        /// Why the item is not eligible.
        eligibility_result_reason: Vec<EligibilityResultReason>,
        /// What the customer could have instead.
        alternate_product_offering_proposal: Vec<AlternateProductOfferingProposal>,
        /// Links to other items in the same request.
        qualification_item_relationship: Vec<ProductOfferingQualificationItemRelationship>,
        /// Errors that stopped the item being assessed.
        termination_error: Vec<TerminationError>,
        /// Free-form notes.
        note: Vec<Note>,
        @renamed {
            /// Items bundled beneath this one.
            ///
            /// Note the capital: TMF679 names this member after its own type,
            /// which is the only member in the vendored corpus to start with an
            /// upper-case letter. It is reproduced verbatim, because a server
            /// matching the specification will send it that way.
            "CheckProductOfferingQualificationItem"
                nested_item: Vec<CheckProductOfferingQualificationItem>,
        }
    }
}

tmf_struct! {
    @name = "QueryProductOfferingQualificationItem";
    /// One offering the customer is eligible for.
    pub struct QueryProductOfferingQualificationItem {
        /// Identifier of the item within the answer.
        id: String,
        /// The offering the customer may have.
        product_offering: Ref<ProductOffering>,
        /// The catalog category it sits under.
        category: Ref<Category>,
        /// An existing product the item relates to.
        product: Product,
        /// A promotion applied to the item — TMF671.
        promotion: Ref<Promotion>,
        /// Links to other items in the same answer.
        qualification_item_relationship: Vec<ProductOfferingQualificationItemRelationship>,
    }
}

tmf_struct! {
    @name = "AlternateProductOfferingProposal";
    /// Something the customer could have instead of an ineligible item.
    pub struct AlternateProductOfferingProposal {
        /// Identifier of the proposal.
        id: String,
        /// The offering being proposed.
        alternate_product_offering: Ref<ProductOffering>,
        /// An existing product the proposal would act on.
        alternate_product: Product,
        /// A promotion that applies to the proposal — TMF671.
        promotion: Ref<Promotion>,
        /// When the alternative could be activated.
        alternate_activation_date: Timestamp,
    }
}

tmf_struct! {
    @name = "EligibilityResultReason";
    /// Why an item is not available to this customer.
    pub struct EligibilityResultReason {
        /// Machine-readable reason code.
        code: String,
        /// Human-readable explanation.
        label: String,
    }
}

tmf_struct! {
    @name = "ProductOfferingQualificationItemRelationship";
    /// A link between two items of one qualification.
    pub struct ProductOfferingQualificationItemRelationship {
        /// Identifier of the item at the other end.
        id: String,
        /// Kind of link, e.g. `dependency`.
        relationship_type: String,
    }
}

tmf_struct! {
    @name = "TerminationError";
    /// An error that stopped an item being assessed.
    pub struct TerminationError {
        /// Identifier of the error.
        id: String,
        /// The error itself.
        value: String,
    }
}
