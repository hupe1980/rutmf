//! The service domain: what runs on the network.
//!
//! Mirrors **TMF638 Service Inventory Management v5.0.0**. A [`Service`] sits
//! between what a customer bought and what the network is made of: a
//! [`Product`](crate::product::Product) is realised by services, and a service
//! is supported by [`Resource`]s.
//!
//! [`Resource`]: crate::resource::Resource
//!
//! # Two states, not one
//!
//! TMF638 splits status the way TMF639 does, though less extravagantly:
//!
//! - [`ServiceState`] — where the service is in its lifecycle, from
//!   `feasibilityChecked` through `active` to `terminated`
//! - [`ServiceOperatingStatus`] — what it is doing right now: `running`,
//!   `degraded`, `stopped`
//!
//! A service can be lifecycle-`active` and operating-`degraded` at the same
//! time, so the two are separate types and cannot be confused.
//!
//! # `state` is required on create
//!
//! Unusually, [`ServiceCreate`] requires both `state` and
//! `service_specification` — a service must be born somewhere in its lifecycle
//! and must say what it is an instance of. `operating_status` is *absent* from
//! the create schema entirely: the network decides that, not the client.

use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct, tmf_value};
use crate::core::{
    AnyEntity, Characteristic, Constraint, ExternalIdentifier, FeatureRelationshipType, Intent,
    ItemAction, Note, Ref, RelatedParty, RelatedPlace, ServiceSpecification, TimePeriod, Timestamp,
};
use crate::resource::Resource;
tmf_struct! {
    @name = "Service", @ref = "ServiceRef";
    /// An instance of a service specification, in the inventory.
    ///
    /// This is the **read model** of TMF638 Service Inventory. Use
    /// [`ServiceCreate`] for `POST` and [`ServiceUpdate`] for `PATCH`.
    ///
    /// The `supporting_service` member is the v5 `ServiceRefOrValue`, a `oneOf`
    /// over this type and a bare reference to one; the reference form is this
    /// type carrying only `id`/`href`.
    ///
    /// ```
    /// use rutmf::service::{Service, ServiceOperatingStatus, ServiceState};
    ///
    /// let json = r#"{
    ///   "id": "5351",
    ///   "name": "Broadband access",
    ///   "state": "active",
    ///   "operatingStatus": "degraded",
    ///   "@type": "Service"
    /// }"#;
    /// let svc: Service = serde_json::from_str(json).unwrap();
    ///
    /// // Alive in its lifecycle, unwell in its operation.
    /// assert_eq!(svc.state, Some(ServiceState::Active));
    /// assert_eq!(svc.operating_status, Some(ServiceOperatingStatus::Degraded));
    /// ```
    pub struct Service {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this service.
        href: String,
        /// Name the service is known by.
        name: String,
        /// Narrative description.
        description: String,
        /// Category the service falls in.
        category: String,
        /// Kind of service, from the provider's own vocabulary.
        service_type: String,
        /// Where the service is in its lifecycle.
        state: ServiceState,
        /// What the service is doing right now.
        operating_status: ServiceOperatingStatus,
        /// Whether the service is currently able to serve traffic.
        is_service_enabled: bool,
        /// Whether the service bundles others.
        is_bundle: bool,
        /// Whether the service retains state between invocations.
        is_stateful: bool,
        /// Whether the service has begun operating.
        has_started: bool,
        /// How the service is started, e.g. `automatic`, `manual`.
        start_mode: String,
        /// When the service started.
        start_date: Timestamp,
        /// When the service ended.
        end_date: Timestamp,
        /// Date the service is expected to be delivered.
        service_date: String,
        /// The specification this service instantiates — TMF633.
        service_specification: Ref<ServiceSpecification>,
        /// Configurable features of the service.
        feature: Vec<Feature>,
        /// Characteristics configuring the service.
        service_characteristic: Vec<Characteristic>,
        /// Relationships to other services.
        service_relationship: Vec<ServiceRelationship>,
        /// Services this one is built on — inline or by reference.
        supporting_service: Vec<Service>,
        /// Resources this service runs on — TMF639.
        supporting_resource: Vec<Ref<Resource>>,
        /// Entities of other kinds related to this service.
        related_entity: Vec<RelatedEntity>,
        /// Order lines that acted on this service — TMF641.
        service_order_item: Vec<RelatedServiceOrderItem>,
        /// Places the service is delivered to.
        place: Vec<RelatedPlace>,
        /// Parties related to the service.
        related_party: Vec<RelatedParty>,
        /// Identifiers for the service in external systems.
        external_identifier: Vec<ExternalIdentifier>,
        /// Intent governing the service — TMF921.
        intent: Ref<Intent>,
        /// Free-form notes.
        note: Vec<Note>,
        @renamed {
            /// The concrete class of the service, when this is the reference
            /// form rather than an inline description.
            "@referredType" referred_type: String,
        }
    }
}

tmf_entity!(Service);

