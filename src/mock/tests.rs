//! Tests for the mock's own surface.
//!
//! The TMF630 semantics it serves are [`crate::server`]'s, and are tested
//! there and by `tests/server.rs`; what is mock-specific is seeding, the
//! notification recorder, and the [`Transport`](crate::api::Transport) shim.

use http::{Method, StatusCode};
use serde_json::json;

use super::*;

fn server_with_hubs() -> MockTmfServer {
    let server = MockTmfServer::new();
    server.seed_all(
        "hub",
        [
            json!({
                "id": "h-create",
                "callback": "https://me/created",
                "query": "eventType=ProductOfferingCreateEvent",
            }),
            json!({"id": "h-all", "callback": "https://me/everything"}),
        ],
    );
    server
}

#[test]
fn clones_share_one_server() {
    let server = MockTmfServer::new();
    let other = server.clone();
    other.seed("thing", json!({"id": "1"}));

    assert_eq!(server.collection("thing").len(), 1);
}

#[test]
fn a_hub_with_no_query_receives_everything() {
    let server = server_with_hubs();

    let matched = server.emit(&json!({"eventType": "ProductOfferingDeleteEvent"}));

    assert_eq!(matched, 1);
    assert_eq!(server.notifications()[0].hub_id, "h-all");
}

#[test]
fn a_hub_query_is_read_as_the_tmf630_filter_it_is() {
    let server = server_with_hubs();

    let matched = server.emit(&json!({"eventType": "ProductOfferingCreateEvent"}));

    assert_eq!(matched, 2, "the filtered hub and the catch-all both match");
    let notifications = server.notifications();
    let ids: Vec<&str> = notifications.iter().map(|n| n.hub_id.as_str()).collect();
    assert!(ids.contains(&"h-create"));
    assert!(ids.contains(&"h-all"));
}

#[test]
fn clearing_empties_the_store_and_the_notifications() {
    let server = server_with_hubs();
    server.emit(&json!({"eventType": "AnyEvent"}));
    assert_ne!(server.notifications().len(), 0);

    server.clear();

    assert_eq!(server.notifications(), Vec::<Notification>::new());
    assert_eq!(server.collection("hub"), Vec::<Value>::new());
}

/// Writes through the API notify on their own.
#[tokio::test]
async fn a_write_through_the_api_notifies_the_subscriptions_that_asked() {
    let server = MockTmfServer::new();
    server.seed(
        "hub",
        json!({"id": "h1", "callback": "https://me/tmf",
               "query": "eventType=ProductOfferingCreateEvent"}),
    );
    let transport = server.transport();

    let mut create = TmfRequest::new(Method::POST, server.url_for("productOffering"));
    create.body = Some(bytes::Bytes::from(
        serde_json::to_vec(&json!({"id": "7655", "name": "Firewall"})).expect("serialises"),
    ));
    transport.execute(create).await.expect("the create lands");

    let notifications = server.notifications();
    assert_eq!(
        notifications.len(),
        1,
        "the subscription asked for exactly this"
    );
    assert_eq!(notifications[0].event_type, "ProductOfferingCreateEvent");
    assert_eq!(
        notifications[0].delivery_url(),
        "https://me/tmf/listener/productOfferingCreateEvent",
    );

    // The envelope is one the crate's own client half can read.
    let event: crate::core::TmfEvent =
        serde_json::from_value(notifications[0].event.clone()).expect("a TMF event");
    assert_eq!(event.resource_key(), Some("productOffering"));
}

