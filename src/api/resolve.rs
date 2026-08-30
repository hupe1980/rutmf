//! Following a typed reference to the resource it points at.
//!
//! TMF payloads are graphs stitched together with `…Ref` objects. Because
//! [`Ref<T>`] carries its target in the type system, resolving one can hand
//! back a `T` without the caller naming the type or the collection path.

use serde::de::DeserializeOwned;

use crate::core::{Ref, TmfType};

use super::client::TmfClient;
use super::error::Result;
use super::query::Query;

/// A resource that a [`Ref`] can be resolved into.
///
/// The collection path is a property of the resource type, so implementing
/// this once per resource is what lets [`Ref::resolve`] work generically.
///
/// [`Ref::resolve`]: ResolveRef::resolve
pub trait Resolvable: TmfType {
    /// What a reference of this class fetches into — usually `Self`.
    ///
    /// It differs where TM Forum gives a **subclass its own reference class but
    /// no collection of its own**. `BillingAccountRef` and `FinancialAccountRef`
    /// name subclasses of `Account`; there is no `/billingAccount/{id}` that
    /// returns a distinct type, and fetching either yields an `Account`.
    ///
    /// Tying the fetched type to the reference class would leave those
    /// references typed, correct and permanently unresolvable — the crate models
    /// `Account`, and a `Ref<BillingAccount>` still could not reach it. The same
    /// applies to `PartyRole`, whose reference is reachable from the
    /// `relatedParty` of nearly every resource in the crate.
    type Output: DeserializeOwned;

    /// The collection segment this resource lives under, e.g. `productOffering`.
    const COLLECTION: &'static str;
}

/// Resolving a reference through a client.
#[allow(async_fn_in_trait, reason = "used directly, not as a trait object")]
pub trait ResolveRef<T> {
    /// Fetches the referenced resource.
    ///
    /// Uses the reference's `href` when the server supplied one, so a
    /// cross-API reference resolves against the API that owns it; otherwise
    /// falls back to `{base_url}/{collection}/{id}`.
    ///
    /// ```no_run
    /// # async fn demo(client: &rutmf::api::TmfClient, offering: rutmf::product::ProductOffering)
    /// # -> rutmf::api::Result<()> {
    /// use rutmf::api::{Query, ResolveRef};
    ///
    /// if let Some(reference) = &offering.product_specification {
    ///     // Typed as `ProductSpecification` — no turbofish, no path string.
    ///     let spec = reference.resolve(client, &Query::new()).await?;
    ///     println!("{:?}", spec.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::CrossOrigin`] when the `href` leaves the client's own
    /// origin. An `href` is *payload data*: it is written by the server, and in
    /// a telco integration it has usually passed through several systems on the
    /// way. The transport attaches this client's credentials to whatever URL it
    /// is handed, so a reference naming another host would send a live bearer
    /// token there — which makes any `…Ref` in any response a place to put one.
    ///
    /// Within a deployment the APIs share a host and differ only by path, so
    /// this refuses nothing that ordinarily happens. Federation across hosts is
    /// still available through
    /// [`resolve_cross_origin`](Self::resolve_cross_origin), which is the same
    /// call with the guard lifted and the decision named.
    ///
    /// [`Error::CrossOrigin`]: crate::api::Error::CrossOrigin
    async fn resolve(&self, client: &TmfClient, query: &Query) -> Result<T>;

    /// Fetches the referenced resource, **following the `href` to any origin**.
    ///
    /// The unguarded counterpart of [`resolve`](Self::resolve), for a
    /// deployment that federates across hosts and trusts the server whose
    /// payload this reference came out of to name only hosts you would
    /// authenticate against.
    ///
    /// If that trust is not something you can state plainly about the server on
    /// the other end, use [`resolve`](Self::resolve).
    async fn resolve_cross_origin(&self, client: &TmfClient, query: &Query) -> Result<T>;
}

impl<T: Resolvable> ResolveRef<T::Output> for Ref<T> {
    async fn resolve(&self, client: &TmfClient, query: &Query) -> Result<T::Output> {
        match self.absolute_href() {
            Some(href) => client.get_absolute(href, query).await,
            None => client.get(T::COLLECTION, &self.id, query).await,
        }
    }

    async fn resolve_cross_origin(&self, client: &TmfClient, query: &Query) -> Result<T::Output> {
        match self.absolute_href() {
            Some(href) => client.get_cross_origin(href, query).await,
            None => client.get(T::COLLECTION, &self.id, query).await,
        }
    }
}

/// Declares where a resource lives, so references to it can be resolved.
///
/// Every type implementing [`Entity`](crate::core::Entity) should appear here,
/// so the invariant "an addressable resource can be resolved" holds.
#[allow(
    unused_macros,
    reason = "an `api`-only build enables no domain to resolve into"
)]
macro_rules! resolvable {
    ($($ty:path => $collection:literal),* $(,)?) => {
        $(impl Resolvable for $ty {
            type Output = Self;
            const COLLECTION: &'static str = $collection;
        })*
    };
}

