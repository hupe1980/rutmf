//! The TMF630 behaviours that live in the client rather than in a resource.
//!
//! Filtering, sorting, paging and patching are defined once by TMF630 and
//! inherited by every API, so they are tested once here — end to end, through
//! the mock server, rather than as unit tests of the pieces.

#![cfg(all(feature = "api-tmf620", feature = "mock"))]

use futures::StreamExt;
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use serde_json::json;

use rutmf::api::{
    FilterOp, JsonPatchOp, Page, Patch, Query, TmfRequest, TmfResponse, Transport,
    tmf620::ProductCatalogClient,
};
use rutmf::mock::MockTmfServer;
use rutmf::product::{ProductOffering, ProductOfferingUpdate};

fn seeded() -> (MockTmfServer, ProductCatalogClient) {
    let server = MockTmfServer::new();
    server.seed_all(
        "productOffering",
        (1..=5).map(|i| {
            json!({
                "id": i.to_string(),
                "name": format!("Offering {i}"),
                "lifecycleStatus": if i % 2 == 0 { "Retired" } else { "Active" },
                "version": i.to_string(),
                "@type": "ProductOffering",
            })
        }),
    );
    let client = ProductCatalogClient::new(server.base_url(), server.transport())
        .expect("the mock's own base URL is valid");
    (server, client)
}

#[tokio::test]
async fn a_comparison_filter_bounds_a_range() {
    let (_server, client) = seeded();

    let page = client
        .list_product_offerings(
            &Query::new()
                .filter_op("version", FilterOp::Gte, "2")
                .filter_op("version", FilterOp::Lte, "4"),
        )
        .await
        .unwrap();

    let ids: Vec<&str> = page.items.iter().filter_map(|o| o.id.as_deref()).collect();
    assert_eq!(ids, ["2", "3", "4"]);
}

#[tokio::test]
async fn alternatives_match_any_listed_value() {
    let (_server, client) = seeded();

    let page = client
        .list_product_offerings(&Query::new().filter_any("id", ["1", "5"]))
        .await
        .unwrap();

    assert_eq!(page.len(), 2);
}

#[tokio::test]
async fn a_repeated_filter_widens_instead_of_replacing() {
    // The previous `Query` kept one value per attribute, so the first call was
    // silently discarded and the result was quietly wrong.
    let (_server, client) = seeded();

    let page = client
        .list_product_offerings(
            &Query::new()
                .filter("lifecycleStatus", "Active")
                .filter("lifecycleStatus", "Retired"),
        )
        .await
        .unwrap();

    assert_eq!(page.len(), 5, "both statuses, not just the last one");
}

#[tokio::test]
async fn sorting_orders_the_collection() {
    let (_server, client) = seeded();

    let page = client
        .list_product_offerings(&Query::new().sort("-name"))
        .await
        .unwrap();

    assert_eq!(page.items[0].name.as_deref(), Some("Offering 5"));
}

#[tokio::test]
async fn a_partial_page_is_reported_as_206_with_counts() {
    let (server, client) = seeded();

    let page = client
        .list_product_offerings(&Query::new().limit(2))
        .await
        .unwrap();

    assert_eq!(page.len(), 2);
    assert_eq!(page.total_count, Some(5));
    assert_eq!(page.result_count, Some(2));
    assert!(page.has_more(2));

    // `Page` does not carry the status, so the name of this test was a claim
    // nothing checked — deleting the 206 branch from the handler left every
    // assertion above passing. Go to the wire for it.
    let mut request = TmfRequest::new(Method::GET, server.url_for("productOffering"));
    request.query.insert("limit".to_owned(), "2".to_owned());
    let response = server.transport().execute(request).await.unwrap();

    assert_eq!(response.status, StatusCode::PARTIAL_CONTENT);
}

#[tokio::test]
async fn a_complete_page_is_200_not_206() {
    // The other half of the rule: 206 has to mean something, so a page that is
    // the whole collection must not claim to be partial.
    let (server, _client) = seeded();

    let mut request = TmfRequest::new(Method::GET, server.url_for("productOffering"));
    request.query.insert("limit".to_owned(), "50".to_owned());
    let response = server.transport().execute(request).await.unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.total_count(), Some(5));
}

