//! End-to-end tests for TMF621 Trouble Ticket — the assurance domain.

#![cfg(all(feature = "api-tmf621", feature = "mock"))]

use serde_json::json;

use rutmf::api::{HubCreate, HubOps, Query, tmf621::TroubleTicketClient};
use rutmf::core::EventKind;
use rutmf::mock::MockTmfServer;
use rutmf::ticket::{TroubleTicket, TroubleTicketCreate, TroubleTicketStatus, TroubleTicketUpdate};

fn tickets(server: &MockTmfServer) -> TroubleTicketClient {
    TroubleTicketClient::new(server.base_url(), server.transport()).expect("a valid base URL")
}

#[tokio::test]
async fn a_ticket_round_trips_through_the_client() {
    let server = MockTmfServer::new();
    let client = tickets(&server);

    let created = client
        .create_trouble_ticket(
            &TroubleTicketCreate::builder()
                .description("Fibre down at site 42")
                .severity("critical")
                .ticket_type("networkFault")
                .build(),
        )
        .await
        .expect("the ticket is raised");

    let id = created.id.clone().expect("the server assigns an id");
    let fetched = client
        .get_trouble_ticket(&id, &Query::new())
        .await
        .expect("and can be read back");

    assert_eq!(
        fetched.description.as_deref(),
        Some("Fibre down at site 42")
    );
    assert_eq!(fetched.type_name(), "TroubleTicket");
}

#[tokio::test]
async fn an_unknown_status_does_not_end_a_poll() {
    let server = MockTmfServer::new();
    server.seed(
        "troubleTicket",
        json!({"id": "t-1", "@type": "TroubleTicket", "status": "awaitingFieldVisit"}),
    );

    let ticket: TroubleTicket = tickets(&server)
        .get_trouble_ticket("t-1", &Query::new())
        .await
        .expect("the seeded ticket is there");

    let status = ticket.status.expect("a status");
    assert_eq!(
        status,
        TroubleTicketStatus::Other("awaitingFieldVisit".into())
    );
    assert!(
        !status.is_terminal(),
        "a client polling to completion must not stop on an unrecognised status"
    );
}

#[tokio::test]
async fn resolving_a_ticket_keeps_the_members_it_does_not_name() {
    let server = MockTmfServer::new();
    server.seed(
        "troubleTicket",
        json!({
            "id": "t-2",
            "@type": "TroubleTicket",
            "description": "keep me",
            "severity": "minor",
            "status": "inProgress",
        }),
    );

    let updated = tickets(&server)
        .update_trouble_ticket(
            "t-2",
            &TroubleTicketUpdate::builder()
                .status(TroubleTicketStatus::Resolved)
                .status_change_reason("cable spliced")
                .build(),
        )
        .await
        .expect("the patch applies");

    assert_eq!(updated.status, Some(TroubleTicketStatus::Resolved));
    assert_eq!(updated.description.as_deref(), Some("keep me"));
    assert_eq!(updated.severity.as_deref(), Some("minor"));
}

/// TMF621 raises two event kinds nothing else in the crate does — and one of
/// them is spelled `Status`, not `State`.
#[tokio::test]
async fn the_two_tmf621_event_kinds_are_derivable() {
    let server = MockTmfServer::new();
    let client = tickets(&server);

    for (kind, expected) in [
        (EventKind::Resolved, "TroubleTicketResolvedEvent"),
        (EventKind::StatusChange, "TroubleTicketStatusChangeEvent"),
    ] {
        let hub = client
            .register_listener(&HubCreate::for_resource::<TroubleTicket>(
                "https://me/callback",
                kind,
            ))
            .await
            .expect("the hub accepts the subscription");

        assert_eq!(
            hub.query.as_deref(),
            Some(&*format!("eventType={expected}"))
        );
    }
}

/// `StatusChangeEvent` and `StateChangeEvent` are different suffixes, and
/// neither may be classified as the other.
#[test]
fn status_change_is_not_state_change() {
    assert_eq!(
        EventKind::from_event_name("TroubleTicketStatusChangeEvent"),
        Some(EventKind::StatusChange)
    );
    assert_eq!(
        EventKind::from_event_name("ProductOfferingStateChangeEvent"),
        Some(EventKind::StateChange)
    );
    assert_eq!(
        EventKind::from_event_name("TroubleTicketResolvedEvent"),
        Some(EventKind::Resolved)
    );
}