/// TMF630 raises `…StateChangeEvent` for a lifecycle move and
/// `…AttributeValueChangeEvent` for anything else, and a client subscribes to
/// them separately — one kind for both would deliver every move to the wrong
/// subscription.
#[tokio::test]
async fn a_patch_reports_a_lifecycle_move_apart_from_an_ordinary_edit() {
    let server = MockTmfServer::new();
    server.seed("hub", json!({"id": "h1", "callback": "https://me/tmf"}));
    server.seed(
        "productOffering",
        json!({"id": "1", "name": "Firewall", "lifecycleStatus": "Active"}),
    );
    let transport = server.transport();

    let patch = |body: &'static str| {
        let mut request = TmfRequest::new(Method::PATCH, server.url_for("productOffering/1"));
        request.headers.insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/merge-patch+json"),
        );
        request.body = Some(bytes::Bytes::from_static(body.as_bytes()));
        request
    };

    transport
        .execute(patch(r#"{"description":"now with more firewall"}"#))
        .await
        .expect("the edit lands");
    transport
        .execute(patch(r#"{"lifecycleStatus":"Retired"}"#))
        .await
        .expect("the retirement lands");

    let kinds: Vec<String> = server
        .notifications()
        .iter()
        .map(|n| n.event_type.clone())
        .collect();
    assert_eq!(
        kinds,
        [
            "ProductOfferingAttributeValueChangeEvent",
            "ProductOfferingStateChangeEvent",
        ]
    );
}

#[tokio::test]
async fn registering_a_subscription_is_not_itself_an_event() {
    // A `HubCreateEvent` goes to the subscription that just registered, and to
    // every other one.
    let server = MockTmfServer::new();
    server.seed("hub", json!({"id": "h1", "callback": "https://me/tmf"}));

    let mut create = TmfRequest::new(Method::POST, server.url_for("hub"));
    create.body = Some(bytes::Bytes::from_static(
        br#"{"callback":"https://me/other"}"#,
    ));
    server
        .transport()
        .execute(create)
        .await
        .expect("the subscription is stored");

    assert_eq!(server.collection("hub").len(), 2);
    assert!(
        server.notifications().is_empty(),
        "registering a listener is not a domain event"
    );
}

#[tokio::test]
async fn a_delete_notifies_with_the_address_of_what_is_gone() {
    let server = MockTmfServer::new();
    server.seed("hub", json!({"id": "h1", "callback": "https://me/tmf"}));
    server.seed("productOffering", json!({"id": "1", "name": "Firewall"}));

    server
        .transport()
        .execute(TmfRequest::new(
            Method::DELETE,
            server.url_for("productOffering/1"),
        ))
        .await
        .expect("the delete lands");

    let notifications = server.notifications();
    assert_eq!(notifications[0].event_type, "ProductOfferingDeleteEvent");
    assert_eq!(
        notifications[0].event["event"]["productOffering"]["id"], "1",
        "a delete event still names what went away"
    );
}

#[tokio::test]
async fn nothing_is_delivered_when_nobody_subscribed() {
    let server = MockTmfServer::new();
    let mut create = TmfRequest::new(Method::POST, server.url_for("productOffering"));
    create.body = Some(bytes::Bytes::from_static(br#"{"name":"Firewall"}"#));

    server.transport().execute(create).await.expect("created");

    assert_eq!(server.notifications(), Vec::<Notification>::new());
}

#[tokio::test]
async fn the_transport_routes_into_the_handler() {
    let server = MockTmfServer::new();
    let transport = server.transport();

    let mut request = TmfRequest::new(Method::POST, server.url_for("productOffering"));
    request.body = Some(bytes::Bytes::from(
        serde_json::to_vec(&json!({"name": "Basic Firewall"})).expect("body serialises"),
    ));

    let created = transport
        .execute(request)
        .await
        .expect("the mock never fails at the transport level");

    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(server.collection("productOffering").len(), 1);
}

#[tokio::test]
async fn expect_error_reports_the_client_level_error() {
    let server = MockTmfServer::new();

    let error = server
        .expect_error(&TmfRequest::new(
            Method::GET,
            server.url_for("productOffering/does-not-exist"),
        ))
        .await;

    assert!(error.is_not_found());
    assert_eq!(
        error.tmf_error().and_then(|e| e.code.as_deref()),
        Some("40401")
    );
}

#[test]
fn a_hand_written_base_url_loses_its_trailing_slash() {
    let server = MockTmfServer::with_base_url("http://host/tmf-api/v5/");
    assert_eq!(server.base_url(), "http://host/tmf-api/v5");
    assert_eq!(server.url_for("/thing"), "http://host/tmf-api/v5/thing");
}

/// The mock's `expect_error` must agree with the real client, not approximate it.
///
/// Every member of `TmfError` is optional, so *any* JSON object deserialises into
/// one — which is why the client checks `is_populated()` first. A second
/// interpretation here would hand a test a different error shape from the one
/// production sees.
#[tokio::test]
async fn expect_error_agrees_with_the_client_on_the_same_response() {
    use crate::api::{Query, tmf620::ProductCatalogClient};

    let server = MockTmfServer::new();
    let request = TmfRequest::new(Method::GET, server.url_for("productOffering/nope"));

    let direct = server.expect_error(&request).await;
    let through_client = ProductCatalogClient::new(server.base_url(), server.transport())
        .expect("a base URL")
        .get_product_offering("nope", &Query::new())
        .await
        .expect_err("there is no such offering");

    assert_eq!(
        format!("{direct}"),
        format!("{through_client}"),
        "the two paths must interpret one response the same way"
    );
    assert_eq!(
        direct.tmf_error().and_then(|e| e.code.as_deref()),
        Some("40401"),
        "the handler's TMF630 body must survive as data"
    );
}

/// The unscoped mock serves any collection with every operation, which is what
/// lets one mock stand in for several APIs in a client test.
#[tokio::test]
async fn an_unscoped_mock_accepts_any_operation() {
    let server = MockTmfServer::new();
    let mut create = TmfRequest::new(Method::POST, server.url_for("customerBill"));
    create.headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/json"),
    );
    create.body = Some(bytes::Bytes::from_static(br#"{"@type":"CustomerBill"}"#));
    let response = server.transport().execute(create).await.expect("answered");
    assert_eq!(response.status, StatusCode::CREATED);
}

/// A mock scoped to one API refuses what that API does not declare, exactly as
/// a server for it would.
///
/// declared-operations/SR-006: the only way to an open surface is the unscoped
/// mock above; `TmfHandler` has no public constructor without an `Api`.
#[cfg(feature = "api-tmf678")]
#[tokio::test]
async fn a_mock_scoped_to_tmf678_refuses_creating_a_bill() {
    let server = MockTmfServer::for_api(&crate::api::tmf678::API);
    let mut create = TmfRequest::new(Method::POST, server.url_for("customerBill"));
    create.body = Some(bytes::Bytes::from_static(b"{}"));
    let response = server.transport().execute(create).await.expect("answered");
    assert_eq!(response.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(
        response
            .headers
            .get(http::header::ALLOW)
            .map(|v| v.to_str().unwrap()),
        Some("GET")
    );
    assert_eq!(server.collection("customerBill"), Vec::<Value>::new());
}