tmf_struct! {
    @name = "Service";
    /// Body of a `POST /service` — the v5 `Service_FVO`.
    ///
    /// `state` and `serviceSpecification` are required; `operatingStatus` and
    /// `serviceDate` are absent, being the network's to report rather than the
    /// client's to assert.
    pub struct ServiceCreate {
        @required {
            /// Lifecycle state the service starts in. **Required on create.**
            state: ServiceState,
            /// The specification this service instantiates. **Required on create.**
            service_specification: Ref<ServiceSpecification>,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Name the service is known by.
        name: String,
        /// Narrative description.
        description: String,
        /// Category the service falls in.
        category: String,
        /// Kind of service.
        service_type: String,
        /// Whether the service is able to serve traffic.
        is_service_enabled: bool,
        /// Whether the service bundles others.
        is_bundle: bool,
        /// Whether the service retains state.
        is_stateful: bool,
        /// Whether the service has begun operating.
        has_started: bool,
        /// How the service is started.
        start_mode: String,
        /// When the service starts.
        start_date: Timestamp,
        /// When the service ends.
        end_date: Timestamp,
        /// Configurable features of the service.
        feature: Vec<Feature>,
        /// Characteristics configuring the service.
        service_characteristic: Vec<Characteristic>,
        /// Relationships to other services.
        service_relationship: Vec<ServiceRelationship>,
        /// Services this one is built on.
        supporting_service: Vec<Service>,
        /// Resources this service runs on.
        supporting_resource: Vec<Ref<Resource>>,
        /// Entities of other kinds related to this service.
        related_entity: Vec<RelatedEntity>,
        /// Order lines that acted on this service.
        service_order_item: Vec<RelatedServiceOrderItem>,
        /// Places the service is delivered to.
        place: Vec<RelatedPlace>,
        /// Parties related to the service.
        related_party: Vec<RelatedParty>,
        /// Identifiers for the service in external systems.
        external_identifier: Vec<ExternalIdentifier>,
        /// Intent governing the service.
        intent: Ref<Intent>,
        /// Free-form notes.
        note: Vec<Note>,
    }
}

tmf_struct! {
    @name = "Service";
    /// Body of a `PATCH /service/{id}` — the v5 `Service_MVO`.
    ///
    /// Nothing is required, and `operatingStatus` *is* present here even though
    /// the create schema omits it — a management system reporting a service as
    /// `degraded` does so by patching.
    pub struct ServiceUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New category.
        category: String,
        /// New service type.
        service_type: String,
        /// New lifecycle state.
        state: ServiceState,
        /// New operating status.
        operating_status: ServiceOperatingStatus,
        /// Whether the service is able to serve traffic.
        is_service_enabled: bool,
        /// Whether the service bundles others.
        is_bundle: bool,
        /// Whether the service retains state.
        is_stateful: bool,
        /// Whether the service has begun operating.
        has_started: bool,
        /// How the service is started.
        start_mode: String,
        /// New start date.
        start_date: Timestamp,
        /// New end date.
        end_date: Timestamp,
        /// Replacement specification reference.
        service_specification: Ref<ServiceSpecification>,
        /// Replacement feature list.
        feature: Vec<Feature>,
        /// Replacement characteristics.
        service_characteristic: Vec<Characteristic>,
        /// Replacement relationships.
        service_relationship: Vec<ServiceRelationship>,
        /// Replacement supporting services.
        supporting_service: Vec<Service>,
        /// Replacement supporting resources.
        supporting_resource: Vec<Ref<Resource>>,
        /// Replacement related entities.
        related_entity: Vec<RelatedEntity>,
        /// Replacement service order items.
        service_order_item: Vec<RelatedServiceOrderItem>,
        /// Replacement places.
        place: Vec<RelatedPlace>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
        /// Replacement intent.
        intent: Ref<Intent>,
        /// Replacement notes.
        note: Vec<Note>,
    }
}

tmf_patch_body!(ServiceUpdate);

tmf_struct! {
    @name = "Feature";
    /// A configurable capability of a service, switched on or off.
    ///
    /// # Why this is not `resource::Feature`
    ///
    /// TMF638 and TMF639 both declare a schema called `Feature`, and they are
    /// *not* the same: a service feature is constrained by a `ConstraintRef`,
    /// a resource feature by a `PolicyRef`. Modelling them as one type would
    /// let you set a member the server will silently drop, so this crate keeps
    /// them apart — see [`crate::resource::Feature`].
    pub struct Feature {
        /// Identifier of the feature within its service.
        id: String,
        /// Name the feature is known by.
        name: String,
        /// Whether the feature is currently switched on.
        is_enabled: bool,
        /// Whether the feature groups others.
        is_bundle: bool,
        /// Characteristics configuring the feature.
        feature_characteristic: Vec<Characteristic>,
        /// Relationships to other features.
        feature_relationship: Vec<FeatureRelationship>,
        /// Constraints on the feature.
        constraint: Vec<Ref<Constraint>>,
    }
}