#[tokio::test]
async fn the_last_page_of_a_walk_is_200() {
    // offset=4, limit=2 returns the fifth of five: nothing follows it.
    let (server, _client) = seeded();

    let mut request = TmfRequest::new(Method::GET, server.url_for("productOffering"));
    request.query.insert("offset".to_owned(), "4".to_owned());
    request.query.insert("limit".to_owned(), "2".to_owned());
    let response = server.transport().execute(request).await.unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.result_count(), Some(1));
}

#[tokio::test]
async fn streaming_walks_every_page() {
    let (_server, client) = seeded();

    let ids: Vec<String> = client
        .stream_product_offerings(Query::new().limit(2))
        .map(|offering| offering.unwrap().id.unwrap_or_default())
        .collect()
        .await;

    assert_eq!(ids, ["1", "2", "3", "4", "5"]);
}

/// A server that fulfils writes asynchronously, as every v5 `POST` and `PATCH`
/// permits.
struct AsyncWrites;

#[async_trait::async_trait]
impl Transport for AsyncWrites {
    async fn execute(&self, _: TmfRequest) -> rutmf::api::Result<TmfResponse> {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::LOCATION,
            HeaderValue::from_static("https://host/tmf-api/v5/importJob/42"),
        );
        Ok(TmfResponse::new(
            StatusCode::ACCEPTED,
            headers,
            bytes::Bytes::new(),
        ))
    }
}

#[tokio::test]
async fn an_asynchronous_write_is_reported_as_accepted() {
    // Feeding a 202's empty body to serde produced `invalid type: null,
    // expected struct ProductOffering`, which says nothing about what happened.
    let client = ProductCatalogClient::new("https://host/tmf-api/v5", AsyncWrites).unwrap();

    let error = client
        .create_product_offering(
            &rutmf::product::ProductOfferingCreate::builder()
                .name("Business Internet")
                .lifecycle_status("Active")
                .last_update(chrono::Utc::now())
                .build(),
        )
        .await
        .unwrap_err();

    assert!(error.is_accepted());
    assert_eq!(
        error.monitor(),
        Some("https://host/tmf-api/v5/importJob/42")
    );
    assert_eq!(error.status(), Some(StatusCode::ACCEPTED));

    // A patch answers the same way.
    let patched = client
        .update_product_offering("1", &ProductOfferingUpdate::builder().name("x").build())
        .await
        .unwrap_err();
    assert!(patched.is_accepted());
}

/// A server that pages by **cursor**: the `Link` header is the only way
/// forward, `offset` is ignored, and there is no `X-Total-Count`.
///
/// TMF630 permits this, and it is the case that breaks a client which treats
/// the link as a mere "there is more" flag and then re-derives an offset
/// request — it would be served the first page forever.
struct CursorPaging;

#[async_trait::async_trait]
impl Transport for CursorPaging {
    async fn execute(&self, request: TmfRequest) -> rutmf::api::Result<TmfResponse> {
        let cursor: usize = request
            .url
            .rsplit_once("cursor=")
            .and_then(|(_, c)| c.parse().ok())
            .unwrap_or(0);

        let mut headers = HeaderMap::new();
        let body = if cursor < 3 {
            if cursor < 2 {
                headers.insert(
                    http::header::LINK,
                    HeaderValue::from_str(&format!(
                        "<https://host/productOffering?cursor={}>; rel=\"next\"",
                        cursor + 1
                    ))
                    .expect("a well-formed header"),
                );
            }
            json!([{"id": cursor.to_string(), "@type": "ProductOffering"}])
        } else {
            json!([])
        };

        Ok(TmfResponse::new(
            StatusCode::OK,
            headers,
            bytes::Bytes::from(serde_json::to_vec(&body).unwrap()),
        ))
    }
}

#[tokio::test]
async fn a_cursor_paging_server_is_followed_by_its_links() {
    let client = ProductCatalogClient::new("https://host/tmf-api/v5", CursorPaging).unwrap();

    let ids: Vec<String> = client
        .stream_product_offerings(Query::new().limit(10))
        .map(|offering| offering.unwrap().id.unwrap_or_default())
        .collect()
        .await;

    // Each page is short, so the fallback heuristic alone would stop after the
    // first; and each cursor is opaque, so re-deriving an offset would fetch
    // page one over and over.
    assert_eq!(ids, ["0", "1", "2"]);
}

/// A cursor-paging server whose last page is exactly full and carries no link.
struct FullLastPage;

