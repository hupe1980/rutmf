//! The server layer: a custom [`ResourceStore`] served over real HTTP, and
//! called by this crate's own client.
//!
//! The loop is the point. If the client and the server disagree about TMF630 —
//! about what `206` means, which headers carry the counts, how a `PATCH` is
//! applied — these tests fail, because one end is checking the other.

#![cfg(all(
    feature = "server-axum",
    feature = "transport-reqwest",
    feature = "api-tmf620"
))]

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::{Value, json};

use rutmf::api::{
    Conditional, Error, Query, ReqwestTransport, Tagged, Transport, tmf620::ProductCatalogClient,
};
use rutmf::product::{ProductOfferingCreate, ProductOfferingUpdate};
use rutmf::server::{Matched, ResourceStore, Selection, StoreError, StoreResult, TmfHandler};

/// A store that is not [`MemoryStore`](rutmf::server::MemoryStore), so the test
/// exercises the trait rather than the implementation that ships with it.
///
/// It also refuses collections it does not serve, and enforces a lifecycle
/// vocabulary — the two things a real store does that a bag of JSON does not.
/// Note that it cannot usefully reject a *missing* `name`: TMF620 marks it
/// required on create, so [`ProductOfferingCreate`] will not build without one.
/// A store's rules are the ones the schema cannot state.
#[derive(Default)]
struct Catalog {
    offerings: Mutex<Vec<Value>>,
}

fn id_of(resource: &Value) -> Option<&str> {
    resource.get("id").and_then(Value::as_str)
}

#[async_trait::async_trait]
impl ResourceStore for Catalog {
    async fn has_collection(&self, collection: &str) -> bool {
        collection == "productOffering"
    }

    async fn list(&self, _collection: &str, selection: &Selection) -> StoreResult<Matched> {
        Ok(selection.apply(self.offerings.lock().unwrap().clone()))
    }

    async fn get(&self, _collection: &str, id: &str) -> StoreResult<Option<Value>> {
        Ok(self
            .offerings
            .lock()
            .unwrap()
            .iter()
            .find(|item| id_of(item) == Some(id))
            .cloned())
    }

    async fn create(&self, _collection: &str, resource: Value) -> StoreResult<Value> {
        const ALLOWED: &[&str] = &["Active", "Launched", "Retired"];
        let status = resource
            .get("lifecycleStatus")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !ALLOWED.contains(&status) {
            return Err(StoreError::Invalid(format!(
                "lifecycleStatus must be one of {ALLOWED:?}, got {status:?}"
            )));
        }
        self.offerings.lock().unwrap().push(resource.clone());
        Ok(resource)
    }

    async fn replace(
        &self,
        _collection: &str,
        id: &str,
        resource: Value,
    ) -> StoreResult<Option<Value>> {
        let mut held = self.offerings.lock().unwrap();
        let Some(slot) = held.iter_mut().find(|item| id_of(item) == Some(id)) else {
            return Ok(None);
        };
        *slot = resource.clone();
        Ok(Some(resource))
    }

    async fn delete(&self, _collection: &str, id: &str) -> StoreResult<bool> {
        let mut held = self.offerings.lock().unwrap();
        let before = held.len();
        held.retain(|item| id_of(item) != Some(id));
        Ok(held.len() != before)
    }
}

/// Serves `store` on an ephemeral port and returns a client pointed at it.
async fn serve(store: Catalog) -> ProductCatalogClient {
    serve_handler(TmfHandler::new("", store)).await
}

/// Serves an already-configured handler, so a test can set a handler option.
///
/// The base URL is only known once a port is bound, so it is stamped in here
/// rather than by the caller.
async fn serve_handler(handler: TmfHandler<Catalog>) -> ProductCatalogClient {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a free port");
    let port = listener.local_addr().expect("a bound address").port();
    let base_url = format!("http://127.0.0.1:{port}/tmf-api/productCatalogManagement/v5");

    let app = axum::Router::new().nest(
        "/tmf-api/productCatalogManagement/v5",
        rutmf::server::router(handler.with_base_url(&base_url)),
    );

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("the server runs");
    });

    ProductCatalogClient::new(base_url, ReqwestTransport::new().expect("a transport"))
        .expect("a valid base URL")
}

