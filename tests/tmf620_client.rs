//! End-to-end tests: the TMF620 client against the in-process mock server.
//!
//! These exercise the full path — query building, request dispatch, TMF630
//! collection semantics, pagination headers, patch content types and error
//! handling — without a socket.

#![cfg(all(feature = "api-tmf620", feature = "mock"))]

use chrono::{TimeZone, Utc};
use futures::StreamExt;
use serde_json::json;

use rutmf::api::{Error, JsonPatchOp, Query, ResolveRef, tmf620::ProductCatalogClient};
use rutmf::core::Ref;
use rutmf::mock::MockTmfServer;
use rutmf::product::{
    ProductOffering, ProductOfferingCreate, ProductOfferingUpdate, ProductSpecification,
};

const BASE: &str = "http://mock.test/tmf-api/productCatalogManagement/v5";

fn offering(id: &str, name: &str, status: &str) -> serde_json::Value {
    json!({
        "id": id,
        "href": format!("{BASE}/productOffering/{id}"),
        "name": name,
        "lifecycleStatus": status,
        "isSellable": true,
        "@type": "ProductOffering",
    })
}

fn seeded(count: usize) -> (MockTmfServer, ProductCatalogClient) {
    let server = MockTmfServer::new();
    for i in 0..count {
        let status = if i % 2 == 0 { "Active" } else { "Retired" };
        server.seed(
            "productOffering",
            offering(&i.to_string(), &format!("Offering {i}"), status),
        );
    }
    let client = ProductCatalogClient::new(BASE, server.transport()).expect("valid base URL");
    (server, client)
}

#[tokio::test]
async fn lists_with_pagination_headers() {
    let (_server, client) = seeded(10);

    let page = client
        .list_product_offerings(&Query::new().limit(3))
        .await
        .unwrap();

    assert_eq!(page.len(), 3);
    assert_eq!(page.total_count, Some(10));
    assert_eq!(page.result_count, Some(3));
    assert_eq!(page.next_offset(), 3);
    assert!(page.has_more(3));
}

#[tokio::test]
async fn filters_by_attribute() {
    let (_server, client) = seeded(10);

    let page = client
        .list_product_offerings(&Query::new().filter("lifecycleStatus", "Active"))
        .await
        .unwrap();

    assert_eq!(page.len(), 5);
    assert!(
        page.items
            .iter()
            .all(|o| o.lifecycle_status.as_deref() == Some("Active"))
    );
}

#[tokio::test]
async fn projects_requested_fields_but_keeps_identity() {
    let (_server, client) = seeded(1);

    let page = client
        .list_product_offerings(&Query::new().fields(["name"]))
        .await
        .unwrap();
    let item = &page.items[0];

    assert_eq!(item.name.as_deref(), Some("Offering 0"));
    assert_eq!(item.id.as_deref(), Some("0"), "id is always returned");
    // `isSellable` was not requested, so the server must not return it.
    assert_eq!(item.is_sellable, None);
}

#[tokio::test]
async fn streams_every_page() {
    let (_server, client) = seeded(25);

    let collected: Vec<ProductOffering> = client
        .stream_product_offerings(Query::new().limit(4))
        .map(|item| item.expect("stream item"))
        .collect()
        .await;

    assert_eq!(collected.len(), 25, "stream must span page boundaries");

    let mut ids: Vec<&str> = collected.iter().filter_map(|o| o.id.as_deref()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 25, "no duplicates or gaps across pages");
}

#[tokio::test]
async fn stream_terminates_on_an_exact_multiple_of_page_size() {
    // The regression this guards: a final full page must not loop forever.
    let (_server, client) = seeded(8);

    let count = client
        .stream_product_offerings(Query::new().limit(4))
        .count()
        .await;
    assert_eq!(count, 8);
}

