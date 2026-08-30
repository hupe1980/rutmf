//! End-to-end tests for the TMF622 product-ordering client.
//!
//! Ordering is where the type system does the most work: the create body drops
//! every server-owned member, the initial state is restricted to what a client
//! may ask for, and cancellation is a task rather than a state change.

#![cfg(all(feature = "api-tmf622", feature = "mock"))]

use futures::StreamExt;
use serde_json::json;

use rutmf::api::{Query, tmf622::ProductOrderClient};
use rutmf::core::{ItemAction, Ref};
use rutmf::mock::MockTmfServer;
use rutmf::order::{
    CancelProductOrderCreate, InitialProductOrderState, Note, ProductOrder, ProductOrderCreate,
    ProductOrderItemCreate, ProductOrderItemState, ProductOrderState, ProductOrderUpdate,
};
use rutmf::product::ProductOffering;

fn client() -> (MockTmfServer, ProductOrderClient) {
    let server = MockTmfServer::new();
    let client = ProductOrderClient::from_host("http://mock.test", server.transport()).unwrap();
    (server, client)
}

#[test]
fn uses_the_conventional_api_path() {
    let (_server, client) = client();
    assert_eq!(
        client.inner().base_url(),
        "http://mock.test/tmf-api/productOrderingManagement/v5"
    );
}

#[tokio::test]
async fn creates_an_order_with_line_items() {
    let (server, client) = client();

    let body = ProductOrderCreate::builder()
        .product_order_item(vec![
            ProductOrderItemCreate::add("1", Ref::<ProductOffering>::new("7655")),
            ProductOrderItemCreate::builder()
                .id("2")
                .action(ItemAction::Add)
                .product_offering(Ref::<ProductOffering>::new("7656"))
                .quantity(3)
                .build(),
        ])
        .description("Firewall for the Berlin office")
        .requested_initial_state(InitialProductOrderState::Acknowledged)
        .note(vec![Note::new("Install before end of quarter")])
        .build();

    let created = client.create_product_order(&body).await.unwrap();

    assert_eq!(
        created.description.as_deref(),
        Some("Firewall for the Berlin office")
    );
    let items = created.product_order_item.as_ref().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].quantity, Some(3));
    assert_eq!(items[0].action, Some(ItemAction::Add));
    assert_eq!(server.collection("productOrder").len(), 1);
}

#[tokio::test]
async fn create_body_omits_every_server_owned_member() {
    let (_server, client) = client();

    let body = ProductOrderCreate::builder()
        .product_order_item(vec![ProductOrderItemCreate::add(
            "1",
            Ref::<ProductOffering>::new("7655"),
        )])
        .build();

    // `state`, `creationDate`, `completionDate`, `cancellationDate` and
    // `expectedCompletionDate` are absent from the type entirely, so they
    // cannot appear on the wire even by accident.
    let wire = serde_json::to_value(&body).unwrap();
    for member in [
        "state",
        "creationDate",
        "completionDate",
        "cancellationDate",
        "cancellationReason",
        "expectedCompletionDate",
        "href",
    ] {
        assert!(
            wire.get(member).is_none(),
            "{member} must not be sent on create"
        );
    }

    let _ = client.create_product_order(&body).await.unwrap();
}

#[tokio::test]
async fn filters_and_streams_orders_by_state() {
    let (server, client) = client();
    for i in 0..9 {
        let state = if i % 3 == 0 {
            "completed"
        } else {
            "inProgress"
        };
        server.seed(
            "productOrder",
            json!({"id": i.to_string(), "state": state, "@type": "ProductOrder"}),
        );
    }

    let page = client
        .list_product_orders(&Query::new().filter("state", "completed"))
        .await
        .unwrap();
    assert_eq!(page.len(), 3);
    assert!(
        page.items
            .iter()
            .all(|o| o.state.as_ref().is_some_and(ProductOrderState::is_terminal))
    );

    let total = client
        .stream_product_orders(Query::new().limit(4))
        .count()
        .await;
    assert_eq!(total, 9);
}

#[tokio::test]
async fn patches_an_order_without_touching_server_owned_members() {
    let (server, client) = client();
    server.seed(
        "productOrder",
        json!({
            "id": "42",
            "description": "Original",
            "state": "acknowledged",
            "creationDate": "2026-01-01T00:00:00Z",
            "@type": "ProductOrder",
        }),
    );

    let update = ProductOrderUpdate::builder()
        .description("Revised")
        .state(ProductOrderState::InProgress)
        .build();

    let patched = client.update_product_order("42", &update).await.unwrap();

    assert_eq!(patched.description.as_deref(), Some("Revised"));
    assert_eq!(patched.state, Some(ProductOrderState::InProgress));
    // The patch body has no `creationDate`, so the server's value survives.
    assert!(
        patched.creation_date.is_some(),
        "server-owned member untouched"
    );
}

#[tokio::test]
async fn cancellation_is_a_task_not_a_state_change() {
    let (server, client) = client();
    server.seed(
        "productOrder",
        json!({"id": "42", "state": "inProgress", "@type": "ProductOrder"}),
    );

    let request = client
        .request_cancellation(
            &CancelProductOrderCreate::builder()
                .product_order(Ref::<ProductOrder>::new("42"))
                .cancellation_reason("Ordered in error")
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(request.product_order.as_ref().unwrap().id, "42");
    assert_eq!(
        request.cancellation_reason.as_deref(),
        Some("Ordered in error")
    );
    assert!(request.id.is_some(), "the request is itself a resource");

    // The order is untouched until the provider acts on the request.
    let order = client.get_product_order("42", &Query::new()).await.unwrap();
    assert_eq!(order.state, Some(ProductOrderState::InProgress));

    let listed = client.list_cancellations(&Query::new()).await.unwrap();
    assert_eq!(listed.len(), 1);
}

#[tokio::test]
async fn nested_bundle_items_round_trip_through_the_client() {
    let (server, client) = client();
    server.seed(
        "productOrder",
        json!({
            "id": "50",
            "@type": "ProductOrder",
            "productOrderItem": [{
                "id": "1",
                "action": "add",
                "state": "inProgress",
                "@type": "ProductOrderItem",
                "productOrderItem": [
                    {"id": "1.1", "action": "add", "@type": "ProductOrderItem"},
                    {"id": "1.2", "action": "noChange", "@type": "ProductOrderItem"}
                ]
            }]
        }),
    );

    let order = client.get_product_order("50", &Query::new()).await.unwrap();
    let parent = &order.product_order_item.as_ref().unwrap()[0];

    assert_eq!(parent.state, Some(ProductOrderItemState::InProgress));
    let children = parent.product_order_item.as_ref().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[1].action, Some(ItemAction::NoChange));
}

#[tokio::test]
async fn unknown_state_from_a_vendor_still_parses() {
    let (server, client) = client();
    server.seed(
        "productOrder",
        json!({"id": "60", "state": "awaitingSiteSurvey", "@type": "ProductOrder"}),
    );

    let order = client.get_product_order("60", &Query::new()).await.unwrap();
    let state = order.state.as_ref().unwrap();

    assert_eq!(
        *state,
        ProductOrderState::Other("awaitingSiteSurvey".into())
    );
    assert!(
        !state.is_terminal(),
        "an unknown state must not be treated as final"
    );
}