#[async_trait::async_trait]
impl Transport for FullLastPage {
    async fn execute(&self, request: TmfRequest) -> rutmf::api::Result<TmfResponse> {
        let second = request.url.contains("cursor=1");
        let mut headers = HeaderMap::new();
        if !second {
            headers.insert(
                http::header::LINK,
                HeaderValue::from_static("<https://host/productOffering?cursor=1>; rel=\"next\""),
            );
        }
        // Both pages are exactly the requested size, so the short-page
        // heuristic never fires.
        let page = if second { ["3", "4"] } else { ["1", "2"] };
        let body = json!(page.map(|id| json!({"id": id, "@type": "ProductOffering"})));
        Ok(TmfResponse::new(
            StatusCode::OK,
            headers,
            bytes::Bytes::from(serde_json::to_vec(&body).unwrap()),
        ))
    }
}

#[tokio::test]
async fn a_full_last_page_ends_the_stream_when_the_server_stops_linking() {
    // Once the server is leading with `Link` headers, the absence of one means
    // the end. Falling back to `offset` here would re-read from the top,
    // because a cursor is not an index.
    let client = ProductCatalogClient::new("https://host/tmf-api/v5", FullLastPage).unwrap();

    let ids: Vec<String> = client
        .stream_product_offerings(Query::new().limit(2))
        .map(|offering| offering.unwrap().id.unwrap_or_default())
        .collect()
        .await;

    assert_eq!(ids, ["1", "2", "3", "4"]);
}

/// A server whose `rel="next"` link points somewhere else entirely.
struct OffOriginLink;

#[async_trait::async_trait]
impl Transport for OffOriginLink {
    async fn execute(&self, _: TmfRequest) -> rutmf::api::Result<TmfResponse> {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::LINK,
            HeaderValue::from_static("<https://attacker.example/collect>; rel=\"next\""),
        );
        Ok(TmfResponse::new(
            StatusCode::OK,
            headers,
            bytes::Bytes::from_static(br#"[{"id":"1","@type":"ProductOffering"}]"#),
        ))
    }
}

#[tokio::test]
async fn a_pagination_link_to_another_origin_is_refused() {
    // The transport attaches credentials to whatever URL it is handed, and a
    // `Link` header is written by the server. Following one off-origin would
    // hand the bearer token to whoever the server named.
    let client = ProductCatalogClient::new("https://host/tmf-api/v5", OffOriginLink).unwrap();

    let outcomes: Vec<rutmf::api::Result<ProductOffering>> = client
        .stream_product_offerings(Query::new().limit(10))
        .collect()
        .await;

    assert!(outcomes[0].is_ok(), "the first page is served normally");
    let error = outcomes[1].as_ref().unwrap_err();
    assert!(
        format!("{error}").contains("attacker.example"),
        "the refusal must name the link it declined: {error}"
    );
}

/// A server that keeps naming a page it has already served.
struct LoopingLink;

#[async_trait::async_trait]
impl Transport for LoopingLink {
    async fn execute(&self, _: TmfRequest) -> rutmf::api::Result<TmfResponse> {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::LINK,
            HeaderValue::from_static("<https://host/productOffering?cursor=same>; rel=\"next\""),
        );
        Ok(TmfResponse::new(
            StatusCode::OK,
            headers,
            bytes::Bytes::from_static(br#"[{"id":"1","@type":"ProductOffering"}]"#),
        ))
    }
}

#[tokio::test]
async fn a_repeated_next_link_does_not_stream_forever() {
    // Following links means trusting the server to advance. It might not.
    let client = ProductCatalogClient::new("https://host/tmf-api/v5", LoopingLink).unwrap();

    let ids: Vec<String> = client
        .stream_product_offerings(Query::new().limit(10))
        .map(|offering| offering.unwrap().id.unwrap_or_default())
        .collect()
        .await;

    assert_eq!(
        ids,
        ["1", "1"],
        "the repeat is served once, then the stream ends"
    );
}

