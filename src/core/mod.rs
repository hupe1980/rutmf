//! Types shared by every TM Forum domain.
//!
//! This module is pure data: no I/O, no async, no HTTP. It builds cleanly for
//! `wasm32-unknown-unknown` and is the only module the domain models depend on.
//!
//! # The three shapes of a resource
//!
//! TMF v5 defines each resource three times, and this crate mirrors that:
//!
//! | OAS schema | Rust type | Used for |
//! |---|---|---|
//! | `ProductOffering` | [`ProductOffering`] | `GET` responses |
//! | `ProductOffering_FVO` | [`ProductOfferingCreate`] | `POST` bodies |
//! | `ProductOffering_MVO` | [`ProductOfferingUpdate`] | `PATCH` bodies |
//!
//! The variants differ in which members exist and which are *required*: a
//! create body must carry `name` and `lifecycleStatus`, while a patch body must
//! not carry `id` or `href` at all. Encoding that in the type system means the
//! compiler rejects a malformed request before it reaches the wire.
//!
//! # What else lives here
//!
//! Two types that look like client concerns are here rather than in
//! [`crate::api`], because they are I/O-free wire data and the layering rule is
//! about I/O rather than about topic:
//!
//! - [`TmfEvent`] — the notification envelope. A webhook handler, a queue
//!   consumer or a server implementation needs it without an HTTP client;
//!   *registering* a subscription is the client's job.
//! - [`JsonPatchOp`] — an RFC 6902 operation. Which of the four v5 `PATCH`
//!   content types a list is sent under is the client's concern.
//!
//! Both are re-exported from [`crate::api`], so client-side code has one import
//! path.
//!
//! # Reference target markers
//!
//! A `ProductOffering` carries a `ChannelRef` into sales-channel management; a
//! `Customer` carries an `AccountRef` into TMF666. Neither API defines the
//! target, only a reference to it. So this module declares zero-sized markers —
//! [`Channel`], [`Place`], [`Promotion`], [`Quote`] and the rest — which give
//! [`Ref<T>`](Ref) a target so the reference stays typed, and which carry the
//! `@type` and `@referredType` values the v5 schemas expect.
//!
//! They live here rather than in a domain module because the same target is
//! referenced from several domains: `Place` appears in both product and party
//! payloads.
//!
//! **A marker is usually a placeholder, not a destination.** When this crate
//! grows a real model for the API that owns the target, the marker is deleted
//! and the references repoint at the real type — which is what happened to
//! `ResourceSpecification` and `ResourceCandidate` when TMF634 arrived, and to
//! `Account` when TMF666 did. That is also what makes
//! [`resolve`](crate::api::ResolveRef::resolve) start working for them: a
//! marker has no collection to fetch from, and a real type does.
//!
//! Two are **not** placeholders and will not go away. [`BillingAccount`] and
//! [`FinancialAccount`] are subclasses of a type this crate *does* model, and
//! TM Forum gives each its own `…Ref` class. Since `Ref<T>` binds one target
//! type to one class name, a reference to a subclass needs a target of its own
//! even when the entity itself is modelled.
//!
//! [`ProductOffering`]: crate::product::ProductOffering
//! [`ProductOfferingCreate`]: crate::product::ProductOfferingCreate
//! [`ProductOfferingUpdate`]: crate::product::ProductOfferingUpdate

mod attachment;
mod characteristic;
mod error;
mod event;
mod extensible;
pub mod macros;
mod party;
mod patch;
mod reference;
mod refs;

#[cfg(feature = "schemars")]
mod schema;
mod state;
mod value;

pub use attachment::{Attachment, ExternalIdentifier};
pub use characteristic::{
    Characteristic, CharacteristicRelationship, CharacteristicSpecification,
    CharacteristicSpecificationRelationship, CharacteristicValueSpecification,
    CharacteristicValueUse, SpecificationTarget, ValueKind,
};
pub use error::TmfError;
pub use event::{EventKind, TmfEvent};
pub use extensible::{Extensions, TmfType, default_ref_type, default_type};
pub use party::{Party, PartyOrPartyRole, PartyRole, RelatedParty};
pub use patch::{JsonPatchOp, PatchOperation};
pub use reference::Ref;
pub use refs::{
    Account, Agreement, AgreementSpecification, AlarmedObject, AnyEntity, Appointment,
    AssociationSpecification, BillCycleSpecification, BillingAccount, Channel, ConnectionPoint,
    ConnectionPointSpecification, Constraint, EndpointSpecification, FinancialAccount,
    GeographicAddress, Intent, IntentSpecification, MarketSegment, Payment, PaymentMethod,
    PermissionSpecificationSet, Place, Policy, ProductOfferingQualification, Promotion, Quote,
    Schedule, ServiceCandidate, ServiceLevelAgreement, ServiceSpecification, Threshold,
};
pub use state::ItemAction;
pub use value::{
    CreditProfile, Duration, FeatureRelationshipType, Money, Note, PlaceRefOrValue, Price,
    Quantity, RelatedPlace, TaskState, TaxDefinition, TaxExemptionCertificate, TaxItem, TimePeriod,
    Timestamp, decimal_opt,
};

/// A body that a `PATCH` may carry as a whole-resource merge.
///
/// Implemented by the `…Update` type of every resource — the v5 `_MVO` schema —
/// and by nothing else. That is what lets
/// [`Patch`](crate::api::Patch) accept `&update` and `&[JsonPatchOp]` through
/// the same argument without the two conversions colliding, and it stops a
/// `PATCH` being handed a body that is not a patch body at all.
pub trait PatchBody: serde::Serialize {}

/// A TM Forum resource that is addressable by `id` and `href`.
///
/// Implemented by the read model of every top-level resource, which is what
/// makes generic helpers such as [`Entity::reference`], pagination and the mock
/// server possible without naming each resource individually.
pub trait Entity: TmfType + Sized {
    /// The resource identifier assigned by the server, if known.
    fn id(&self) -> Option<&str>;

    /// The canonical URI of this resource, if known.
    fn href(&self) -> Option<&str>;

    /// Builds a reference pointing at this entity.
    ///
    /// Copies across the `id` and `href`, so the reference resolves against
    /// whichever API owns the target — see
    /// [`ResolveRef`](crate::api::ResolveRef).
    ///
    /// Returns `None` when the entity has no `id`: TMF630 makes `id` mandatory
    /// on every `…Ref`, so an unsaved resource is not referenceable, and that
    /// is a state worth handling rather than panicking on.
    ///
    /// ```
    /// use rutmf::core::Entity;
    /// use rutmf::product::ProductOffering;
    ///
    /// let saved = ProductOffering::builder().id("7655").name("Firewall").build();
    /// assert_eq!(saved.reference().unwrap().id, "7655");
    ///
    /// let unsaved = ProductOffering::builder().name("Firewall").build();
    /// assert!(unsaved.reference().is_none());
    /// ```
    #[must_use]
    fn reference(&self) -> Option<Ref<Self>> {
        let mut reference = Ref::new(self.id()?);
        reference.href = self.href().map(ToOwned::to_owned);
        Some(reference)
    }
}