#[tokio::test]
async fn creates_from_the_fvo_body() {
    let (server, client) = seeded(0);

    let body = ProductOfferingCreate::builder()
        .name("Business Internet")
        .lifecycle_status("Active")
        .last_update(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
        .is_sellable(true)
        .build();

    let created = client.create_product_offering(&body).await.unwrap();

    assert_eq!(created.name.as_deref(), Some("Business Internet"));
    assert!(created.id.is_some(), "server assigns an id");
    assert_eq!(server.collection("productOffering").len(), 1);
}

#[tokio::test]
async fn merge_patch_replaces_named_members_only() {
    let (_server, client) = seeded(1);

    let update = ProductOfferingUpdate::builder()
        .lifecycle_status("Retired")
        .build();
    let patched = client.update_product_offering("0", &update).await.unwrap();

    assert_eq!(patched.lifecycle_status.as_deref(), Some("Retired"));
    assert_eq!(
        patched.name.as_deref(),
        Some("Offering 0"),
        "untouched member survives"
    );
}

#[tokio::test]
async fn json_patch_operations_are_applied() {
    let (server, client) = seeded(1);

    let ops = [
        JsonPatchOp::replace("/name", "Renamed"),
        JsonPatchOp::remove("/isSellable"),
    ];
    let patched: ProductOffering = client
        .inner()
        .patch::<ProductOfferingUpdate, ProductOffering>("productOffering", "0", &ops)
        .await
        .unwrap();

    assert_eq!(patched.name.as_deref(), Some("Renamed"));
    assert_eq!(patched.is_sellable, None);
    assert_eq!(server.collection("productOffering")[0]["name"], "Renamed");
}

#[tokio::test]
async fn deletes_a_resource() {
    let (server, client) = seeded(2);

    client.delete_product_offering("0").await.unwrap();

    assert_eq!(server.collection("productOffering").len(), 1);
    assert!(
        client
            .get_product_offering("0", &Query::new())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn missing_resource_surfaces_a_typed_tmf_error() {
    let (_server, client) = seeded(1);

    let error = client
        .get_product_offering("nope", &Query::new())
        .await
        .unwrap_err();

    assert!(error.is_not_found());
    assert!(!error.is_retryable());

    let body = error.tmf_error().expect("a TMF630 error body");
    assert_eq!(body.code.as_deref(), Some("40401"));
    assert_eq!(body.http_status(), Some(404));
    assert!(matches!(error, Error::Api { .. }));
}

#[tokio::test]
async fn round_trips_a_resource_through_the_full_client_path() {
    // A payload with a vendor extension must survive create → read unchanged.
    let seeded_value = json!({
        "id": "7655",
        "name": "Basic Firewall for Business",
        "lifecycleStatus": "Active",
        "@type": "ProductOffering",
        "x-vendor-tier": {"level": 2, "tags": ["gold"]},
    });
    let server = MockTmfServer::new();
    server.seed("productOffering", seeded_value.clone());
    let client = ProductCatalogClient::new(BASE, server.transport()).unwrap();

    let fetched = client
        .get_product_offering("7655", &Query::new())
        .await
        .unwrap();

    assert_eq!(fetched.extensions.get("x-vendor-tier").unwrap()["level"], 2);
    assert_eq!(serde_json::to_value(&fetched).unwrap(), seeded_value);
}

#[tokio::test]
async fn catalog_resource_uses_the_v5_path() {
    let server = MockTmfServer::new();
    // v5 renamed the collection from `catalog` to `productCatalog`.
    server.seed(
        "productCatalog",
        json!({"id": "1", "name": "Retail", "@type": "ProductCatalog"}),
    );
    let client = ProductCatalogClient::new(BASE, server.transport()).unwrap();

    let page = client.list_product_catalogs(&Query::new()).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page.items[0].name.as_deref(), Some("Retail"));
}

#[tokio::test]
async fn resolves_a_typed_reference_to_its_resource() {
    let server = MockTmfServer::new();
    server.seed(
        "productOffering",
        json!({
            "id": "7655",
            "name": "Basic Firewall",
            "productSpecification": {
                "id": "9881",
                "@type": "ProductSpecificationRef",
                "@referredType": "ProductSpecification",
            },
            "@type": "ProductOffering",
        }),
    );
    server.seed(
        "productSpecification",
        json!({"id": "9881", "name": "Firewall Spec", "@type": "ProductSpecification"}),
    );
    let client = ProductCatalogClient::new(BASE, server.transport()).unwrap();

    let offering = client
        .get_product_offering("7655", &Query::new())
        .await
        .unwrap();

    // The reference is typed, so `resolve` returns a ProductSpecification with
    // no turbofish and no collection path at the call site.
    let spec: ProductSpecification = offering
        .product_specification
        .as_ref()
        .unwrap()
        .resolve(client.inner(), &Query::new())
        .await
        .unwrap();

    assert_eq!(spec.name.as_deref(), Some("Firewall Spec"));
}

#[tokio::test]
async fn resolve_prefers_an_absolute_href_on_the_same_origin() {
    // An href naming a *different API* must win over this client's base URL.
    // Within a deployment those differ by path, not by host, which is the case
    // that has to keep working.
    let server = MockTmfServer::new();
    server.seed(
        "productSpecification",
        json!({"id": "9881", "name": "Elsewhere", "@type": "ProductSpecification"}),
    );
    let client = ProductCatalogClient::new(BASE, server.transport()).unwrap();

    let reference = Ref::<ProductSpecification>::new("9881")
        .with_href("http://mock.test/tmf-api/otherCatalog/v5/productSpecification/9881");

    let spec = reference
        .resolve(client.inner(), &Query::new())
        .await
        .unwrap();
    assert_eq!(spec.name.as_deref(), Some("Elsewhere"));
}

#[tokio::test]
async fn resolve_refuses_an_href_that_leaves_the_origin() {
    // The attack this closes: `href` is payload data, so any `…Ref` in any
    // response is a place to put an attacker's host — and the transport
    // attaches this client's credentials to whatever URL it is handed. A
    // resolved reference would post a live bearer token to `attacker.test`.
    let server = MockTmfServer::new();
    server.seed(
        "productSpecification",
        json!({"id": "9881", "name": "Elsewhere", "@type": "ProductSpecification"}),
    );
    let client = ProductCatalogClient::new(BASE, server.transport()).unwrap();

    let reference = Ref::<ProductSpecification>::new("9881")
        .with_href("http://attacker.test/tmf-api/x/v5/productSpecification/9881");

    let error = reference
        .resolve(client.inner(), &Query::new())
        .await
        .expect_err("a cross-origin href must not be followed");
    assert!(
        matches!(&error, Error::CrossOrigin { url, .. } if url.contains("attacker.test")),
        "expected a CrossOrigin refusal, got {error:?}"
    );

    // Federation across hosts is still possible — it just has to be asked for.
    let spec = reference
        .resolve_cross_origin(client.inner(), &Query::new())
        .await
        .unwrap();
    assert_eq!(spec.name.as_deref(), Some("Elsewhere"));
}