tmf_struct! {
    @name = "ServiceRelationship";
    /// A typed link from one service to another.
    pub struct ServiceRelationship {
        /// Kind of relationship, e.g. `dependsOn`, `substitutes`.
        relationship_type: String,
        /// The service at the other end, inline or by reference.
        service: Service,
        /// Characteristics qualifying the relationship.
        service_relationship_characteristic: Vec<Characteristic>,
    }
}

tmf_struct! {
    @name = "RelatedEntityRefOrValue";
    /// An entity of some other kind, and the role it plays for this service.
    ///
    /// TMF638 leaves the target deliberately open — the v5 member is typed
    /// `EntityRefOrValue`, which says only that it is *some* TM Forum entity.
    /// `entity.referred_type` names which.
    pub struct RelatedEntity {
        /// Role the entity plays, e.g. `monitoredBy`.
        role: String,
        /// The entity itself, as a reference.
        entity: Ref<AnyEntity>,
    }
}

tmf_struct! {
    @name = "RelatedServiceOrderItem";
    /// A service order line that acted on this service — TMF641.
    pub struct RelatedServiceOrderItem {
        /// Identifier of the service order.
        service_order_id: String,
        /// URI of the service order.
        service_order_href: String,
        /// Identifier of the line within that order.
        item_id: String,
        /// What the line asked for.
        item_action: ItemAction,
        /// Role the order played for this service.
        role: String,
        @renamed {
            /// The concrete class of the referenced order.
            "@referredType" referred_type: String,
        }
    }
}

tmf_value! {
    /// A typed link between two features of the same service.
    ///
    /// # Why this is not `resource::FeatureRelationship`
    ///
    /// TMF638 declares this as a bare object: no `@type`, no `href`, nothing to
    /// dereference. TMF639's extends `EntityRef` and has all three. Same name,
    /// different schema; see [`crate::resource::FeatureRelationship`].
    pub struct FeatureRelationship {
        /// Identifier of the feature at the other end.
        id: String,
        /// Name of the feature at the other end.
        name: String,
        /// Kind of relationship.
        relationship_type: FeatureRelationshipType,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
    }
}

/// Where a service is in its lifecycle.
///
/// Distinct from [`ServiceOperatingStatus`], which says what the service is
/// doing rather than whether it exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ServiceState {
    /// Feasibility of providing the service has been checked.
    #[serde(rename = "feasibilityChecked")]
    FeasibilityChecked,
    /// The service has been designed but not reserved.
    #[serde(rename = "designed")]
    Designed,
    /// Capacity for the service has been reserved.
    #[serde(rename = "reserved")]
    Reserved,
    /// Provisioned but not carrying traffic.
    #[serde(rename = "inactive")]
    Inactive,
    /// Provisioned and carrying traffic.
    #[serde(rename = "active")]
    Active,
    /// Temporarily withdrawn, and restorable.
    #[serde(rename = "suspended")]
    Suspended,
    /// Ceased, and not restorable.
    #[serde(rename = "terminated")]
    Terminated,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl ServiceState {
    /// Whether the service has reached a state it will not leave.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminated)
    }
}

/// What a service is doing right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ServiceOperatingStatus {
    /// Not yet configured.
    #[serde(rename = "pending")]
    Pending,
    /// Configured but not started.
    #[serde(rename = "configured")]
    Configured,
    /// Starting up.
    #[serde(rename = "starting")]
    Starting,
    /// Running normally.
    #[serde(rename = "running")]
    Running,
    /// Running below its specified capability.
    #[serde(rename = "degraded")]
    Degraded,
    /// Not running because of a fault.
    #[serde(rename = "failed")]
    Failed,
    /// Running with reduced function by design.
    #[serde(rename = "limited")]
    Limited,
    /// Shutting down.
    #[serde(rename = "stopping")]
    Stopping,
    /// Not running.
    #[serde(rename = "stopped")]
    Stopped,
    /// Could not be determined.
    #[serde(rename = "unknown")]
    Unknown,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_and_operation_are_separate_dimensions() {
        let svc: Service =
            serde_json::from_str(r#"{"state":"active","operatingStatus":"degraded"}"#).unwrap();
        assert_eq!(svc.state, Some(ServiceState::Active));
        assert_eq!(svc.operating_status, Some(ServiceOperatingStatus::Degraded));
        assert!(!svc.state.unwrap().is_terminal());
    }

    #[test]
    fn a_supporting_service_may_be_a_bare_reference() {
        let svc: Service = serde_json::from_str(
            r#"{"id":"1","supportingService":[{"id":"2","@type":"ServiceRef"}]}"#,
        )
        .unwrap();
        let supporting = &svc.supporting_service.as_ref().unwrap()[0];
        assert_eq!(supporting.id.as_deref(), Some("2"));
        assert_eq!(supporting.type_name(), "ServiceRef");
    }

    #[test]
    fn the_create_body_demands_a_state_and_a_specification() {
        let body = ServiceCreate::builder()
            .state(ServiceState::Reserved)
            .service_specification(Ref::<ServiceSpecification>::new("SS-1"))
            .name("Broadband access")
            .build();
        assert_eq!(body.state, ServiceState::Reserved);
        assert_eq!(body.service_specification.id, "SS-1");
    }
}
