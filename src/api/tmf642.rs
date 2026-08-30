//! **TMF642 Alarm Management v5.0.1** — the client.
//!
//! Covers `alarm` and the six task collections TMF642 models its operations as
//! — `ackAlarm`, `unAckAlarm`, `clearAlarm`, `commentAlarm`, `groupAlarm` and
//! `unGroupAlarm` — plus the notification `hub`.
//!
//! Acting on alarms is not `PATCH`ing them. Each operation is a `POST` to its
//! own collection carrying an `alarm_pattern`, so one request acts on every
//! matching alarm; the task record you get back is the receipt, readable but
//! not editable.

use crate::alarm::{
    AckAlarm, AckAlarmCreate, Alarm, AlarmCreate, AlarmUpdate, ClearAlarm, ClearAlarmCreate,
    CommentAlarm, CommentAlarmCreate, GroupAlarm, GroupAlarmCreate, UnAckAlarm, UnAckAlarmCreate,
    UnGroupAlarm, UnGroupAlarmCreate,
};

use super::client::TmfClient;
use super::error::Result;
use super::hub::HubOps;
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::transport::Transport;
use super::{resource_ops, task_ops};

/// The conventional API root, appended to a host to form the base URL.
pub const API_PATH: &str = "tmf-api/alarmManagement/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.1";

const ALARMS: &str = "alarm";

/// A client for TMF642 Alarm Management.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf642::AlarmClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = AlarmClient::new("https://mycsp.com/tmf-api/alarmManagement/v5", transport)?;
///
/// let page = client
///     .list_alarms(&Query::new().filter("perceivedSeverity", "critical").limit(20))
///     .await?;
///
/// for alarm in page {
///     println!("{:?}: {:?}", alarm.id, alarm.probable_cause);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct AlarmClient {
    inner: TmfClient,
}

impl AlarmClient {
    /// Creates a client for `base_url`, dispatching through `transport`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidBaseUrl`] if the URL is empty.
    ///
    /// [`Error::InvalidBaseUrl`]: super::Error::InvalidBaseUrl
    pub fn new(base_url: impl Into<String>, transport: impl Transport + 'static) -> Result<Self> {
        Ok(Self {
            inner: TmfClient::new(base_url, transport)?,
        })
    }

    /// Creates a client for `host`, appending the conventional [`API_PATH`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidBaseUrl`] if the host is empty.
    ///
    /// [`Error::InvalidBaseUrl`]: super::Error::InvalidBaseUrl
    pub fn from_host(host: &str, transport: impl Transport + 'static) -> Result<Self> {
        Self::new(
            format!("{}/{API_PATH}", host.trim_end_matches('/')),
            transport,
        )
    }

    /// The underlying generic client, for operations this API does not wrap.
    #[must_use]
    pub fn inner(&self) -> &TmfClient {
        &self.inner
    }

    resource_ops!(
        ALARMS,
        read = Alarm,
        create = AlarmCreate,
        update = AlarmUpdate,
        list = list_alarms,
        stream = stream_alarms,
        get = get_alarm,
        new = create_alarm,
        patch = update_alarm,
        delete = delete_alarm,
        doc = "an alarm"
    );

    task_ops!(
        "ackAlarm",
        read = AckAlarm,
        create = AckAlarmCreate,
        list = list_acknowledgements,
        stream = stream_acknowledgements,
        get = get_acknowledgement,
        new = acknowledge_alarms,
        doc = "a request to acknowledge alarms"
    );

    task_ops!(
        "unAckAlarm",
        read = UnAckAlarm,
        create = UnAckAlarmCreate,
        list = list_unacknowledgements,
        stream = stream_unacknowledgements,
        get = get_unacknowledgement,
        new = unacknowledge_alarms,
        doc = "a request to withdraw an acknowledgement"
    );

    task_ops!(
        "clearAlarm",
        read = ClearAlarm,
        create = ClearAlarmCreate,
        list = list_clearances,
        stream = stream_clearances,
        get = get_clearance,
        new = clear_alarms,
        doc = "a request to clear alarms"
    );

    task_ops!(
        "commentAlarm",
        read = CommentAlarm,
        create = CommentAlarmCreate,
        list = list_comments,
        stream = stream_comments,
        get = get_comment,
        new = comment_on_alarms,
        doc = "a request to comment on alarms"
    );

    task_ops!(
        "groupAlarm",
        read = GroupAlarm,
        create = GroupAlarmCreate,
        list = list_groupings,
        stream = stream_groupings,
        get = get_grouping,
        new = group_alarms,
        doc = "a request to group alarms under a parent"
    );

    task_ops!(
        "unGroupAlarm",
        read = UnGroupAlarm,
        create = UnGroupAlarmCreate,
        list = list_ungroupings,
        stream = stream_ungroupings,
        get = get_ungrouping,
        new = ungroup_alarms,
        doc = "a request to break a correlation group apart"
    );
}

impl HubOps for AlarmClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
