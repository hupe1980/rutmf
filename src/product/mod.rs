//! The product domain: what is sold, and what a customer has.
//!
//! Mirrors **TMF620 Product Catalog Management v5.0** — the catalog side, what
//! a provider *offers* — and **TMF637 Product Inventory Management v5.0.0** —
//! the inventory side, what a customer actually *has*. They are one domain: a
//! [`Product`] realises a [`ProductOffering`], which realises a
//! [`ProductSpecification`].
//!
//! Each
//! top-level resource appears three times, matching the v5 schema triple:
//!
//! - `ProductOffering` — the read model returned by `GET`
//! - `ProductOfferingCreate` — the `_FVO` body accepted by `POST`
//! - `ProductOfferingUpdate` — the `_MVO` body accepted by `PATCH`
//!
//! See [`crate::core`] for why the variants exist and what differs between
//! them, and [`crate::core::macros`] for how each is declared.

mod catalog;
mod inventory;
mod job;
mod offering;
mod price;
mod qualification;
mod specification;

pub use catalog::{
    Category, CategoryCreate, CategoryUpdate, ProductCatalog, ProductCatalogCreate,
    ProductCatalogUpdate,
};
pub use inventory::{
    AgreementItemRef, PriceAlteration, Product, ProductCreate, ProductPrice, ProductRelationship,
    ProductStatus, ProductTerm, ProductUpdate, RelatedOrderItem,
};
pub use job::{ExportJob, ExportJobCreate, ImportJob, ImportJobCreate, JobState};
pub use offering::{
    AllowedProductAction, BundledGroupProductOffering, BundledGroupProductOfferingOption,
    BundledProductOffering, BundledProductOfferingOption, OfferingRelationship, ProductOffering,
    ProductOfferingCreate, ProductOfferingUpdate,
};
pub use price::{
    BundledPriceRelationship, PriceRelationship, PricingLogicAlgorithm, ProductOfferingPrice,
    ProductOfferingPriceCreate, ProductOfferingPriceUpdate, ProductOfferingTerm,
};
pub use qualification::{
    AlternateProductOfferingProposal, CheckProductOfferingQualification,
    CheckProductOfferingQualificationCreate, CheckProductOfferingQualificationItem,
    CheckProductOfferingQualificationUpdate, EligibilityResultReason,
    ProductOfferingQualificationItemRelationship, QueryProductOfferingQualification,
    QueryProductOfferingQualificationCreate, QueryProductOfferingQualificationItem,
    QueryProductOfferingQualificationUpdate, TerminationError,
};
pub use specification::{
    BundledProductSpecification, ProductSpecification, ProductSpecificationCreate,
    ProductSpecificationUpdate, SpecificationRelationship, TargetProductSchema,
};
