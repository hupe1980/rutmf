//! TMF635 Product Usage end to end: the client against a mock that serves
//! exactly what TMF635 declares.

#![cfg(all(feature = "api-tmf635", feature = "mock"))]

use rutmf::api::{Error, HubCreate, HubLookup, HubOps, Query, tmf635};
use rutmf::core::Ref;
use rutmf::mock::MockTmfServer;
use rutmf::usage::{ProductUsageCreate, ProductUsageStatus, ProductUsageUpdate};
use serde_json::json;

fn client(server: &MockTmfServer) -> tmf635::ProductUsageClient {
    tmf635::ProductUsageClient::new(server.base_url(), server.transport()).expect("a base URL")
}

fn seeded() -> MockTmfServer {
    let server = MockTmfServer::for_api(&tmf635::API);
    server.seed_all(
        "productUsage",
        [
            json!({"@type": "ProductUsage", "id": "U1", "status": "rated", "usageType": "voice",
                   "usageProductPrice": [{"@type": "ProductPrice", "priceType": "usage",
                       "price": {"@type": "Price", "taxIncludedAmount": {"unit": "EUR", "value": 0.1}}}]}),
            json!({"@type": "ProductUsage", "id": "U2", "status": "validated", "usageType": "data"}),
            json!({"@type": "ProductUsage", "id": "U3", "status": "rated", "usageType": "data"}),
        ],
    );
    server
}

#[tokio::test]
async fn lists_rated_usage_with_exact_prices() {
    let server = seeded();
    let rated = client(&server)
        .list_product_usages(&Query::new().filter("status", "rated"))
        .await
        .unwrap();
    assert_eq!(rated.total_count, Some(2));
    assert!(
        rated
            .items
            .iter()
            .all(|u| u.status.as_ref().is_some_and(ProductUsageStatus::is_rated))
    );
    let price = serde_json::to_value(&rated.items[0].usage_product_price).unwrap();
    assert_eq!(price[0]["price"]["taxIncludedAmount"]["value"], json!(0.1));
}

/// The append-friendly correction: record a corrected usage, then move only the
/// status of the one it replaces.
#[tokio::test]
async fn corrects_rated_usage_by_superseding_it() {
    let server = seeded();
    let usages = client(&server);

    let corrected = usages
        .create_product_usage(
            &ProductUsageCreate::builder()
                .product(Ref::new("P-1"))
                .usage_specification(Ref::new("voice-call"))
                .usage_type("voice")
                .build(),
        )
        .await
        .unwrap();
    assert!(corrected.id.is_some());

    let superseded = usages
        .update_product_usage(
            "U1",
            &ProductUsageUpdate::builder()
                .status(ProductUsageStatus::Superseded)
                .build(),
        )
        .await
        .unwrap();
    assert_eq!(superseded.status, Some(ProductUsageStatus::Superseded));
    // Only the status moved: the rated price is still the evidence it was.
    assert_eq!(superseded.usage_product_price.map(|p| p.len()), Some(1));
}

/// A mock scoped to TMF635 refuses what TMF635 does not declare.
#[tokio::test]
async fn refuses_what_tmf635_does_not_declare() {
    let server = seeded();
    let usages = client(&server);
    let mut patch =
        rutmf::api::TmfRequest::new(http::Method::PATCH, server.url_for("exportJob/J1"));
    patch.body = Some(bytes::Bytes::from_static(b"{}"));
    let response = usages.inner().send(patch).await;
    assert!(
        matches!(response, Err(Error::Api { status, .. }) if status == http::StatusCode::METHOD_NOT_ALLOWED)
    );
}

#[tokio::test]
async fn reads_a_subscription_back_because_tmf635_declares_it() {
    let server = seeded();
    let usages = client(&server);
    let hub = usages
        .register_listener(&HubCreate::to("https://me/callback"))
        .await
        .unwrap();
    let id = hub.id.expect("an id");
    assert_eq!(
        usages.get_listener(&id).await.unwrap().id.as_deref(),
        Some(id.as_str())
    );
}

/// "Why this amount?": a bill line names the usage it was rated from, and the
/// reference resolves against TMF635.
#[tokio::test]
async fn a_bill_line_resolves_to_its_usage() {
    use rutmf::api::ResolveRef;
    use rutmf::bill::AppliedCustomerBillingRate;

    let server = seeded();
    let line = AppliedCustomerBillingRate::builder()
        .name("Voice, March")
        .build()
        .with_product_usages([Ref::new("U1")]);

    let usage = line.product_usages()[0]
        .resolve(client(&server).inner(), &Query::new())
        .await
        .unwrap();
    assert_eq!(usage.usage_type.as_deref(), Some("voice"));
}
