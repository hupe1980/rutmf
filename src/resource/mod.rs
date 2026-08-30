//! The resource domain: what the network is made of.
//!
//! Mirrors **TMF639 Resource Inventory Management v5.0.0**. A [`Resource`] is a
//! physical or logical component of the infrastructure — a port, a SIM, a
//! virtual machine, a licence. Services run on resources, and products are
//! realised by services, which is the chain
//! [`Product`](crate::product::Product) → [`Service`](crate::service::Service)
//! → [`Resource`] this crate models end to end.
//!
//! # Nine status dimensions, not one
//!
//! Where most TM Forum resources have a single `state`, TMF639 inherits the
//! TMN/ITU-T X.731 state model and splits status across nine independent
//! members: [`ResourceOperationalState`], [`ResourceUsageState`],
//! [`ResourceAdministrativeState`], [`ResourceLifecycleState`],
//! [`ResourceAlarmStatus`], [`ResourceProceduralStatus`],
//! [`ResourceAvailabilityStatus`], [`ResourceControlStatus`] and
//! `allocation_status`. They are orthogonal: a resource can be
//! administratively `locked`, operationally `enabled` and availability
//! `degraded` at the same time. Each is its own type, so none can be assigned
//! to another.
//!
//! `allocationStatus` is the odd one out — TMF639 describes its values
//! (`available`, `reserved`, `allocated`, `partiallyAllocated`) in prose but
//! declares no enumeration, so it stays a [`String`].
//!
//! # One type for four subclasses
//!
//! TMF639 sub-types `Resource` twice over: `LogicalResource` and
//! `PhysicalResource` extend it, and `ResourceFunction` and `SoftwareResource`
//! extend `LogicalResource` in turn. Rather than four near-identical structs,
//! [`Resource`] carries the union of their members and [`ResourceKind`] recovers
//! which one a server sent — the same call [`ResourceSpecification`] makes on
//! the catalog side, so the two halves of the domain read alike.
//!
//! A server that sends a vendor subclass parses as [`ResourceKind::Other`]
//! rather than failing.
//!
//! # What is wired to what
//!
//! A `ResourceFunction`'s internal connectivity is a [`ResourceGraph`]:
//! [`Connection`] edges over [`Endpoint`] vertices. That is the instance
//! counterpart of [`ResourceGraphSpecification`] — the specification says what
//! *may* be wired together, the graph says what *is*.
//!
//! # There is no patch body
//!
//! TMF639 declares no `Resource_MVO`. Its `PATCH /resource/{id}` takes the
//! plain `Resource` schema, so [`Resource`] is *itself* the patch body — the
//! only resource in this crate where the read model and the update model are
//! one type. See [`ResourceUpdate`], which is an alias rather than a struct.

use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    Attachment, Characteristic, ConnectionPoint, ExternalIdentifier, Intent, ItemAction, Note,
    Place, Ref, RelatedParty, Schedule, TimePeriod, Timestamp,
};

mod catalog;
mod feature;
mod specification;
mod topology;

pub use catalog::{
    ResourceCandidate, ResourceCandidateCreate, ResourceCandidateUpdate, ResourceCatalog,
    ResourceCatalogCreate, ResourceCatalogUpdate, ResourceCategory, ResourceCategoryCreate,
    ResourceCategoryUpdate,
};
pub use feature::{Feature, FeatureRelationship};
pub use specification::{
    ConnectionAssociationType, ConnectionSpecification, FeatureSpecification,
    FeatureSpecificationRelationship, ResourceGraphSpecification,
    ResourceGraphSpecificationRelationship, ResourceSpecification, ResourceSpecificationCreate,
    ResourceSpecificationKind, ResourceSpecificationRelationship, ResourceSpecificationUpdate,
    TargetResourceSchema,
};
pub use topology::{Connection, Endpoint, ResourceGraph, ResourceGraphRelationship};