/// A timestamp for the `lastUpdate` TMF620 requires on create.
fn now() -> rutmf::core::Timestamp {
    "2026-08-27T00:00:00Z"
        .parse()
        .expect("a valid RFC 3339 time")
}

fn seeded(count: usize) -> Catalog {
    let catalog = Catalog::default();
    catalog
        .offerings
        .lock()
        .unwrap()
        .extend((0..count).map(|i| {
            json!({
                "id": i.to_string(),
                "name": format!("Offering {i}"),
                "lifecycleStatus": if i % 2 == 0 { "Active" } else { "Retired" },
                "@type": "ProductOffering",
            })
        }));
    catalog
}

#[tokio::test]
async fn a_custom_store_round_trips_through_the_real_client() {
    let client = serve(Catalog::default()).await;

    let created = client
        .create_product_offering(
            &ProductOfferingCreate::builder()
                .name("Basic Firewall for Business")
                .lifecycle_status("Active")
                .last_update(now())
                .is_bundle(false)
                .build(),
        )
        .await
        .expect("the create succeeds");

    let id = created.id.clone().expect("the server assigned an id");
    assert_eq!(created.name.as_deref(), Some("Basic Firewall for Business"));
    // The handler stamps an absolute href so a `Ref` resolves against it.
    assert!(
        created
            .href
            .as_deref()
            .is_some_and(|href| href.ends_with(&format!("/productOffering/{id}"))),
        "href was {:?}",
        created.href
    );

    let fetched = client
        .get_product_offering(&id, &Query::new())
        .await
        .expect("the read succeeds");
    assert_eq!(fetched.id, created.id);
}

#[tokio::test]
async fn the_client_and_the_server_agree_about_paging() {
    let client = serve(seeded(10)).await;

    let page = client
        .list_product_offerings(&Query::new().limit(3))
        .await
        .expect("the list succeeds");

    // The server said 206 with the counts; the client read them.
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.total_count, Some(10));
    assert_eq!(page.result_count, Some(3));
}

#[tokio::test]
async fn a_filter_reaches_the_store_as_a_selection() {
    let client = serve(seeded(10)).await;

    let page = client
        .list_product_offerings(&Query::new().filter("lifecycleStatus", "Active"))
        .await
        .expect("the list succeeds");

    assert_eq!(page.total_count, Some(5));
    assert!(
        page.items
            .iter()
            .all(|o| o.lifecycle_status.as_deref() == Some("Active"))
    );
}

#[tokio::test]
async fn fields_projection_happens_without_the_store_knowing() {
    let client = serve(seeded(3)).await;

    let page = client
        .list_product_offerings(&Query::new().fields(["name"]))
        .await
        .expect("the list succeeds");

    for offering in &page.items {
        assert!(offering.name.is_some());
        // `id` and `@type` survive a projection; TMF630 requires them.
        assert!(offering.id.is_some());
        // Everything else was dropped by the handler, not by the store.
        assert!(offering.lifecycle_status.is_none());
    }
}

#[tokio::test]
async fn a_merge_patch_updates_only_what_it_names() {
    let client = serve(seeded(3)).await;

    let updated = client
        .update_product_offering(
            "1",
            &ProductOfferingUpdate::builder()
                .lifecycle_status("Active")
                .build(),
        )
        .await
        .expect("the patch succeeds");

    assert_eq!(updated.lifecycle_status.as_deref(), Some("Active"));
    assert_eq!(updated.name.as_deref(), Some("Offering 1"));
}

