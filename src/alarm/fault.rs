//! The `Alarm` resource and its value objects — TMF642.

use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct, tmf_value};
use crate::core::{AlarmedObject, ExternalIdentifier, Ref, Threshold, Timestamp};
use crate::service::Service;

/// The X.733 alarm category: what *kind* of fault this is.
///
/// [`AlarmType::Other`] preserves a value outside the v5 enumeration rather
/// than failing the parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum AlarmType {
    /// A fault in the communications path.
    #[serde(rename = "communicationsAlarm")]
    Communications,
    /// A software or processing fault.
    #[serde(rename = "processingErrorAlarm")]
    ProcessingError,
    /// A condition of the surroundings — power, temperature, humidity.
    #[serde(rename = "environmentalAlarm")]
    Environmental,
    /// A degradation of service quality.
    #[serde(rename = "qualityOfServiceAlarm")]
    QualityOfService,
    /// A hardware fault.
    #[serde(rename = "equipmentAlarm")]
    Equipment,
    /// Data was created, altered or destroyed improperly.
    #[serde(rename = "integrityViolation")]
    IntegrityViolation,
    /// An operation was performed improperly.
    #[serde(rename = "operationalViolation")]
    OperationalViolation,
    /// A physical resource was interfered with.
    #[serde(rename = "physicalViolation")]
    PhysicalViolation,
    /// A security service or mechanism reported a violation.
    #[serde(rename = "securityService")]
    SecurityService,
    /// A security mechanism was violated.
    #[serde(rename = "mechanismViolation")]
    MechanismViolation,
    /// Something happened at an improper time.
    #[serde(rename = "timeDomainViolation")]
    TimeDomainViolation,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// How bad the fault is, as the reporting system judges it — X.733 severity.
///
/// Note that [`PerceivedSeverity::Cleared`] is a severity rather than a
/// separate state: an alarm that has been cleared reports it here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum PerceivedSeverity {
    /// Service-affecting and needs immediate action.
    #[serde(rename = "critical")]
    Critical,
    /// Service-affecting and needs urgent action.
    #[serde(rename = "major")]
    Major,
    /// Not service-affecting; act before it becomes so.
    #[serde(rename = "minor")]
    Minor,
    /// A potential fault, reported before service is affected.
    #[serde(rename = "warning")]
    Warning,
    /// The severity could not be determined.
    #[serde(rename = "indeterminate")]
    Indeterminate,
    /// The condition that raised the alarm is gone.
    #[serde(rename = "cleared")]
    Cleared,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl PerceivedSeverity {
    /// Whether the alarm is still live.
    ///
    /// An unrecognised severity counts as **active**: a dashboard that hides
    /// what it does not understand hides exactly the fault worth looking at.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Cleared)
    }
}

/// Where an [`Alarm`] sits in its own lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum AlarmState {
    /// First reported.
    #[serde(rename = "raised")]
    Raised,
    /// Changed since it was raised.
    #[serde(rename = "updated")]
    Updated,
    /// The condition is gone.
    #[serde(rename = "cleared")]
    Cleared,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// Whether an operator has taken responsibility for an [`Alarm`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum AckState {
    /// Nobody has picked it up.
    #[serde(rename = "unacknowledged")]
    Unacknowledged,
    /// Somebody has.
    #[serde(rename = "acknowledged")]
    Acknowledged,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// Whether the fault falls inside planned work.
///
/// Note the casing: TMF642 spells these values with a leading capital, against
/// the `camelCase` of every other v5 enumeration. The wire values are what the
/// specification says, not what consistency would suggest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum PlannedOutageIndicator {
    /// The resource is meant to be carrying traffic.
    #[serde(rename = "InService")]
    InService,
    /// The resource is out for planned work.
    #[serde(rename = "OutOfService")]
    OutOfService,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

