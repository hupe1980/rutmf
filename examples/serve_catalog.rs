//! Serve a TMF620 Product Catalog: implement five methods, get TMF630 free.
//!
//! The store below knows nothing about HTTP. It does not parse a query string,
//! set a status code, choose between `200` and `206`, or apply a JSON Patch —
//! [`TmfHandler`] does all of that, the same way it does for every other TM
//! Forum API, because TMF630 defines it once.
//!
//! ```console
//! cargo run --example serve_catalog --features server-axum,api-tmf620,transport-reqwest
//! ```
//!
//! The example serves the API on an ephemeral port and then calls it with this
//! crate's own client, so both ends are exercised in one process.

use std::sync::Mutex;

use serde_json::{Value, json};

use rutmf::api::{
    FilterOp, HubCreate, HubOps, Query, ReqwestTransport, tmf620::ProductCatalogClient,
};
use rutmf::core::{EventKind, Timestamp};
use rutmf::product::{ProductOffering, ProductOfferingCreate, ProductOfferingUpdate};
use rutmf::server::{
    Listener, Matched, Notifier, ResourceStore, Selection, StoreError, StoreResult, TmfHandler,
};

/// A catalog held in a `Vec`. Swap this for a database and nothing above it
/// changes — that is the point of the trait.
#[derive(Default)]
struct Catalog {
    offerings: Mutex<Vec<Value>>,
    /// Subscriptions. `hub` is a collection like any other, so a store that
    /// restricts `has_collection` has to include it or `POST /hub` answers 404
    /// and nobody can subscribe.
    hubs: Mutex<Vec<Value>>,
}

impl Catalog {
    /// The `Vec` behind a collection name.
    fn table(&self, collection: &str) -> &Mutex<Vec<Value>> {
        if collection == rutmf::server::HUB_COLLECTION {
            &self.hubs
        } else {
            &self.offerings
        }
    }
}

fn id_of(resource: &Value) -> Option<&str> {
    resource.get("id").and_then(Value::as_str)
}

#[async_trait::async_trait]
impl ResourceStore for Catalog {
    /// This API serves one collection. Saying so turns a request for any other
    /// into a `404` rather than an empty list — different answers, and a client
    /// can act on the difference.
    async fn has_collection(&self, collection: &str) -> bool {
        collection == "productOffering" || collection == rutmf::server::HUB_COLLECTION
    }

    async fn list(&self, collection: &str, selection: &Selection) -> StoreResult<Matched> {
        // `Selection::apply` does the filtering, sorting and paging for a store
        // that holds everything in memory. A SQL-backed store would translate
        // the selection into a `WHERE`/`ORDER BY`/`LIMIT` instead.
        Ok(selection.apply(self.table(collection).lock().unwrap().clone()))
    }

    async fn get(&self, collection: &str, id: &str) -> StoreResult<Option<Value>> {
        Ok(self
            .table(collection)
            .lock()
            .unwrap()
            .iter()
            .find(|resource| id_of(resource) == Some(id))
            .cloned())
    }

    async fn create(&self, collection: &str, resource: Value) -> StoreResult<Value> {
        // A rule the schema cannot state: this catalog's lifecycle vocabulary.
        // Requiredness is already enforced by `ProductOfferingCreate`, so what
        // is left for a store is the business logic.
        const ALLOWED: &[&str] = &["Active", "Launched", "Retired"];

        // A subscription carries none of an offering's rules, so it is stored
        // and that is all.
        if collection == rutmf::server::HUB_COLLECTION {
            self.hubs.lock().unwrap().push(resource.clone());
            return Ok(resource);
        }

        let status = resource
            .get("lifecycleStatus")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !ALLOWED.contains(&status) {
            return Err(StoreError::Invalid(format!(
                "lifecycleStatus must be one of {ALLOWED:?}, got {status:?}"
            )));
        }

        let name = resource.get("name").and_then(Value::as_str).unwrap_or("");
        let mut held = self.offerings.lock().unwrap();
        if held
            .iter()
            .any(|o| o.get("name").and_then(Value::as_str) == Some(name))
        {
            return Err(StoreError::Conflict(format!(
                "an offering named {name:?} already exists"
            )));
        }

        held.push(resource.clone());
        Ok(resource)
    }

    async fn replace(
        &self,
        collection: &str,
        id: &str,
        resource: Value,
    ) -> StoreResult<Option<Value>> {
        // The handler already read the resource, applied the patch — merge or
        // RFC 6902, whichever the request asked for — and hands back the
        // result. A store never sees a patch document.
        let mut held = self.table(collection).lock().unwrap();
        let Some(slot) = held.iter_mut().find(|o| id_of(o) == Some(id)) else {
            return Ok(None);
        };
        *slot = resource.clone();
        Ok(Some(resource))
    }

    async fn delete(&self, collection: &str, id: &str) -> StoreResult<bool> {
        let mut held = self.table(collection).lock().unwrap();
        let before = held.len();
        held.retain(|o| id_of(o) != Some(id));
        Ok(held.len() != before)
    }
}

