//! The six alarm tasks — TMF642.
//!
//! Acknowledging, clearing, commenting on, grouping and ungrouping are
//! collections you `POST` to, not fields you `PATCH`. Each has a read model and
//! a `…Create`, and deliberately **no** `…Update`: TMF642 defines `POST` and
//! `GET` on these paths and nothing else, so a type that could be sent to a
//! `PATCH` would be a type no endpoint accepts.
//!
//! # Why these are written out rather than generated
//!
//! The six look alike and are not. Four carry an `alarmPattern` for bulk
//! action; the two grouping tasks do not, because a group is defined by naming
//! its members rather than by matching them. Each `_FVO` requires a different
//! set. A macro that assumed the common shape produced types the specification
//! does not define — so the field lists are written per task, and
//! `tests/coverage.rs` is what checks them.

use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_struct};
use crate::core::{Ref, Timestamp};

use super::{Alarm, Comment};

/// How far one of the six alarm tasks has got.
///
/// All six share this vocabulary, which is not the same as an alarm's own
/// [`AlarmState`](super::AlarmState) — a task can be `done` while the alarm it
/// acted on is still `raised`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum AlarmTaskState {
    /// Accepted and queued.
    #[serde(rename = "acknowledged")]
    Acknowledged,
    /// Being carried out.
    #[serde(rename = "inProgress")]
    InProgress,
    /// Carried out.
    #[serde(rename = "done")]
    Done,
    /// Asked to stop.
    #[serde(rename = "cancel")]
    Cancel,
    /// Stopped before completing.
    #[serde(rename = "canceled")]
    Canceled,
    /// Failed.
    #[serde(rename = "terminatedWithError")]
    TerminatedWithError,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl AlarmTaskState {
    /// Whether the task has stopped moving.
    ///
    /// An unrecognised state is **not** finished, so a client polling a task
    /// keeps polling rather than giving up on a state it does not know.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            Self::Done | Self::Canceled | Self::TerminatedWithError
        )
    }
}

tmf_struct! {
    @name = "AckAlarm";
    /// A request to acknowledge one or more alarms.
    pub struct AckAlarm {
        /// Server-assigned identifier of the task.
        id: String,
        /// Canonical URI of the task.
        href: String,
        /// How far the task has got.
        state: AlarmTaskState,
        /// A filter selecting the alarms to act on, for bulk requests.
        alarm_pattern: Vec<Ref<Alarm>>,
        /// The alarms this request acknowledged.
        acked_alarm: Vec<Ref<Alarm>>,
        /// When they were acknowledged.
        ack_time: Timestamp,
        /// The system that acknowledged them.
        ack_system_id: String,
        /// The user who acknowledged them.
        ack_user_id: String,
    }
}

