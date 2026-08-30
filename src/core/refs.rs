//! Marker types for entities that live in *another* TM Forum API.
//!
//! See the [`core` module documentation](crate::core#reference-target-markers)
//! for what they are for and when one goes away.

use super::extensible::TmfType;

/// Declares a reference target: the entity type name and its `…Ref` name.
macro_rules! ref_target {
    ($(#[$meta:meta])* $name:ident, $ty:literal, $ref_ty:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name;

        impl TmfType for $name {
            const TYPE_NAME: &'static str = $ty;
            const REF_TYPE_NAME: &'static str = $ref_ty;
        }
    };
}

ref_target!(
    /// A sales channel through which an offering is available.
    Channel, "Channel", "ChannelRef"
);
ref_target!(
    /// A geographic place — TMF673 / TMF674.
    Place, "Place", "PlaceRef"
);
ref_target!(
    /// A geographic address — TMF673.
    GeographicAddress, "GeographicAddress", "GeographicAddressRef"
);
ref_target!(
    /// A market segment targeted by an offering.
    MarketSegment, "MarketSegment", "MarketSegmentRef"
);
ref_target!(
    /// A policy constraining an entity.
    Policy, "Policy", "PolicyRef"
);
ref_target!(
    /// A service level agreement — TMF623.
    ServiceLevelAgreement, "ServiceLevelAgreement", "SLARef"
);
ref_target!(
    /// An agreement — TMF651.
    Agreement, "Agreement", "AgreementRef"
);
ref_target!(
    /// A payment method — TMF670.
    PaymentMethod, "PaymentMethod", "PaymentMethodRef"
);
ref_target!(
    /// A service candidate in a service catalog — TMF633.
    ServiceCandidate, "ServiceCandidate", "ServiceCandidateRef"
);
ref_target!(
    /// A service specification — TMF633.
    ServiceSpecification, "ServiceSpecification", "ServiceSpecificationRef"
);
ref_target!(
    /// An intent specification — TMF921.
    IntentSpecification, "IntentSpecification", "IntentSpecificationRef"
);
ref_target!(
    /// A connection point of a resource function — TMF634 references it, and
    /// no vendored specification defines the entity itself.
    ConnectionPointSpecification, "ConnectionPointSpecification", "ConnectionPointSpecificationRef"
);
ref_target!(
    /// An endpoint joined by a connection specification — TMF634.
    EndpointSpecification, "EndpointSpecification", "EndpointSpecificationRef"
);
ref_target!(
    /// A connection point of a resource function: the service access point
    /// where its inputs and outputs are available.
    ///
    /// The instance counterpart of [`ConnectionPointSpecification`]. TMF639
    /// references it from `ResourceFunction.connectionPoint` and from an
    /// [`Endpoint`](crate::resource::Endpoint), and no vendored specification
    /// defines the entity itself.
    ConnectionPoint, "ConnectionPoint", "ConnectionPointRef"
);
ref_target!(
    /// A schedule a resource function runs to — TMF645.
    Schedule, "Schedule", "ScheduleRef"
);
ref_target!(
    /// Whatever an alarm is raised against — TMF642 leaves the class open, so
    /// the reference carries `@referredType` rather than a fixed target.
    AlarmedObject, "AlarmedObject", "AlarmedObjectRef"
);
ref_target!(
    /// A performance threshold whose crossing raised an alarm — TMF628.
    Threshold, "Threshold", "ThresholdRef"
);
ref_target!(
    /// A billing account, as *referenced* — TMF678 uses `BillingAccountRef`,
    /// which TMF666 does not define even though it declares the entity.
    ///
    /// The entity itself is [`account::Account`](crate::account::Account) with
    /// `AccountKind::Billing`; this marker exists because a `Ref<T>` binds one
    /// target type to one `…Ref` class name, and the subclasses have their own.
    BillingAccount, "BillingAccount", "BillingAccountRef"
);
ref_target!(
    /// A financial account money is posted to, as *referenced* — TMF666
    /// declares `FinancialAccountRef` beside the entity.
    ///
    /// See [`BillingAccount`] for why a modelled entity still needs a marker.
    FinancialAccount, "FinancialAccount", "FinancialAccountRef"
);
ref_target!(
    /// The specification a billing cycle runs to — TMF678 references it and no
    /// vendored specification defines the entity.
    BillCycleSpecification, "BillCycleSpecification", "BillCycleSpecificationRef"
);
ref_target!(
    /// A payment — TMF676.
    Payment, "Payment", "PaymentRef"
);
ref_target!(
    /// A quote — TMF648.
    Quote, "Quote", "QuoteRef"
);
ref_target!(
    /// An appointment — TMF646.
    Appointment, "Appointment", "AppointmentRef"
);
ref_target!(
    /// A policy or rule constraining an entity — TMF632.
    Constraint, "Constraint", "ConstraintRef"
);
ref_target!(
    /// An entity of unspecified kind.
    ///
    /// The v5 `EntityRef`, used where a schema declines to say what it points
    /// at: TMF638's `relatedEntity` is *some* TM Forum entity, and an event's
    /// `source` is *some* system. In both, `referred_type` names which.
    AnyEntity, "Entity", "EntityRef"
);
ref_target!(
    /// An intent — TMF921.
    Intent, "Intent", "IntentRef"
);
ref_target!(
    /// A specification for an agreement — TMF651.
    AgreementSpecification, "AgreementSpecification", "AgreementSpecificationRef"
);
ref_target!(
    /// A set of permissions granted to a party role — TMF672.
    PermissionSpecificationSet, "PermissionSpecificationSet", "PermissionSpecificationSetRef"
);
ref_target!(
    /// The specification of an association between two entity specifications.
    AssociationSpecification, "AssociationSpecification", "AssociationSpecificationRef"
);
ref_target!(
    /// An account — TMF666. The base class a bare `AccountRef` names.
    Account, "Account", "AccountRef"
);
ref_target!(
    /// A promotion applied to an offering — TMF671.
    Promotion, "Promotion", "PromotionRef"
);
ref_target!(
    /// A product offering qualification — TMF679.
    ///
    /// This one stays a marker permanently. TMF622 references
    /// `ProductOfferingQualificationRef`, but TMF679 defines no resource of
    /// that name: it serves `CheckProductOfferingQualification` and
    /// `QueryProductOfferingQualification`, and a bare reference does not say
    /// which. Pointing it at either would guess.
    ProductOfferingQualification, "ProductOfferingQualification", "ProductOfferingQualificationRef"
);
