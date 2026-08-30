//! End-to-end tests for TMF642 Alarm Management — fault management.

#![cfg(all(feature = "api-tmf642", feature = "mock"))]

use serde_json::json;

use rutmf::alarm::{
    AckAlarmCreate, Alarm, AlarmCreate, AlarmState, AlarmType, AlarmUpdate, PerceivedSeverity,
};
use rutmf::api::{HubCreate, HubOps, Query, tmf642::AlarmClient};
use rutmf::core::{EventKind, Ref};
use rutmf::mock::MockTmfServer;

fn alarms(server: &MockTmfServer) -> AlarmClient {
    AlarmClient::new(server.base_url(), server.transport()).expect("a valid base URL")
}

fn raised_at() -> rutmf::core::Timestamp {
    "2026-08-27T00:00:00Z"
        .parse()
        .expect("a valid RFC 3339 time")
}

#[tokio::test]
async fn an_alarm_round_trips_through_the_client() {
    let server = MockTmfServer::new();
    let client = alarms(&server);

    let created = client
        .create_alarm(
            &AlarmCreate::builder()
                .alarm_type(AlarmType::Equipment)
                .perceived_severity(PerceivedSeverity::Critical)
                .probable_cause("lossOfSignal")
                .alarmed_object(Ref::new("port-3/1/4"))
                .alarm_raised_time(raised_at())
                .source_system_id("ems-north")
                .state(AlarmState::Raised)
                .build(),
        )
        .await
        .expect("the alarm is raised");

    let id = created.id.clone().expect("the server assigns an id");
    let fetched: Alarm = client
        .get_alarm(&id, &Query::new())
        .await
        .expect("and can be read back");

    assert_eq!(fetched.probable_cause.as_deref(), Some("lossOfSignal"));
    assert_eq!(fetched.alarm_type, Some(AlarmType::Equipment));
    assert!(fetched.perceived_severity.unwrap().is_active());
}

/// Acting on alarms is a `POST` to a task collection, not a `PATCH`.
#[tokio::test]
async fn acknowledging_is_a_task_that_acts_in_bulk() {
    let server = MockTmfServer::new();

    let receipt = alarms(&server)
        .acknowledge_alarms(
            &AckAlarmCreate::builder()
                .alarm_pattern(vec![Ref::new("alarm-1"), Ref::new("alarm-2")])
                .ack_system_id("noc")
                .ack_user_id("operator")
                .build(),
        )
        .await
        .expect("the acknowledgement is accepted");

    assert!(receipt.id.is_some(), "the task is a resource with an id");
    assert_eq!(receipt.ack_user_id.as_deref(), Some("operator"));
    assert_eq!(
        receipt.alarm_pattern.as_ref().map(Vec::len),
        Some(2),
        "one request covers every matching alarm"
    );

    // And the receipt is readable afterwards.
    let id = receipt.id.clone().unwrap();
    let read = alarms(&server)
        .get_acknowledgement(&id, &Query::new())
        .await
        .expect("the task record can be read back");
    assert_eq!(read.ack_system_id.as_deref(), Some("noc"));
}

#[tokio::test]
async fn clearing_an_alarm_sets_the_severity_that_says_so() {
    let server = MockTmfServer::new();
    server.seed(
        "alarm",
        json!({
            "id": "a-1",
            "@type": "Alarm",
            "perceivedSeverity": "major",
            "probableCause": "linkDown",
        }),
    );

    let updated = alarms(&server)
        .update_alarm(
            "a-1",
            &AlarmUpdate::builder()
                .perceived_severity(PerceivedSeverity::Cleared)
                .build(),
        )
        .await
        .expect("the patch applies");

    let severity = updated.perceived_severity.expect("a severity");
    assert!(
        !severity.is_active(),
        "cleared is how X.733 says it is over"
    );
    assert_eq!(
        updated.probable_cause.as_deref(),
        Some("linkDown"),
        "a merge patch must not clear members it does not name"
    );
}

#[tokio::test]
async fn filtering_by_severity_reaches_the_store() {
    let server = MockTmfServer::new();
    for (id, severity) in [("a-1", "critical"), ("a-2", "minor"), ("a-3", "critical")] {
        server.seed(
            "alarm",
            json!({"id": id, "@type": "Alarm", "perceivedSeverity": severity}),
        );
    }

    let page = alarms(&server)
        .list_alarms(&Query::new().filter("perceivedSeverity", "critical"))
        .await
        .expect("the filter is applied");

    assert_eq!(page.items.len(), 2);
}

#[tokio::test]
async fn subscribing_derives_the_alarm_event_name() {
    let server = MockTmfServer::new();

    let hub = alarms(&server)
        .register_listener(&HubCreate::for_resource::<Alarm>(
            "https://me/callback",
            EventKind::StateChange,
        ))
        .await
        .expect("the hub accepts the subscription");

    assert_eq!(
        hub.query.as_deref(),
        Some("eventType=AlarmStateChangeEvent")
    );
}