#[tokio::test]
async fn a_merge_patch_changes_only_what_it_names() {
    let (server, client) = seeded();

    let updated: ProductOffering = client
        .update_product_offering(
            "1",
            &ProductOfferingUpdate::builder()
                .lifecycle_status("Retired")
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(updated.lifecycle_status.as_deref(), Some("Retired"));
    assert_eq!(updated.name.as_deref(), Some("Offering 1"), "untouched");
    assert_eq!(
        server.collection("productOffering")[0]["lifecycleStatus"],
        "Retired"
    );
}

#[tokio::test]
async fn an_operation_list_edits_in_place() {
    let (_server, client) = seeded();

    let ops = [
        JsonPatchOp::replace("/name", "Renamed"),
        JsonPatchOp::remove("/version"),
    ];
    let updated: ProductOffering = client.update_product_offering("1", &ops).await.unwrap();

    assert_eq!(updated.name.as_deref(), Some("Renamed"));
    assert_eq!(updated.version, None);
}

#[tokio::test]
async fn a_failed_test_operation_leaves_the_resource_untouched() {
    // RFC 6902 §5: a patch applies in full or not at all. Without that, a
    // conditional update that loses its race corrupts the resource.
    let (server, client) = seeded();

    let ops = [
        JsonPatchOp::replace("/name", "Renamed"),
        JsonPatchOp::test("/lifecycleStatus", "Retired"),
    ];
    let outcome: rutmf::api::Result<ProductOffering> =
        client.update_product_offering("1", &ops).await;

    assert!(outcome.is_err(), "the precondition does not hold");
    assert_eq!(
        server.collection("productOffering")[0]["name"],
        "Offering 1",
        "the earlier operation must have been rolled back"
    );
}

#[tokio::test]
async fn each_patch_flavour_sends_its_own_content_type() {
    // The pairing of body and `Content-Type` is what `Patch` exists to keep
    // together; this asserts what actually reaches the wire.
    #[derive(Clone, Default)]
    struct Recorder(std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>);

    #[async_trait::async_trait]
    impl Transport for Recorder {
        async fn execute(&self, request: TmfRequest) -> rutmf::api::Result<TmfResponse> {
            self.0.lock().unwrap().push((
                request
                    .headers
                    .get(http::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_owned(),
                String::from_utf8_lossy(&request.body.unwrap_or_default()).into_owned(),
            ));
            Ok(TmfResponse::new(
                StatusCode::OK,
                HeaderMap::new(),
                bytes::Bytes::from_static(br#"{"@type":"ProductOffering"}"#),
            ))
        }
    }

    let recorder = Recorder::default();
    let client = ProductCatalogClient::new("https://host/v5", recorder.clone()).unwrap();

    let update = ProductOfferingUpdate::builder().name("n").build();
    let ops = [JsonPatchOp::remove("/description")];

    client.update_product_offering("1", &update).await.unwrap();
    client
        .update_product_offering("1", Patch::Implicit(&update))
        .await
        .unwrap();
    client.update_product_offering("1", &ops).await.unwrap();
    client
        .update_product_offering("1", Patch::Query(&ops))
        .await
        .unwrap();

    let sent = recorder.0.lock().unwrap().clone();
    let types: Vec<&str> = sent.iter().map(|(ct, _)| ct.as_str()).collect();
    assert_eq!(
        types,
        [
            "application/merge-patch+json",
            "application/json",
            "application/json-patch+json",
            "application/json-patch-query+json",
        ]
    );

    // And the body matches the content type in every case.
    assert!(sent[0].1.starts_with('{'), "an _MVO object");
    assert!(sent[2].1.starts_with('['), "an operation list");
    assert!(sent[3].1.starts_with('['), "an operation list");
}

#[tokio::test]
async fn field_selection_keeps_the_identity_members() {
    let (_server, client) = seeded();

    let request = TmfRequest::new(Method::GET, client.inner().url("productOffering/1"));
    let _ = request; // the client builds its own; this documents the URL shape.

    let offering = client
        .get_product_offering("1", &Query::new().fields(["name"]))
        .await
        .unwrap();

    assert_eq!(offering.name.as_deref(), Some("Offering 1"));
    assert_eq!(offering.id.as_deref(), Some("1"), "id is always returned");
    assert_eq!(offering.lifecycle_status, None, "not selected");
}

#[test]
fn a_page_reports_what_the_server_said_about_the_rest() {
    let page: Page<u8> = Page::new(vec![1, 2]).with_total_count(10).with_offset(4);
    assert_eq!(page.next_offset(), 6);
    assert!(page.has_more(2));
    assert!(!page.is_empty());
}