tmf_struct! {
    @name = "Resource", @ref = "ResourceRef";
    /// A physical or logical component of the infrastructure.
    ///
    /// This is the **read model** of TMF639 Resource Inventory. Use
    /// [`ResourceCreate`] for `POST`, and [`Resource`] itself for `PATCH` —
    /// see the [module docs](self) for why.
    ///
    /// The `supporting_resource` member is the v5 `ResourceRefOrValue`, a
    /// `oneOf` over this type and a bare reference to one; the reference form
    /// is this type carrying only `id`/`href`.
    ///
    /// ```
    /// use rutmf::resource::{Resource, ResourceAdministrativeState, ResourceOperationalState};
    ///
    /// let json = r#"{
    ///   "id": "3472",
    ///   "name": "Optical port 3/1/2",
    ///   "operationalState": "enabled",
    ///   "administrativeState": "locked",
    ///   "@type": "Resource"
    /// }"#;
    /// let port: Resource = serde_json::from_str(json).unwrap();
    ///
    /// // The dimensions are independent, and typed apart.
    /// assert_eq!(port.operational_state, Some(ResourceOperationalState::Enabled));
    /// assert_eq!(port.administrative_state, Some(ResourceAdministrativeState::Locked));
    /// ```
    pub struct Resource {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this resource.
        href: String,
        /// Name the resource is known by.
        name: String,
        /// Narrative description.
        description: String,
        /// Category the resource falls in.
        category: String,
        /// Version of the resource, distinct from its specification's version.
        resource_version: String,
        /// When the resource record was created.
        creation_date: Timestamp,
        /// When the resource started operating.
        start_operating_date: Timestamp,
        /// When the resource stopped operating.
        end_operating_date: Timestamp,
        /// Period during which the resource record is valid.
        valid_for: TimePeriod,
        /// Whether the resource is able to perform its function.
        operational_state: ResourceOperationalState,
        /// Whether the resource is in use.
        usage_state: ResourceUsageState,
        /// Whether an operator has permitted the resource to be used.
        administrative_state: ResourceAdministrativeState,
        /// Where the resource is in its plan-install-remove lifecycle.
        lifecycle_state: ResourceLifecycleState,
        /// Severities of any outstanding alarms — TMF639 permits several.
        alarm_status: Vec<ResourceAlarmStatus>,
        /// Where the resource is in its initialisation procedure.
        procedural_status: ResourceProceduralStatus,
        /// Why the resource is unavailable, when it is.
        availability_status: ResourceAvailabilityStatus,
        /// Operator-imposed restriction on the resource.
        control_status: ResourceControlStatus,
        /// How much of the resource is spoken for — `available`, `reserved`,
        /// `allocated`, `partiallyAllocated`. TMF639 names these values in
        /// prose but declares no enumeration, so this is a free string.
        allocation_status: String,
        /// Whether the status of the resource could not be determined.
        unknown_status: bool,
        /// The specification this resource conforms to.
        resource_specification: Ref<ResourceSpecification>,
        /// Configurable features of the resource.
        activation_feature: Vec<Feature>,
        /// Characteristics of the resource.
        resource_characteristic: Vec<Characteristic>,
        /// Relationships to other resources.
        resource_relationship: Vec<ResourceRelationship>,
        /// Resources this one is built on — inline or by reference.
        supporting_resource: Vec<Resource>,
        /// Order lines that acted on this resource — TMF652.
        resource_order_item: Vec<RelatedResourceOrderItem>,
        /// Places the resource is installed at.
        place: Vec<RelatedPlace>,
        /// Parties related to the resource.
        related_party: Vec<RelatedParty>,
        /// Identifiers for the resource in external systems.
        external_identifier: Vec<ExternalIdentifier>,
        /// Documents attached to the resource.
        attachment: Vec<Attachment>,
        /// Intent governing the resource — TMF921.
        intent: Ref<Intent>,
        /// Free-form notes.
        note: Vec<Note>,

        /// The value the resource carries — `LogicalResource` and below.
        ///
        /// A logical resource *is* its value: an MSISDN, an IP address, a TPE.
        value: String,

        /// When the item was manufactured — `PhysicalResource`.
        manufacture_date: Timestamp,
        /// Whether the item is serving or standing by — `PhysicalResource`.
        standby_status: ResourceStandbyStatus,
        /// Current power status of the hardware — `PhysicalResource`.
        ///
        /// TMF639 describes this member's values in prose without declaring an
        /// enumeration, so it stays a [`String`] — the same call
        /// `allocation_status` gets, and for the same reason.
        power_state: String,
        /// The item's power-consumption policy — `PhysicalResource`.
        power_consuming_state: ResourcePowerConsumingState,
        /// How much power the item is drawing — `PhysicalResource`.
        power_consuming_level: i64,
        /// Manufacturer's serial number — `PhysicalResource`.
        serial_number: String,
        /// Manufacturing batch — `PhysicalResource`.
        batch_number: String,
        /// Hardware version — `PhysicalResource`.
        version_number: String,

        /// Service access points for the function's inputs and outputs —
        /// `ResourceFunction`.
        connection_point: Vec<Ref<ConnectionPoint>>,
        /// Internal wiring of the contained functions — `ResourceFunction`.
        connectivity: Vec<ResourceGraph>,
        /// Ranking against other functions — `ResourceFunction`.
        priority: i64,
        /// The part the function plays — `ResourceFunction`.
        role: String,
        /// What the function does — `ResourceFunction`.
        function_type: String,
        /// Characteristics the function may change by itself —
        /// `ResourceFunction`.
        auto_modification: Vec<Characteristic>,
        /// Schedules the function runs to — `ResourceFunction`.
        schedule: Vec<Ref<Schedule>>,

        /// When the software resource was last changed — `SoftwareResource`.
        last_update: Timestamp,
        /// Whether the software is currently spread across several nodes —
        /// `SoftwareResource`.
        is_distributed_current: bool,
        /// The platform the software is deployed on — `SoftwareResource`.
        target_platform: String,
        @renamed {
            /// The concrete class of the resource, when this is the reference
            /// form of `ResourceRefOrValue`.
            "@referredType" referred_type: String,
        }
    }
}

