//! Fault management: what the network is complaining about — TMF642.
//!
//! An [`Alarm`] is a fault the network reported. Where a
//! [`TroubleTicket`](crate::ticket::TroubleTicket) is the human process of
//! fixing something, an alarm is the machine's account of what is wrong — and
//! one usually raises the other.
//!
//! # Six operations that are resources
//!
//! Acknowledging, clearing, commenting on, grouping and ungrouping alarms are
//! not `PATCH`es in TMF642. Each is its own collection — `ackAlarm`,
//! `clearAlarm`, `commentAlarm`, `groupAlarm`, `unAckAlarm`, `unGroupAlarm` —
//! that you `POST` to and then read back, exactly the shape TMF622 gives
//! cancellation.
//!
//! That is why [`AckAlarm`] and its siblings have a `…Create` type but **no**
//! `…Update`: the specification defines `POST` and `GET` on them and nothing
//! else. The task record is the receipt for the request, not a thing you edit.
//!
//! Acting on several alarms at once is the point of the design. The four
//! tasks that select alarms — acknowledge, un-acknowledge, clear, comment —
//! carry an `alarm_pattern`, so "acknowledge everything matching this" is one
//! request rather than a loop. The two grouping tasks do **not**: a correlation
//! group is defined by naming its members, not by matching them.

mod fault;
mod task;

pub use fault::{
    AckState, Alarm, AlarmCreate, AlarmState, AlarmType, AlarmUpdate, Comment,
    CrossedThresholdInformation, PerceivedSeverity, Place, PlannedOutageIndicator, RelatedPlace,
};
pub use task::{
    AckAlarm, AckAlarmCreate, AlarmTaskState, ClearAlarm, ClearAlarmCreate, CommentAlarm,
    CommentAlarmCreate, GroupAlarm, GroupAlarmCreate, UnAckAlarm, UnAckAlarmCreate, UnGroupAlarm,
    UnGroupAlarmCreate,
};