/// A `Notifier` is the one part of the event story a deployment supplies: the
/// handler names the event, matches it against each subscription's filter and
/// works out where it goes — this just has to deliver it. Here "delivering" is a
/// `println!`, so the example needs no second server.
struct Announce;

#[async_trait::async_trait]
impl Notifier for Announce {
    async fn notify(&self, listener: &Listener, event_type: &str, _event: &Value) {
        println!("  → POST {}", listener.delivery_url(event_type));
    }
}

/// Subscribe, then show which changes reach the subscription and which do not.
async fn demonstrate_notifications(
    client: &ProductCatalogClient,
) -> Result<(), Box<dyn std::error::Error>> {
    // Subscribing is the client half; the event class name is derived from the
    // type, so it cannot be misspelled into a subscription that never fires.
    client
        .register_listener(&HubCreate::for_resource::<ProductOffering>(
            "https://me/callbacks",
            EventKind::StateChange,
        ))
        .await?;
    println!("  subscribed to ProductOfferingStateChangeEvent");

    // A lifecycle move raises `…StateChangeEvent`; an ordinary edit raises
    // `…AttributeValueChangeEvent` and this subscription will not see it.
    client
        .update_product_offering(
            "1",
            &ProductOfferingUpdate::builder()
                .description("now with more firewall")
                .build(),
        )
        .await?;
    println!("  edited the description — no delivery, the filter excludes it");

    client
        .update_product_offering(
            "1",
            &ProductOfferingUpdate::builder()
                .lifecycle_status("Retired")
                .build(),
        )
        .await?;
    println!("  retired it — the `Announce` notifier above printed the delivery");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Seed a catalog and serve it.
    let catalog = Catalog::default();
    catalog.offerings.lock().unwrap().extend((1..=7).map(|i| {
        json!({
            "id": i.to_string(),
            "name": format!("Offering {i}"),
            "lifecycleStatus": if i % 2 == 0 { "Retired" } else { "Active" },
            "@type": "ProductOffering",
        })
    }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let base_url = format!("http://127.0.0.1:{port}/tmf-api/productCatalogManagement/v5");
    println!("== serving TMF620 on {base_url} ==\n");

    let app = axum::Router::new().nest(
        "/tmf-api/productCatalogManagement/v5",
        rutmf::server::router(TmfHandler::new(&base_url, catalog).with_notifier(Announce)),
    );
    tokio::spawn(async move { axum::serve(listener, app).await });

    // 2. Call it with this crate's own client. Everything below went over a
    //    real socket.
    let client = ProductCatalogClient::new(&base_url, ReqwestTransport::new()?)?;

    println!("== paging, with the count headers the store never set ==");
    let page = client
        .list_product_offerings(&Query::new().limit(3).sort("name"))
        .await?;
    println!(
        "  {} of {} offerings (206 partial: {})",
        page.result_count.unwrap_or(0),
        page.total_count.unwrap_or(0),
        page.total_count > page.result_count
    );
    for offering in &page.items {
        println!("    {}", offering.name.as_deref().unwrap_or("-"));
    }

    println!("\n== a filter operator, translated into a Selection ==");
    let active = client
        .list_product_offerings(&Query::new().filter_op("lifecycleStatus", FilterOp::Ne, "Retired"))
        .await?;
    println!("  {} not retired", active.total_count.unwrap_or(0));

    println!("\n== creating one; the handler assigns id and href ==");
    let created = client
        .create_product_offering(
            &ProductOfferingCreate::builder()
                .name("Basic Firewall for Business")
                .lifecycle_status("Active")
                .last_update("2026-08-27T00:00:00Z".parse::<Timestamp>()?)
                .build(),
        )
        .await?;
    println!("  id:   {}", created.id.as_deref().unwrap_or("-"));
    println!("  href: {}", created.href.as_deref().unwrap_or("-"));

    println!("\n== the store's own rules become TMF630 errors ==");
    match client
        .create_product_offering(
            &ProductOfferingCreate::builder()
                .name("Basic Firewall for Business")
                .lifecycle_status("Active")
                .last_update("2026-08-27T00:00:00Z".parse::<Timestamp>()?)
                .build(),
        )
        .await
    {
        Ok(_) => println!("  unexpectedly accepted a duplicate"),
        Err(error) => {
            println!("  status: {:?}", error.status());
            if let Some(body) = error.tmf_error() {
                println!(
                    "  code {} — {}",
                    body.code.as_deref().unwrap_or("-"),
                    body.reason.as_deref().unwrap_or("-")
                );
            }
        }
    }

    println!("\n== fields projection, which the store knows nothing about ==");
    let projected = client
        .list_product_offerings(&Query::new().fields(["name"]).limit(2))
        .await?;
    for offering in &projected.items {
        println!(
            "  name={:?} lifecycleStatus={:?}",
            offering.name.as_deref(),
            offering.lifecycle_status.as_deref()
        );
    }

    println!("\n== notifications, which the store also knows nothing about ==");
    demonstrate_notifications(&client).await?;

    Ok(())
}