impl Resource {
    /// Recovers the subclass implied by `@type`.
    ///
    /// ```
    /// use rutmf::resource::{Resource, ResourceKind};
    ///
    /// let json = r#"{"@type":"PhysicalResource","serialNumber":"SN-1","batchNumber":"B7"}"#;
    /// let resource: Resource = serde_json::from_str(json).unwrap();
    ///
    /// assert_eq!(resource.kind(), ResourceKind::Physical);
    /// assert_eq!(resource.serial_number.as_deref(), Some("SN-1"));
    /// ```
    #[must_use]
    pub fn kind(&self) -> ResourceKind {
        ResourceKind::from_type_name(self.type_name())
    }
}

/// The subclass of a [`Resource`], recovered from its `@type`.
///
/// TMF639 sub-types `Resource` twice over: `LogicalResource` and
/// `PhysicalResource` extend it, and `ResourceFunction` and `SoftwareResource`
/// extend `LogicalResource` in turn. Rather than four near-identical Rust
/// structs, [`Resource`] carries the union of their members and this recovers
/// which one a server sent — the same call
/// [`ResourceSpecificationKind`] makes on the catalog side, so the instance and
/// specification halves of the domain read alike.
///
/// [`Other`](Self::Other) means a vendor subclass this crate does not know,
/// which parses rather than failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResourceKind {
    /// The abstract base, carrying no subclass-specific member.
    Base,
    /// A logical resource: adds `value`.
    Logical,
    /// A physical resource: serial and batch numbers, power and standby status.
    Physical,
    /// A network function: connection points, internal connectivity, schedule.
    ///
    /// A `LogicalResource` in the schema hierarchy, so it carries `value` too.
    Function,
    /// A software resource: target platform and distribution state.
    ///
    /// Also a `LogicalResource`, and likewise carries `value`.
    Software,
    /// A subclass this crate does not know.
    Other,
}