tmf_struct! {
    @name = "Alarm", @ref = "AlarmRef";
    /// A fault reported by the network.
    ///
    /// ```
    /// use rutmf::alarm::{Alarm, PerceivedSeverity};
    ///
    /// let json = r#"{"@type":"Alarm","perceivedSeverity":"critical"}"#;
    /// let alarm: Alarm = serde_json::from_str(json).unwrap();
    ///
    /// assert!(alarm.perceived_severity.unwrap().is_active());
    /// ```
    pub struct Alarm {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this alarm.
        href: String,
        /// Short name for the alarm, as an `AlarmRef` carries it.
        name: String,
        /// The category of fault — X.733 `alarmType`.
        alarm_type: AlarmType,
        /// How bad it is.
        perceived_severity: PerceivedSeverity,
        /// The probable cause, from the X.733 vocabulary.
        probable_cause: String,
        /// A narrower description of the problem.
        specific_problem: String,
        /// Free-text detail from the reporting system.
        alarm_details: String,
        /// What the alarm is about.
        alarmed_object: Ref<AlarmedObject>,
        /// The class of the alarmed object.
        alarmed_object_type: String,
        /// Where the alarm sits in its own lifecycle.
        state: AlarmState,
        /// Whether the alarm has been acknowledged.
        ack_state: AckState,
        /// The system that acknowledged it.
        ack_system_id: String,
        /// The user who acknowledged it.
        ack_user_id: String,
        /// The system that cleared it.
        clear_system_id: String,
        /// The user who cleared it.
        clear_user_id: String,
        /// When the fault was first detected.
        alarm_raised_time: Timestamp,
        /// When the alarm last changed.
        alarm_changed_time: Timestamp,
        /// When the fault stopped.
        alarm_cleared_time: Timestamp,
        /// When the alarm was reported onward.
        alarm_reporting_time: Timestamp,
        /// Whether the alarm has been escalated.
        alarm_escalation: bool,
        /// Whether this alarm is the root cause of a correlated group.
        is_root_cause: bool,
        /// Whether service is affected.
        service_affecting: bool,
        /// Whether the fault falls inside a planned outage.
        planned_outage_indicator: PlannedOutageIndicator,
        /// What the reporting system suggests doing about it.
        proposed_repaired_actions: String,
        /// The system that reported the alarm onward.
        reporting_system_id: String,
        /// The system the alarm originated in.
        source_system_id: String,
        /// The identifier the source system knows this alarm by.
        external_alarm_id: String,
        /// Alarms correlated with this one.
        correlated_alarm: Vec<Ref<Alarm>>,
        /// The alarms this one was grouped under.
        parent_alarm: Vec<Ref<Alarm>>,
        /// Services this fault affects.
        affected_service: Vec<Ref<Service>>,
        /// Where the fault is.
        place: Vec<RelatedPlace>,
        /// Operator comments added as the alarm is worked.
        comment: Vec<Comment>,
        /// The threshold crossing that raised the alarm, where one did.
        crossed_threshold_information: CrossedThresholdInformation,
        @renamed {
            /// The concrete class an `AlarmRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "Alarm";
    /// Body of a `POST /alarm` — the v5 `Alarm_FVO`.
    ///
    /// TMF642 demands more on create than most v5 resources: seven members,
    /// because an alarm nobody can categorise, locate in time or attribute to a
    /// system is not actionable.
    pub struct AlarmCreate {
        @required {
            /// The category of fault. **Required on create.**
            alarm_type: AlarmType,
            /// How bad it is. **Required on create.**
            perceived_severity: PerceivedSeverity,
            /// The probable cause. **Required on create.**
            probable_cause: String,
            /// What the alarm is about. **Required on create.**
            alarmed_object: Ref<AlarmedObject>,
            /// When the fault was first detected. **Required on create.**
            alarm_raised_time: Timestamp,
            /// The system the alarm originated in. **Required on create.**
            source_system_id: String,
            /// The alarm's own lifecycle state. **Required on create.**
            state: AlarmState,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// A narrower description of the problem.
        specific_problem: String,
        /// Free-text detail from the reporting system.
        alarm_details: String,
        /// The class of the alarmed object.
        alarmed_object_type: String,
        /// Whether the alarm has been acknowledged.
        ack_state: AckState,
        /// The system that acknowledged it.
        ack_system_id: String,
        /// The user who acknowledged it.
        ack_user_id: String,
        /// The system that cleared it.
        clear_system_id: String,
        /// The user who cleared it.
        clear_user_id: String,
        /// When the alarm last changed.
        alarm_changed_time: Timestamp,
        /// When the fault stopped.
        alarm_cleared_time: Timestamp,
        /// When the alarm was reported onward.
        alarm_reporting_time: Timestamp,
        /// Whether the alarm has been escalated.
        alarm_escalation: bool,
        /// Whether this alarm is the root cause.
        is_root_cause: bool,
        /// Whether service is affected.
        service_affecting: bool,
        /// Whether the fault falls inside a planned outage.
        planned_outage_indicator: PlannedOutageIndicator,
        /// What to do about it.
        proposed_repaired_actions: String,
        /// The system that reported the alarm onward.
        reporting_system_id: String,
        /// The identifier the source system knows this alarm by.
        external_alarm_id: String,
        /// Alarms correlated with this one.
        correlated_alarm: Vec<Ref<Alarm>>,
        /// The alarms this one was grouped under.
        parent_alarm: Vec<Ref<Alarm>>,
        /// Services this fault affects.
        affected_service: Vec<Ref<Service>>,
        /// Where the fault is.
        place: Vec<RelatedPlace>,
        /// Operator comments.
        comment: Vec<Comment>,
        /// The threshold crossing that raised the alarm.
        crossed_threshold_information: CrossedThresholdInformation,
    }
}

tmf_struct! {
    @name = "Alarm";
    /// Body of a `PATCH /alarm/{id}` — the v5 `Alarm_MVO`.
    ///
    /// Like TMF621 and unlike most of this crate, TMF642 leaves `id` and `href`
    /// on the patch body. What it does drop is `alarmRaisedTime` and
    /// `sourceSystemId`: when and where a fault was first seen is history, and
    /// history is not editable.
    pub struct AlarmUpdate {
        /// Identifier, which TMF642 leaves on the patch body.
        id: String,
        /// Canonical URI, likewise.
        href: String,
        /// New alarm type.
        alarm_type: AlarmType,
        /// New severity — set `cleared` to clear the alarm.
        perceived_severity: PerceivedSeverity,
        /// New probable cause.
        probable_cause: String,
        /// New specific problem.
        specific_problem: String,
        /// New detail.
        alarm_details: String,
        /// New alarmed object.
        alarmed_object: Ref<AlarmedObject>,
        /// New alarmed object type.
        alarmed_object_type: String,
        /// New lifecycle state.
        state: AlarmState,
        /// New acknowledgement state.
        ack_state: AckState,
        /// New acknowledging system.
        ack_system_id: String,
        /// New acknowledging user.
        ack_user_id: String,
        /// New clearing system.
        clear_system_id: String,
        /// New clearing user.
        clear_user_id: String,
        /// New changed time.
        alarm_changed_time: Timestamp,
        /// New cleared time.
        alarm_cleared_time: Timestamp,
        /// New reporting time.
        alarm_reporting_time: Timestamp,
        /// New escalation flag.
        alarm_escalation: bool,
        /// New root-cause flag.
        is_root_cause: bool,
        /// New service-affecting flag.
        service_affecting: bool,
        /// New planned-outage indicator.
        planned_outage_indicator: PlannedOutageIndicator,
        /// New proposed actions.
        proposed_repaired_actions: String,
        /// New reporting system.
        reporting_system_id: String,
        /// New external alarm identifier.
        external_alarm_id: String,
        /// Replacement correlated alarms.
        correlated_alarm: Vec<Ref<Alarm>>,
        /// Replacement parent alarms.
        parent_alarm: Vec<Ref<Alarm>>,
        /// Replacement affected services.
        affected_service: Vec<Ref<Service>>,
        /// Replacement places.
        place: Vec<RelatedPlace>,
        /// Replacement comments.
        comment: Vec<Comment>,
        /// New threshold-crossing detail.
        crossed_threshold_information: CrossedThresholdInformation,
    }
}

tmf_struct! {
    @name = "Comment";
    /// An operator's remark on an alarm.
    pub struct Comment {
        /// The remark itself.
        comment: String,
        /// Who wrote it.
        user_id: String,
        /// Which system it came from.
        system_id: String,
        /// When it was written.
        time: Timestamp,
    }
}

tmf_value! {
    /// The threshold crossing that raised an alarm.
    ///
    /// A plain object: TMF642 gives it no `@type` and no polymorphism
    /// attributes, so it is declared with `tmf_value!` rather than
    /// `tmf_struct!`.
    pub struct CrossedThresholdInformation {
        /// The indicator that crossed, e.g. `packetLoss`.
        indicator_name: String,
        /// Its unit of measure.
        indicator_unit: String,
        /// The value observed when the alarm was raised.
        observed_value: String,
        /// Which way it crossed — `up` or `down`.
        direction: String,
        /// How the indicator was sampled.
        granularity: String,
        /// A description of the crossing.
        threshold_crossing_description: String,
        /// The threshold that was crossed — TMF628.
        threshold: Ref<Threshold>,
    }
}

tmf_struct! {
    @name = "RelatedPlace";
    /// A place in a named role on an alarm.
    ///
    /// TMF642 declares its own `RelatedPlace`, and it is not the shape TMF639
    /// gives that name — this one nests a whole [`Place`], where the inventory's
    /// carries a reference. Two schemas, one name, so two types.
    pub struct RelatedPlace {
        /// The place itself.
        related_place: Place,
        /// The role it plays — where the fault is, where the equipment sits.
        role: String,
    }
}

tmf_struct! {
    @name = "Place", @ref = "PlaceRef";
    /// A location, as TMF642 carries it.
    pub struct Place {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this place.
        href: String,
        /// Identifiers this place is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_entity!(Alarm);
tmf_patch_body!(AlarmUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_severity_is_still_active() {
        // A dashboard that hides what it does not understand hides the fault
        // worth looking at.
        let json = r#"{"@type":"Alarm","perceivedSeverity":"catastrophic"}"#;
        let alarm: Alarm = serde_json::from_str(json).unwrap();
        let severity = alarm.perceived_severity.clone().expect("a severity");

        assert_eq!(severity, PerceivedSeverity::Other("catastrophic".into()));
        assert!(severity.is_active());
        assert_eq!(
            serde_json::to_value(&alarm).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn cleared_is_a_severity_not_a_state() {
        // X.733 reports "the condition is gone" through the severity, which is
        // why there is no separate `cleared` flag to keep in step with it.
        assert!(!PerceivedSeverity::Cleared.is_active());
        assert!(PerceivedSeverity::Critical.is_active());
    }

    #[test]
    fn an_unknown_alarm_type_round_trips() {
        let json = r#"{"@type":"Alarm","alarmType":"vendorSpecificAlarm"}"#;
        let alarm: Alarm = serde_json::from_str(json).unwrap();
        assert_eq!(
            alarm.alarm_type,
            Some(AlarmType::Other("vendorSpecificAlarm".into()))
        );
    }
}