#[tokio::test]
async fn a_store_error_becomes_the_status_and_body_it_stands_for() {
    let client = serve(Catalog::default()).await;

    // The schema cannot say which lifecycle values this catalog accepts, so
    // the store does — and the handler renders the refusal as TMF630.
    let error = client
        .create_product_offering(
            &ProductOfferingCreate::builder()
                .name("Basic Firewall for Business")
                .lifecycle_status("Speculative")
                .last_update(now())
                .build(),
        )
        .await
        .expect_err("the store refuses this");

    assert_eq!(error.status(), Some(http::StatusCode::BAD_REQUEST));
    let body = error.tmf_error().expect("a TMF630 error body");
    assert_eq!(body.code.as_deref(), Some("40001"));
    assert!(
        body.reason
            .as_deref()
            .is_some_and(|reason| reason.contains("Speculative")),
        "reason was {:?}",
        body.reason
    );
}

#[tokio::test]
async fn a_collection_the_api_does_not_serve_is_a_404_not_an_empty_list() {
    let client = serve(Catalog::default()).await;

    // `has_collection` refuses everything but `productOffering`.
    let error = client
        .list_categories(&Query::new())
        .await
        .expect_err("this API serves no categories");

    assert!(error.is_not_found());
}

#[tokio::test]
async fn deleting_twice_reports_the_second_as_missing() {
    let client = serve(seeded(2)).await;

    client
        .delete_product_offering("0")
        .await
        .expect("the first delete succeeds");

    let error = client
        .delete_product_offering("0")
        .await
        .expect_err("the second finds nothing");
    assert!(error.is_not_found());
}