tmf_struct! {
    @name = "AckAlarm";
    /// Body of a `POST /ackAlarm` — the v5 `AckAlarm_FVO`.
    ///
    /// There is no matching update type: TMF642 defines `POST` and `GET` on
    /// this collection and nothing else.
    pub struct AckAlarmCreate {
        @required {
            /// Which alarms to acknowledge. **Required.**
            alarm_pattern: Vec<Ref<Alarm>>,
            /// The system acknowledging them. **Required.**
            ack_system_id: String,
            /// The user acknowledging them. **Required.**
            ack_user_id: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// The state to open the task in.
        state: AlarmTaskState,
        /// The alarms acknowledged.
        acked_alarm: Vec<Ref<Alarm>>,
        /// When they were acknowledged.
        ack_time: Timestamp,
    }
}

tmf_struct! {
    @name = "UnAckAlarm";
    /// A request to withdraw an acknowledgement.
    pub struct UnAckAlarm {
        /// Server-assigned identifier of the task.
        id: String,
        /// Canonical URI of the task.
        href: String,
        /// How far the task has got.
        state: AlarmTaskState,
        /// A filter selecting the alarms to act on.
        alarm_pattern: Vec<Ref<Alarm>>,
        /// The alarms this request un-acknowledged.
        un_acked_alarm: Vec<Ref<Alarm>>,
        /// When the acknowledgement was withdrawn.
        ack_time: Timestamp,
        /// The system that withdrew it.
        ack_system_id: String,
        /// The user who withdrew it.
        ack_user_id: String,
    }
}

tmf_struct! {
    @name = "UnAckAlarm";
    /// Body of a `POST /unAckAlarm` — the v5 `UnAckAlarm_FVO`.
    pub struct UnAckAlarmCreate {
        @required {
            /// Which alarms to un-acknowledge. **Required.**
            alarm_pattern: Vec<Ref<Alarm>>,
            /// The system withdrawing the acknowledgement. **Required.**
            ack_system_id: String,
            /// The user withdrawing it. **Required.**
            ack_user_id: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// The state to open the task in.
        state: AlarmTaskState,
        /// The alarms un-acknowledged.
        un_acked_alarm: Vec<Ref<Alarm>>,
        /// When the acknowledgement was withdrawn.
        ack_time: Timestamp,
    }
}

tmf_struct! {
    @name = "ClearAlarm";
    /// A request to clear one or more alarms.
    ///
    /// Clearing is a task rather than a severity edit because it is an
    /// *assertion by an operator* that the condition is gone — which the
    /// network may disagree with by raising the alarm again.
    pub struct ClearAlarm {
        /// Server-assigned identifier of the task.
        id: String,
        /// Canonical URI of the task.
        href: String,
        /// How far the task has got.
        state: AlarmTaskState,
        /// A filter selecting the alarms to act on.
        alarm_pattern: Vec<Ref<Alarm>>,
        /// The alarms this request cleared.
        cleared_alarm: Vec<Ref<Alarm>>,
        /// When they were cleared.
        alarm_cleared_time: Timestamp,
        /// The system that cleared them.
        clear_system_id: String,
        /// The user who cleared them.
        clear_user_id: String,
    }
}

tmf_struct! {
    @name = "ClearAlarm";
    /// Body of a `POST /clearAlarm` — the v5 `ClearAlarm_FVO`.
    pub struct ClearAlarmCreate {
        @required {
            /// Which alarms to clear. **Required.**
            alarm_pattern: Vec<Ref<Alarm>>,
            /// When they were cleared. **Required.**
            alarm_cleared_time: Timestamp,
            /// The system clearing them. **Required.**
            clear_system_id: String,
            /// The user clearing them. **Required.**
            clear_user_id: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// The state to open the task in.
        state: AlarmTaskState,
        /// The alarms cleared.
        cleared_alarm: Vec<Ref<Alarm>>,
    }
}

tmf_struct! {
    @name = "CommentAlarm";
    /// A request to add a comment to one or more alarms.
    pub struct CommentAlarm {
        /// Server-assigned identifier of the task.
        id: String,
        /// Canonical URI of the task.
        href: String,
        /// How far the task has got.
        state: AlarmTaskState,
        /// A filter selecting the alarms to act on.
        alarm_pattern: Vec<Ref<Alarm>>,
        /// The alarms this request commented on.
        commented_alarm: Vec<Ref<Alarm>>,
        /// The comment added.
        comment: Comment,
    }
}

tmf_struct! {
    @name = "CommentAlarm";
    /// Body of a `POST /commentAlarm` — the v5 `CommentAlarm_FVO`.
    pub struct CommentAlarmCreate {
        @required {
            /// Which alarms to comment on. **Required.**
            alarm_pattern: Vec<Ref<Alarm>>,
            /// The comment to add. **Required.**
            comment: Comment,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// The state to open the task in.
        state: AlarmTaskState,
        /// The alarms commented on.
        commented_alarm: Vec<Ref<Alarm>>,
    }
}

tmf_struct! {
    @name = "GroupAlarm";
    /// A request to group alarms under a parent, for correlation.
    ///
    /// Note the absence of an `alarmPattern`: a group is defined by naming its
    /// members, not by matching them.
    pub struct GroupAlarm {
        /// Server-assigned identifier of the task.
        id: String,
        /// Canonical URI of the task.
        href: String,
        /// How far the task has got.
        state: AlarmTaskState,
        /// The alarms being grouped.
        grouped_alarm: Vec<Ref<Alarm>>,
        /// The alarms correlated by the grouping.
        correlated_alarm: Vec<Ref<Alarm>>,
        /// The alarm they are grouped under.
        parent_alarm: Ref<Alarm>,
        /// When the grouping happened.
        alarm_changed_time: Timestamp,
        /// The system that grouped them.
        source_system_id: String,
    }
}

tmf_struct! {
    @name = "GroupAlarm";
    /// Body of a `POST /groupAlarm` — the v5 `GroupAlarm_FVO`.
    pub struct GroupAlarmCreate {
        @required {
            /// The alarms to correlate. **Required.**
            correlated_alarm: Vec<Ref<Alarm>>,
            /// The alarm to group them under. **Required.**
            parent_alarm: Ref<Alarm>,
            /// When the grouping happened. **Required.**
            alarm_changed_time: Timestamp,
            /// The system grouping them. **Required.**
            source_system_id: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// The state to open the task in.
        state: AlarmTaskState,
        /// The alarms grouped.
        grouped_alarm: Vec<Ref<Alarm>>,
    }
}

tmf_struct! {
    @name = "UnGroupAlarm";
    /// A request to break a correlation group apart.
    pub struct UnGroupAlarm {
        /// Server-assigned identifier of the task.
        id: String,
        /// Canonical URI of the task.
        href: String,
        /// How far the task has got.
        state: AlarmTaskState,
        /// The alarms being removed from the group.
        un_grouped_alarm: Vec<Ref<Alarm>>,
        /// The alarms that were correlated.
        correlated_alarm: Vec<Ref<Alarm>>,
        /// The alarm they were grouped under.
        parent_alarm: Ref<Alarm>,
        /// When the grouping was broken.
        alarm_changed_time: Timestamp,
        /// The system that broke it.
        source_system_id: String,
    }
}

tmf_struct! {
    @name = "UnGroupAlarm";
    /// Body of a `POST /unGroupAlarm` — the v5 `UnGroupAlarm_FVO`.
    pub struct UnGroupAlarmCreate {
        @required {
            /// The alarms to un-correlate. **Required.**
            correlated_alarm: Vec<Ref<Alarm>>,
            /// The alarm they were grouped under. **Required.**
            parent_alarm: Ref<Alarm>,
            /// When the grouping was broken. **Required.**
            alarm_changed_time: Timestamp,
            /// The system breaking it. **Required.**
            source_system_id: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// The state to open the task in.
        state: AlarmTaskState,
        /// The alarms removed from the group.
        un_grouped_alarm: Vec<Ref<Alarm>>,
    }
}

tmf_entity!(
    AckAlarm,
    UnAckAlarm,
    ClearAlarm,
    CommentAlarm,
    GroupAlarm,
    UnGroupAlarm
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_task_state_is_not_finished() {
        // A client polling a task must keep polling through a state it does not
        // recognise; treating unknown as finished would report an operation as
        // complete on the strength of not understanding it.
        let unknown = AlarmTaskState::Other("awaitingApproval".into());
        assert!(!unknown.is_finished());
        assert!(!AlarmTaskState::InProgress.is_finished());
        assert!(AlarmTaskState::TerminatedWithError.is_finished());
    }

    #[test]
    fn task_states_round_trip_their_wire_spelling() {
        for (state, wire) in [
            (AlarmTaskState::InProgress, "inProgress"),
            (AlarmTaskState::TerminatedWithError, "terminatedWithError"),
            (AlarmTaskState::Canceled, "canceled"),
        ] {
            assert_eq!(
                serde_json::to_string(&state).unwrap(),
                format!("\"{wire}\"")
            );
            assert_eq!(
                serde_json::from_str::<AlarmTaskState>(&format!("\"{wire}\"")).unwrap(),
                state
            );
        }
    }

    #[test]
    fn a_task_carries_a_pattern_for_bulk_action() {
        // The reason four of these are collections rather than a `PATCH`: one
        // request acts on everything matching, instead of a loop.
        let ack = AckAlarmCreate::builder()
            .alarm_pattern(vec![Ref::new("alarm-1"), Ref::new("alarm-2")])
            .ack_system_id("noc")
            .ack_user_id("operator")
            .build();

        let json = serde_json::to_value(&ack).unwrap();
        assert_eq!(json["alarmPattern"].as_array().unwrap().len(), 2);
        assert_eq!(json["@type"], "AckAlarm");
    }

    #[test]
    fn grouping_names_its_members_rather_than_matching_them() {
        // `GroupAlarm` has no `alarmPattern`, which is why the six tasks are
        // written out rather than generated from one shape.
        let group = GroupAlarmCreate::builder()
            .correlated_alarm(vec![Ref::new("a-1")])
            .parent_alarm(Ref::new("a-root"))
            .alarm_changed_time(
                "2026-08-27T00:00:00Z"
                    .parse::<crate::core::Timestamp>()
                    .unwrap(),
            )
            .source_system_id("correlator")
            .build();

        let json = serde_json::to_value(&group).unwrap();
        assert!(json.get("alarmPattern").is_none());
        assert_eq!(json["parentAlarm"]["id"], "a-root");
    }

    #[test]
    fn every_task_declares_its_own_discriminator() {
        use crate::core::TmfType;
        assert_eq!(AckAlarm::TYPE_NAME, "AckAlarm");
        assert_eq!(UnAckAlarm::TYPE_NAME, "UnAckAlarm");
        assert_eq!(ClearAlarm::TYPE_NAME, "ClearAlarm");
        assert_eq!(CommentAlarm::TYPE_NAME, "CommentAlarm");
        assert_eq!(GroupAlarm::TYPE_NAME, "GroupAlarm");
        assert_eq!(UnGroupAlarm::TYPE_NAME, "UnGroupAlarm");
    }
}