impl ResourceKind {
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
            Self::Logical,
            Self::Physical,
            Self::Function,
            Self::Software,
        ]
    }

    /// Maps a `@type` value to its kind; unknown names become [`Self::Other`].
    #[must_use]
    pub fn from_type_name(name: &str) -> Self {
        match name {
            "Resource" => Self::Base,
            "LogicalResource" => Self::Logical,
            "PhysicalResource" => Self::Physical,
            "ResourceFunction" => Self::Function,
            "SoftwareResource" => Self::Software,
            _ => Self::Other,
        }
    }

    /// The canonical `@type` for this kind.
    ///
    /// [`Self::Other`] has no canonical name and maps to the abstract base.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Base | Self::Other => "Resource",
            Self::Logical => "LogicalResource",
            Self::Physical => "PhysicalResource",
            Self::Function => "ResourceFunction",
            Self::Software => "SoftwareResource",
        }
    }

    /// Whether this kind is a `LogicalResource` or one of its subclasses.
    ///
    /// TMF639's hierarchy is two levels deep, so "is this logical" is not the
    /// same question as "is `@type` exactly `LogicalResource`".
    #[must_use]
    pub fn is_logical(self) -> bool {
        matches!(self, Self::Logical | Self::Function | Self::Software)
    }
}

/// Whether a physical resource is serving or standing by (ITU-T X.731).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceStandbyStatus {
    /// Standing by, ready to take over immediately.
    #[serde(rename = "hotStandby")]
    HotStandby,
    /// Standing by, and needs initialising before it can take over.
    #[serde(rename = "coldStandby")]
    ColdStandby,
    /// Currently doing the work.
    #[serde(rename = "providingService")]
    ProvidingService,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// The power-consumption policy of a physical resource (ITU-T M.3701).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourcePowerConsumingState {
    /// Drawing full power.
    #[serde(rename = "fullPower")]
    FullPower,
    /// Drawing reduced power.
    #[serde(rename = "powerSaving")]
    PowerSaving,
    /// Suspended, and wakeable.
    #[serde(rename = "sleeping")]
    Sleeping,
    /// Powered down.
    #[serde(rename = "shutdown")]
    Shutdown,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

tmf_entity!(Resource);
tmf_patch_body!(Resource);

/// Body of a `PATCH /resource/{id}`.
///
/// TMF639 declares no `Resource_MVO`: its patch operation takes the plain
/// `Resource` schema. This alias exists so the naming stays uniform across the
/// crate, and to make the absence deliberate rather than an oversight — but it
/// really is the same type, and `id` and `href` really are on it.
pub type ResourceUpdate = Resource;

tmf_struct! {
    @name = "Resource";
    /// Body of a `POST /resource` — the v5 `Resource_FVO`.
    ///
    /// Unusually, TMF639's create schema retains every member of the read
    /// model, `creationDate` included. Nothing is required.
    pub struct ResourceCreate {
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Client-supplied URI, where the server permits one.
        href: String,
        /// Name the resource is known by.
        name: String,
        /// Narrative description.
        description: String,
        /// Category the resource falls in.
        category: String,
        /// Version of the resource.
        resource_version: String,
        /// Creation timestamp, which this API unusually accepts on create.
        creation_date: Timestamp,
        /// When the resource started operating.
        start_operating_date: Timestamp,
        /// When the resource stopped operating.
        end_operating_date: Timestamp,
        /// Period during which the resource record is valid.
        valid_for: TimePeriod,
        /// Initial operational state.
        operational_state: ResourceOperationalState,
        /// Initial usage state.
        usage_state: ResourceUsageState,
        /// Initial administrative state.
        administrative_state: ResourceAdministrativeState,
        /// Initial lifecycle state.
        lifecycle_state: ResourceLifecycleState,
        /// Initial alarm statuses.
        alarm_status: Vec<ResourceAlarmStatus>,
        /// Initial procedural status.
        procedural_status: ResourceProceduralStatus,
        /// Initial availability status.
        availability_status: ResourceAvailabilityStatus,
        /// Initial control status.
        control_status: ResourceControlStatus,
        /// Initial allocation status.
        allocation_status: String,
        /// Whether the status of the resource is unknown.
        unknown_status: bool,
        /// The specification this resource conforms to.
        resource_specification: Ref<ResourceSpecification>,
        /// Configurable features of the resource.
        activation_feature: Vec<Feature>,
        /// Characteristics of the resource.
        resource_characteristic: Vec<Characteristic>,
        /// Relationships to other resources.
        resource_relationship: Vec<ResourceRelationship>,
        /// Resources this one is built on.
        supporting_resource: Vec<Resource>,
        /// Order lines that acted on this resource.
        resource_order_item: Vec<RelatedResourceOrderItem>,
        /// Places the resource is installed at.
        place: Vec<RelatedPlace>,
        /// Parties related to the resource.
        related_party: Vec<RelatedParty>,
        /// Identifiers for the resource in external systems.
        external_identifier: Vec<ExternalIdentifier>,
        /// Documents attached to the resource.
        attachment: Vec<Attachment>,
        /// Intent governing the resource.
        intent: Ref<Intent>,
        /// Free-form notes.
        note: Vec<Note>,
    }
}