#[tokio::test]
async fn a_store_may_answer_202_for_an_asynchronous_write() {
    // Every v5 POST and PATCH declares 202 alongside its synchronous answer.
    // A store says so with `StoreError::Accepted`; the client reads it back as
    // `Error::Accepted` with the monitor URL. The two ends model it the same.
    struct Queueing;

    #[async_trait::async_trait]
    impl ResourceStore for Queueing {
        async fn list(&self, _c: &str, _s: &Selection) -> StoreResult<Matched> {
            Ok(Matched::complete(Vec::new()))
        }
        async fn get(&self, _c: &str, _id: &str) -> StoreResult<Option<Value>> {
            Ok(None)
        }
        async fn create(&self, _c: &str, _resource: Value) -> StoreResult<Value> {
            Err(StoreError::Accepted {
                monitor: Some("https://mycsp.com/monitor/42".to_owned()),
            })
        }
        async fn replace(&self, _c: &str, _id: &str, _r: Value) -> StoreResult<Option<Value>> {
            Ok(None)
        }
        async fn delete(&self, _c: &str, _id: &str) -> StoreResult<bool> {
            Ok(false)
        }
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let base_url = format!("http://127.0.0.1:{port}/tmf-api/productCatalogManagement/v5");
    let app = axum::Router::new().nest(
        "/tmf-api/productCatalogManagement/v5",
        rutmf::server::router(TmfHandler::new(&base_url, Queueing)),
    );
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = ProductCatalogClient::new(base_url, ReqwestTransport::new().unwrap()).unwrap();

    let error = client
        .create_product_offering(
            &ProductOfferingCreate::builder()
                .name("Anything")
                .lifecycle_status("Active")
                .last_update(now())
                .build(),
        )
        .await
        .expect_err("202 carries no resource");

    assert!(error.is_accepted(), "got {error:?}");
    assert_eq!(error.monitor(), Some("https://mycsp.com/monitor/42"));
    assert!(matches!(error, Error::Accepted { .. }));
}

#[tokio::test]
async fn a_query_string_survives_percent_encoding_end_to_end() {
    let catalog = Catalog::default();
    catalog.offerings.lock().unwrap().push(json!({
        "id": "1",
        "name": "Basic Firewall for Business",
        "@type": "ProductOffering",
    }));
    let client = serve(catalog).await;

    // A space in a filter value has to survive the client's encoding, the
    // wire, and the adapter's decoding, or the filter silently matches nothing.
    let page = client
        .list_product_offerings(&Query::new().filter("name", "Basic Firewall for Business"))
        .await
        .expect("the list succeeds");

    assert_eq!(page.items.len(), 1, "the filter value lost its spaces");
}

#[tokio::test]
async fn selection_is_what_the_store_receives() {
    // The store sees the parsed selection, not a query string — this pins that
    // down, because it is the contract a database-backed store translates.
    struct Recording(Mutex<Option<Selection>>);

    #[async_trait::async_trait]
    impl ResourceStore for Recording {
        async fn list(&self, _c: &str, selection: &Selection) -> StoreResult<Matched> {
            *self.0.lock().unwrap() = Some(selection.clone());
            Ok(Matched::complete(Vec::new()))
        }
        async fn get(&self, _c: &str, _id: &str) -> StoreResult<Option<Value>> {
            Ok(None)
        }
        async fn create(&self, _c: &str, r: Value) -> StoreResult<Value> {
            Ok(r)
        }
        async fn replace(&self, _c: &str, _id: &str, _r: Value) -> StoreResult<Option<Value>> {
            Ok(None)
        }
        async fn delete(&self, _c: &str, _id: &str) -> StoreResult<bool> {
            Ok(false)
        }
    }

    let handler = TmfHandler::new("http://host/tmf-api/v5", Recording(Mutex::new(None)));
    let mut request =
        rutmf::api::TmfRequest::new(http::Method::GET, "http://host/tmf-api/v5/productOffering");
    request.query = BTreeMap::from([
        ("lifecycleStatus".to_owned(), "Active".to_owned()),
        ("orderDate.gte".to_owned(), "2026-01-01".to_owned()),
        ("sort".to_owned(), "-name".to_owned()),
        ("offset".to_owned(), "20".to_owned()),
        ("limit".to_owned(), "10".to_owned()),
        ("fields".to_owned(), "id,name".to_owned()),
    ]);

    handler.handle(&request).await;

    let seen = handler.store().0.lock().unwrap().clone().expect("list ran");
    assert_eq!(seen.offset, 20);
    assert_eq!(seen.limit, Some(10));
    assert_eq!(seen.sort.as_deref(), Some("-name"));
    // Reserved parameters are not filters, and `fields` never reaches a store.
    assert_eq!(
        seen.filters,
        BTreeMap::from([
            ("lifecycleStatus".to_owned(), "Active".to_owned()),
            ("orderDate.gte".to_owned(), "2026-01-01".to_owned()),
        ])
    );
}

/// A `PATCH` is read-modify-write, so without a precondition two clients
/// editing different members of one resource each silently discard the other's
/// change. `If-Match` is how HTTP says "only if it still looks like what I
/// read", and the handler enforces it against the tag it issued on `GET`.
#[tokio::test]
async fn if_match_guards_a_patch_against_a_concurrent_edit() {
    use http::{Method, StatusCode, header};
    use rutmf::api::TmfRequest;

    let client = serve(seeded(1)).await;
    let raw = client.inner();
    let url = format!("{}/productOffering/0", raw.base_url());

    // Read the resource and the tag that describes it.
    let read = raw
        .send(TmfRequest::new(Method::GET, url.clone()))
        .await
        .expect("the seeded offering is there");
    let etag = read
        .headers
        .get(header::ETAG)
        .expect("a GET carries an ETag")
        .to_str()
        .expect("an ASCII tag")
        .to_owned();

    // Someone else changes it in the meantime.
    client
        .update_product_offering(
            "0",
            &ProductOfferingUpdate::builder().name("Renamed").build(),
        )
        .await
        .expect("the concurrent edit lands");

    // The original tag no longer describes the resource, so the write is
    // refused rather than silently clobbering the rename.
    let mut stale = TmfRequest::new(Method::PATCH, url.clone());
    stale.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/merge-patch+json"),
    );
    stale.headers.insert(
        header::IF_MATCH,
        header::HeaderValue::from_str(&etag).expect("a valid tag"),
    );
    stale.body = Some(br#"{"description":"late"}"#.to_vec().into());

    let refused = raw.send(stale).await.expect_err("a stale tag is refused");
    assert_eq!(refused.status(), Some(StatusCode::PRECONDITION_FAILED));

    // The rename survived: the stale write never applied.
    let current = client
        .get_product_offering("0", &Query::new())
        .await
        .expect("still there");
    assert_eq!(current.name.as_deref(), Some("Renamed"));
    assert_eq!(current.description, None, "the stale patch did not apply");

    // Re-reading yields the *current* tag, and the same write then succeeds.
    let fresh = raw
        .send(TmfRequest::new(Method::GET, url.clone()))
        .await
        .expect("still there");
    let fresh_etag = fresh
        .headers
        .get(header::ETAG)
        .expect("an ETag")
        .to_str()
        .expect("ASCII")
        .to_owned();
    assert_ne!(fresh_etag, etag, "the edit changed the tag");

    let mut retried = TmfRequest::new(Method::PATCH, url);
    retried.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/merge-patch+json"),
    );
    retried.headers.insert(
        header::IF_MATCH,
        header::HeaderValue::from_str(&fresh_etag).expect("a valid tag"),
    );
    retried.body = Some(br#"{"description":"late"}"#.to_vec().into());

    let accepted = raw.send(retried).await.expect("a current tag is accepted");
    assert_eq!(accepted.status, StatusCode::OK);
}

/// A filter naming a member of a collection matches when any element has it.
/// Most of what is worth filtering on in TM Forum is an array.
#[tokio::test]
async fn a_filter_descends_into_a_collection_end_to_end() {
    let catalog = Catalog::default();
    catalog.offerings.lock().unwrap().extend([
        json!({
            "id": "1", "name": "Bundle", "lifecycleStatus": "Active", "@type": "ProductOffering",
            "productOfferingPrice": [{"id": "p1", "name": "Monthly"}, {"id": "p2", "name": "Setup"}],
        }),
        json!({
            "id": "2", "name": "Solo", "lifecycleStatus": "Active", "@type": "ProductOffering",
            "productOfferingPrice": [{"id": "p3", "name": "Monthly"}],
        }),
    ]);

    let client = serve(catalog).await;

    let page = client
        .list_product_offerings(&Query::new().filter("productOfferingPrice.id", "p2"))
        .await
        .expect("the filter reaches the store");

    assert_eq!(page.items.len(), 1, "only the bundle carries price p2");
    assert_eq!(page.items[0].id.as_deref(), Some("1"));
}

/// A dotted `fields=` selects *into* a member rather than dropping it.
#[tokio::test]
async fn a_dotted_field_selection_narrows_a_nested_member() {
    let catalog = Catalog::default();
    catalog.offerings.lock().unwrap().push(json!({
        "id": "1", "name": "Bundle", "lifecycleStatus": "Active", "@type": "ProductOffering",
        "productSpecification": {"id": "9", "name": "Spec", "version": "1.0", "@type": "ProductSpecificationRef"},
    }));

    let client = serve(catalog).await;

    let offering = client
        .get_product_offering("1", &Query::new().fields(["productSpecification.id"]))
        .await
        .expect("the projection is applied");

    let spec = offering
        .product_specification
        .expect("the member survives the projection");
    assert_eq!(spec.id, "9");
    assert!(
        spec.extensions.get("name").is_none(),
        "only the named member is returned"
    );
}

#[tokio::test]
async fn a_jsonpath_filter_is_refused_rather_than_ignored() {
    // TMF621 and TMF639 declare a `filter` parameter holding a JSONPath
    // expression — a different mechanism from the attribute filtering every
    // other collection uses, and one this handler does not implement.
    //
    // Before `filter` was reserved, it was treated as a filter on a member
    // literally named `filter` and matched nothing. Reserving it alone would
    // have swapped that for the opposite failure: answering a request to narrow
    // a collection with the entire collection. Refusing is the honest answer.
    let client = serve(Catalog::default()).await;

    let error = client
        .list_product_offerings(&Query::new().json_path("$[?(@.isSellable==true)]"))
        .await
        .expect_err("an unsupported filter is refused");

    match error {
        Error::Api { status, .. } => assert_eq!(status, 400),
        other => panic!("expected a 400, got {other:?}"),
    }
}

#[tokio::test]
async fn cursor_parameters_are_not_treated_as_filters() {
    // The same defect one parameter over: `after` and `before` were not
    // reserved, so a server matched them against a member of that name and
    // returned nothing for TMF621's and TMF639's own cursor pagination.
    let catalog = Catalog::default();
    catalog.offerings.lock().unwrap().extend((1..=3).map(
        |i| json!({"id": i.to_string(), "lifecycleStatus": "Active", "@type": "ProductOffering"}),
    ));
    let client = serve(catalog).await;

    let page = client
        .list_product_offerings(&Query::new().after("1"))
        .await
        .expect("the cursor is honoured");

    let ids: Vec<&str> = page.items.iter().filter_map(|o| o.id.as_deref()).collect();
    assert_eq!(ids, ["2", "3"]);
}

/// The whole optimistic-concurrency exchange, over a real socket.
///
/// The client half and the server half of this crate were written against the
/// same RFC but not against each other, and every part of the loop is a place
/// they can disagree silently: a tag re-spelled on the way out, a precondition
/// attached to the wrong header, a strong comparison done weakly. Each of those
/// failures looks like success — the write lands, the response is `200`, and the
/// concurrent edit is gone. So the loop is asserted end to end rather than at
/// either end.
#[tokio::test]
async fn a_conditional_write_refuses_to_discard_a_concurrent_edit() {
    let client = serve(Catalog::default()).await;

    let created = client
        .create_product_offering(
            &ProductOfferingCreate::builder()
                .name("Basic Firewall for Business")
                .lifecycle_status("Active")
                .last_update(now())
                .build(),
        )
        .await
        .expect("the create succeeds");
    let id = created.id.clone().expect("the server assigned an id");

    // Two operators read the same offering.
    let held: Tagged<rutmf::product::ProductOffering> = client
        .inner()
        .fetch(&id, &Query::new())
        .await
        .expect("the read succeeds");
    assert!(
        held.etag().is_some(),
        "the server must issue a tag, or nothing below is conditional"
    );

    // The first one moves it to Retired, unconditionally.
    client
        .update_product_offering(
            &id,
            &ProductOfferingUpdate::builder()
                .lifecycle_status("Retired")
                .build(),
        )
        .await
        .expect("the first write succeeds");

    // The second one writes back what they read. Without the precondition this
    // succeeds and the first operator's change is gone with no error anywhere.
    let error = held
        .update(
            client.inner(),
            &ProductOfferingUpdate::builder()
                .description("Rewritten")
                .build(),
        )
        .await
        .expect_err("the stale write must be refused");
    assert!(
        error.is_precondition_failed(),
        "expected a 412, got {error:?}"
    );

    // The retired state survived, which is the whole point.
    let current = client
        .get_product_offering(&id, &Query::new())
        .await
        .expect("the read succeeds");
    assert_eq!(current.lifecycle_status.as_deref(), Some("Retired"));
    assert_eq!(current.description, None);

    // Re-reading gives a tag that works, so the loop is recoverable rather than
    // merely refusing.
    let fresh: Tagged<rutmf::product::ProductOffering> = client
        .inner()
        .fetch(&id, &Query::new())
        .await
        .expect("the re-read succeeds");
    let updated = fresh
        .update(
            client.inner(),
            &ProductOfferingUpdate::builder()
                .description("Rewritten")
                .build(),
        )
        .await
        .expect("a write against a current tag succeeds");
    assert_eq!(updated.description.as_deref(), Some("Rewritten"));
}

/// A conditional read of an unchanged resource transfers no body.
#[tokio::test]
async fn an_unchanged_resource_answers_304_over_the_wire() {
    let client = serve(seeded(1)).await;

    let held: Tagged<rutmf::product::ProductOffering> = client
        .inner()
        .fetch("0", &Query::new())
        .await
        .expect("the read succeeds");

    let unchanged: Option<Tagged<rutmf::product::ProductOffering>> = client
        .inner()
        .fetch_if_changed("0", &Query::new(), held.etag())
        .await
        .expect("the conditional read succeeds");
    assert!(unchanged.is_none(), "nothing changed, so nothing was sent");

    // After an edit the tag no longer matches and the body comes back.
    client
        .update_product_offering(
            "0",
            &ProductOfferingUpdate::builder()
                .description("Edited")
                .build(),
        )
        .await
        .expect("the write succeeds");

    let fresh = client
        .inner()
        .fetch_if_changed::<rutmf::product::ProductOffering>("0", &Query::new(), held.etag())
        .await
        .expect("the conditional read succeeds")
        .expect("the resource changed, so it was sent");
    assert_eq!(fresh.description.as_deref(), Some("Edited"));
    assert_ne!(fresh.etag(), held.etag(), "the tag must track the content");
}

/// A `limit` the server cannot read must not become "no limit".
#[tokio::test]
async fn a_malformed_paging_parameter_is_refused_rather_than_ignored() {
    let client = serve(seeded(50)).await;

    // The client layer has no way to send this — it builds `limit` from a
    // `usize` — so it goes through the transport directly, which is also how it
    // arrives in production: a template that interpolated an empty variable.
    let transport = ReqwestTransport::new().expect("a default client builds");
    for (parameter, value) in [("limit", "abc"), ("offset", "-1"), ("limit", "")] {
        let mut request = rutmf::api::TmfRequest::new(
            http::Method::GET,
            format!("{}/productOffering", client.inner().base_url()),
        );
        request.query.insert(parameter.to_owned(), value.to_owned());

        let response = transport
            .execute(request)
            .await
            .expect("the request reaches the server");

        assert_eq!(
            response.status,
            http::StatusCode::BAD_REQUEST,
            "?{parameter}={value} returned {} — reading it as `no limit` \
             answers a request for one page with the whole collection",
            response.status
        );
        let body: Value = serde_json::from_slice(&response.body).expect("a TMF error body");
        assert_eq!(body["code"], "40001");
        assert!(
            body["reason"]
                .as_str()
                .is_some_and(|r| r.contains(parameter)),
            "the error should name the parameter: {body}"
        );
    }
}

/// An unbounded collection request is bounded when the deployment says so.
#[tokio::test]
async fn a_page_cap_bounds_a_request_that_named_no_limit() {
    let client = serve_handler(TmfHandler::new("", seeded(50)).with_max_page_size(10)).await;

    // No `limit` at all: without the cap this is all fifty.
    let page = client
        .list_product_offerings(&Query::new())
        .await
        .expect("the list succeeds");
    assert_eq!(page.items.len(), 10, "the cap applies when none was asked");
    assert_eq!(
        page.total_count,
        Some(50),
        "the count still reports the whole match, so the client knows there is more"
    );

    // A larger `limit` is lowered rather than refused.
    let page = client
        .list_product_offerings(&Query::new().limit(5_000))
        .await
        .expect("the list succeeds");
    assert_eq!(page.items.len(), 10);

    // A smaller one is left alone.
    let page = client
        .list_product_offerings(&Query::new().limit(3))
        .await
        .expect("the list succeeds");
    assert_eq!(page.items.len(), 3);
}

/// A capped page keeps the client streaming rather than truncating.
///
/// The page comes back shorter than the client asked for, which on the
/// short-page heuristic alone would read as the end of the collection. This
/// handler always sends `X-Total-Count`, so that is what carries the client
/// through here — the `206` path, for a server that omits the counters, is
/// covered by `a_206_says_there_is_more_when_no_count_does`. What this asserts
/// is that the two halves agree end to end: the server marks the response
/// partial and the client reads it back.
#[tokio::test]
async fn a_capped_page_still_streams_to_the_end() {
    use futures::StreamExt as _;

    let client = serve_handler(TmfHandler::new("", seeded(25)).with_max_page_size(4)).await;

    let all: Vec<_> = client
        .stream_product_offerings(Query::new().limit(50))
        .collect()
        .await;

    assert_eq!(all.len(), 25, "the cap must not truncate the stream");
    assert!(all.into_iter().all(|item| item.is_ok()));

    // And the first page really was a 206 with fewer items than asked for.
    let page = client
        .list_product_offerings(&Query::new().limit(50))
        .await
        .expect("the list succeeds");
    assert_eq!(page.items.len(), 4);
    assert!(page.partial, "the server marked it a partial collection");
}