/// Declares a reference class that fetches into a *different* type.
///
/// For the subclass references TM Forum gives no collection of their own — see
/// [`Resolvable::Output`]. Keeping this a separate macro means the ordinary case
/// stays a one-liner and the exceptions are visible as exceptions.
#[allow(
    unused_macros,
    reason = "an `api`-only build enables no domain to resolve into"
)]
macro_rules! resolvable_into {
    ($($ty:path => $collection:literal as $out:path),* $(,)?) => {
        $(impl Resolvable for $ty {
            type Output = $out;
            const COLLECTION: &'static str = $collection;
        })*
    };
}

#[cfg(feature = "product")]
resolvable! {
    crate::product::ProductOffering => "productOffering",
    crate::product::ProductSpecification => "productSpecification",
    crate::product::ProductOfferingPrice => "productOfferingPrice",
    crate::product::ProductCatalog => "productCatalog",
    crate::product::Category => "category",
    crate::product::ImportJob => "importJob",
    crate::product::ExportJob => "exportJob",
    // TMF637: the same `Product` an order line acts on.
    crate::product::Product => "product",
    // TMF679, the eligibility step between catalog and order.
    crate::product::CheckProductOfferingQualification => "checkProductOfferingQualification",
    crate::product::QueryProductOfferingQualification => "queryProductOfferingQualification",
}

#[cfg(feature = "service")]
resolvable! {
    crate::service::Service => "service",
}

#[cfg(feature = "resource")]
resolvable! {
    crate::resource::Resource => "resource",
    // TMF634, the catalog half of the resource domain.
    crate::resource::ResourceCatalog => "resourceCatalog",
    crate::resource::ResourceCategory => "resourceCategory",
    crate::resource::ResourceCandidate => "resourceCandidate",
    crate::resource::ResourceSpecification => "resourceSpecification",
}

#[cfg(feature = "party")]
resolvable! {
    crate::party::Individual => "individual",
    crate::party::Organization => "organization",
    // TMF669, the general case of TMF629's customer.
    crate::party::PartyRole => "partyRole",
    crate::party::PartyRoleSpecification => "partyRoleSpecification",
}

#[cfg(feature = "party")]
resolvable_into! {
    // `PartyOrPartyRole::Role` names this marker, because `core` cannot depend
    // on a domain feature. Resolving it hands back the real TMF669 entity, so
    // the `relatedParty` of any resource in the crate can be followed.
    crate::core::PartyRole => "partyRole" as crate::party::PartyRole,
}

#[cfg(feature = "order")]
resolvable! {
    crate::order::ProductOrder => "productOrder",
    crate::order::CancelProductOrder => "cancelProductOrder",
}

#[cfg(feature = "account")]
resolvable_into! {
    // TMF666 exposes four collections that all serve `Account`, and gives two of
    // its subclasses their own reference class. A `BillingAccountRef` is fetched
    // from `/billingAccount` and comes back as an `Account` carrying
    // `@type: BillingAccount`; `AccountKind` recovers which subclass it is.
    crate::core::BillingAccount => "billingAccount" as crate::account::Account,
    crate::core::FinancialAccount => "financialAccount" as crate::account::Account,
}

#[cfg(feature = "account")]
resolvable! {
    // The four account collections all serve `Account`; `partyAccount` is the
    // one a bare `AccountRef` resolves against.
    crate::account::Account => "partyAccount",
    crate::account::BillFormat => "billFormat",
    crate::account::BillPresentationMedia => "billPresentationMedia",
    crate::account::BillingCycleSpecification => "billingCycleSpecification",
}

#[cfg(feature = "bill")]
resolvable! {
    crate::bill::CustomerBill => "customerBill",
    crate::bill::CustomerBillOnDemand => "customerBillOnDemand",
    crate::bill::AppliedCustomerBillingRate => "appliedCustomerBillingRate",
    crate::bill::BillCycle => "billCycle",
}

#[cfg(feature = "alarm")]
resolvable! {
    crate::alarm::Alarm => "alarm",
    crate::alarm::AckAlarm => "ackAlarm",
    crate::alarm::UnAckAlarm => "unAckAlarm",
    crate::alarm::ClearAlarm => "clearAlarm",
    crate::alarm::CommentAlarm => "commentAlarm",
    crate::alarm::GroupAlarm => "groupAlarm",
    crate::alarm::UnGroupAlarm => "unGroupAlarm",
}

#[cfg(feature = "ticket")]
resolvable! {
    crate::ticket::TroubleTicket => "troubleTicket",
    crate::ticket::TroubleTicketSpecification => "troubleTicketSpecification",
}

#[cfg(feature = "customer")]
resolvable! {
    crate::customer::Customer => "customer",
}