tmf_struct! {
    @name = "ResourceRelationship";
    /// A typed link from one resource to another.
    pub struct ResourceRelationship {
        /// Kind of relationship, e.g. `contains`, `dependsOn`.
        relationship_type: String,
        /// The resource at the other end.
        resource: Ref<Resource>,
        /// Characteristics qualifying the relationship.
        resource_relationship_characteristic: Vec<Characteristic>,
    }
}

tmf_struct! {
    @name = "RelatedResourceOrderItem";
    /// A resource order line that acted on this resource — TMF652.
    pub struct RelatedResourceOrderItem {
        /// Identifier of the resource order.
        resource_order_id: String,
        /// URI of the resource order.
        resource_order_href: String,
        /// Identifier of the line within that order.
        item_id: String,
        /// What the line asked for.
        item_action: ItemAction,
        /// Role the order played for this resource.
        role: String,
        @renamed {
            /// The concrete class of the referenced order.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "RelatedPlaceRef";
    /// A place a resource is installed at, with the role it plays.
    ///
    /// Unlike TMF622 and TMF637, which permit an inline place, TMF639 accepts
    /// only a reference — the member is `RelatedPlaceRef`, not
    /// `RelatedPlaceRefOrValue`.
    pub struct RelatedPlace {
        /// Role the place plays, e.g. `installationSite`.
        role: String,
        /// The place itself.
        place: Ref<Place>,
    }
}

/// Whether a resource is able to perform its function.
///
/// The X.731 *operational state*: a statement about the resource itself,
/// independent of whether an operator has permitted its use — that is
/// [`ResourceAdministrativeState`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceOperationalState {
    /// The resource is able to perform its function.
    #[serde(rename = "enabled")]
    Enabled,
    /// The resource is not able to perform its function.
    #[serde(rename = "disabled")]
    Disabled,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// Whether a resource is in use, and how heavily.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceUsageState {
    /// Not in use.
    #[serde(rename = "idle")]
    Idle,
    /// In use, with capacity to spare.
    #[serde(rename = "active")]
    Active,
    /// In use, with no capacity to spare.
    #[serde(rename = "busy")]
    Busy,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// Whether an operator has permitted the resource to be used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceAdministrativeState {
    /// Use is administratively prohibited.
    #[serde(rename = "locked")]
    Locked,
    /// Use is administratively permitted.
    #[serde(rename = "unlocked")]
    Unlocked,
    /// Existing users may finish; no new users are admitted.
    #[serde(rename = "shuttingDown")]
    ShuttingDown,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// Where a resource is in its plan-install-remove lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceLifecycleState {
    /// Designed but not yet installed.
    #[serde(rename = "planned")]
    Planned,
    /// Physically or logically in place.
    #[serde(rename = "installed")]
    Installed,
    /// Scheduled for removal.
    #[serde(rename = "pendingRemoval")]
    PendingRemoval,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// Severity of an outstanding alarm on a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceAlarmStatus {
    /// Being repaired.
    #[serde(rename = "underRepair")]
    UnderRepair,
    /// Critical alarm outstanding.
    #[serde(rename = "critical")]
    Critical,
    /// Major alarm outstanding.
    #[serde(rename = "major")]
    Major,
    /// Minor alarm outstanding.
    #[serde(rename = "minor")]
    Minor,
    /// An alarm is outstanding whose severity is not given.
    #[serde(rename = "alarmOutstanding")]
    AlarmOutstanding,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// Where a resource is in its initialisation procedure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceProceduralStatus {
    /// Needs initialising before it can be used.
    #[serde(rename = "initializationRequired")]
    InitializationRequired,
    /// Not initialised, and does not require it.
    #[serde(rename = "notInitialized")]
    NotInitialized,
    /// Initialising now.
    #[serde(rename = "initializing")]
    Initializing,
    /// Producing a report.
    #[serde(rename = "reporting")]
    Reporting,
    /// Shutting down.
    #[serde(rename = "terminating")]
    Terminating,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// Why a resource is unavailable, when it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceAvailabilityStatus {
    /// Undergoing a test.
    #[serde(rename = "inTest")]
    InTest,
    /// Has an internal fault.
    #[serde(rename = "failed")]
    Failed,
    /// Powered off.
    #[serde(rename = "powerOff")]
    PowerOff,
    /// Connected and reachable.
    #[serde(rename = "online")]
    Online,
    /// Requires a routine operation before it can be used.
    #[serde(rename = "offline")]
    Offline,
    /// Outside its scheduled service period.
    #[serde(rename = "offDuty")]
    OffDuty,
    /// Unavailable because something it depends on is.
    #[serde(rename = "dependency")]
    Dependency,
    /// Working, but below its specified capability.
    #[serde(rename = "degraded")]
    Degraded,
    /// Not installed.
    #[serde(rename = "notInstalled")]
    NotInstalled,
    /// Its log is full.
    #[serde(rename = "logFull")]
    LogFull,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// An operator-imposed restriction on a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ResourceControlStatus {
    /// Reserved for testing, but still carrying traffic.
    #[serde(rename = "subjectToTest")]
    SubjectToTest,
    /// Locked as part of a wider service lock.
    #[serde(rename = "partOfServicesLocked")]
    PartOfServicesLocked,
    /// Reserved for testing, and not carrying traffic.
    #[serde(rename = "reservedForTest")]
    ReservedForTest,
    /// Service has been administratively suspended.
    #[serde(rename = "suspended")]
    Suspended,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_dimensions_are_independent() {
        let json = r#"{"id":"1","operationalState":"enabled","administrativeState":"locked","availabilityStatus":"degraded","usageState":"busy"}"#;
        let r: Resource = serde_json::from_str(json).unwrap();
        assert_eq!(r.operational_state, Some(ResourceOperationalState::Enabled));
        assert_eq!(
            r.administrative_state,
            Some(ResourceAdministrativeState::Locked)
        );
        assert_eq!(
            r.availability_status,
            Some(ResourceAvailabilityStatus::Degraded)
        );
        assert_eq!(r.usage_state, Some(ResourceUsageState::Busy));
    }

    #[test]
    fn an_unknown_status_value_survives_the_round_trip() {
        let r: Resource = serde_json::from_str(r#"{"operationalState":"quiesced"}"#).unwrap();
        assert_eq!(
            r.operational_state,
            Some(ResourceOperationalState::Other("quiesced".into()))
        );
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            r#"{"operationalState":"quiesced"}"#
        );
    }

    #[test]
    fn the_read_model_is_the_patch_body() {
        // TMF639 declares no `Resource_MVO`; this is the point of the alias.
        let update: ResourceUpdate = Resource::builder().name("renamed").build();
        assert_eq!(update.name.as_deref(), Some("renamed"));
    }
}
