//! End-to-end tests for the inventory triad: TMF637 product, TMF638 service,
//! TMF639 resource.
//!
//! These three APIs are the answer to "what does this customer actually have,
//! and what is it running on". The chain they form — product realised by
//! services, services supported by resources — is what the last test walks.

#![cfg(all(
    feature = "api-tmf637",
    feature = "api-tmf638",
    feature = "api-tmf639",
    feature = "mock"
))]

use futures::StreamExt;
use serde_json::json;

use rutmf::api::{
    FilterOp, HubCreate, HubOps, Query, tmf637::ProductInventoryClient,
    tmf638::ServiceInventoryClient, tmf639::ResourceInventoryClient,
};
use rutmf::core::{EventKind, Ref, ServiceSpecification};
use rutmf::mock::MockTmfServer;
use rutmf::product::{Product, ProductCreate, ProductStatus, ProductUpdate};
use rutmf::resource::{
    Resource, ResourceAdministrativeState, ResourceAlarmStatus, ResourceCreate,
    ResourceOperationalState,
};
use rutmf::service::{Service, ServiceCreate, ServiceOperatingStatus, ServiceState, ServiceUpdate};

#[test]
fn each_client_appends_its_own_conventional_api_path() {
    let server = MockTmfServer::new();
    let products = ProductInventoryClient::from_host("http://mock.test", server.transport())
        .expect("valid host");
    let services = ServiceInventoryClient::from_host("http://mock.test", server.transport())
        .expect("valid host");
    let resources = ResourceInventoryClient::from_host("http://mock.test/", server.transport())
        .expect("valid host");

    assert_eq!(
        products.inner().base_url(),
        "http://mock.test/tmf-api/productInventory/v5"
    );
    assert_eq!(
        services.inner().base_url(),
        "http://mock.test/tmf-api/serviceInventory/v5"
    );
    // A trailing slash on the host must not produce a doubled separator.
    assert_eq!(
        resources.inner().base_url(),
        "http://mock.test/tmf-api/resourceInventoryManagement/v5"
    );
}

// --- TMF637 ----------------------------------------------------------------

#[tokio::test]
async fn creates_reads_and_patches_a_product() {
    let server = MockTmfServer::new();
    let client = ProductInventoryClient::new(server.base_url(), server.transport()).unwrap();

    let created = client
        .create_product(
            &ProductCreate::builder()
                .name("Fibre 500")
                .status(ProductStatus::Active)
                .is_customer_visible(true)
                .build(),
        )
        .await
        .expect("create succeeds");

    let id = created.id.clone().expect("the server assigns an id");
    assert_eq!(created.name.as_deref(), Some("Fibre 500"));
    assert_eq!(created.status, Some(ProductStatus::Active));

    let fetched = client.get_product(&id, &Query::new()).await.unwrap();
    assert_eq!(fetched.id, created.id);

    let suspended = client
        .update_product(
            &id,
            &ProductUpdate::builder()
                .status(ProductStatus::Suspended)
                .build(),
        )
        .await
        .expect("patch succeeds");
    assert_eq!(suspended.status, Some(ProductStatus::Suspended));
    // The patch was a merge: members it did not mention survive.
    assert_eq!(suspended.name.as_deref(), Some("Fibre 500"));
}

#[tokio::test]
async fn the_order_line_type_and_the_inventory_type_are_one() {
    // TMF622 and TMF637 declare `Product` identically, so a product read out of
    // the inventory can be handed straight back to an order line. If these ever
    // become two types, this stops compiling.
    let server = MockTmfServer::new();
    server.seed(
        "product",
        json!({"id": "42", "name": "Fibre 500", "@type": "Product"}),
    );
    let client = ProductInventoryClient::new(server.base_url(), server.transport()).unwrap();

    let product: Product = client.get_product("42", &Query::new()).await.unwrap();
    let line = rutmf::order::ProductOrderItemCreate::builder()
        .id("1")
        .action(rutmf::core::ItemAction::Modify)
        .product(product)
        .build();

    assert_eq!(line.product.unwrap().name.as_deref(), Some("Fibre 500"));
}

// --- TMF638 ----------------------------------------------------------------

#[tokio::test]
async fn a_service_carries_lifecycle_and_operation_apart() {
    let server = MockTmfServer::new();
    let client = ServiceInventoryClient::new(server.base_url(), server.transport()).unwrap();

    let created = client
        .create_service(
            &ServiceCreate::builder()
                .state(ServiceState::Active)
                .service_specification(Ref::<ServiceSpecification>::new("SS-1"))
                .name("Broadband access")
                .build(),
        )
        .await
        .expect("create succeeds");
    let id = created.id.clone().expect("the server assigns an id");
    assert_eq!(created.state, Some(ServiceState::Active));
    assert!(created.operating_status.is_none());

    // The network reports trouble without changing the lifecycle state.
    let degraded = client
        .update_service(
            &id,
            &ServiceUpdate::builder()
                .operating_status(ServiceOperatingStatus::Degraded)
                .build(),
        )
        .await
        .expect("patch succeeds");

    assert_eq!(degraded.state, Some(ServiceState::Active));
    assert_eq!(
        degraded.operating_status,
        Some(ServiceOperatingStatus::Degraded)
    );
}

#[tokio::test]
async fn filters_services_by_operating_status() {
    let server = MockTmfServer::new();
    server.seed_all(
        "service",
        [
            json!({"id": "1", "operatingStatus": "running"}),
            json!({"id": "2", "operatingStatus": "degraded"}),
            json!({"id": "3", "operatingStatus": "failed"}),
        ],
    );
    let client = ServiceInventoryClient::new(server.base_url(), server.transport()).unwrap();

    let unhealthy: Vec<Service> = client
        .stream_services(Query::new().filter_op("operatingStatus", FilterOp::Ne, "running"))
        .map(|service| service.expect("page decodes"))
        .collect()
        .await;

    let mut ids: Vec<_> = unhealthy
        .iter()
        .map(|s| s.id.clone().unwrap_or_default())
        .collect();
    ids.sort();
    assert_eq!(ids, ["2", "3"]);
}

// --- TMF639 ----------------------------------------------------------------

#[tokio::test]
async fn a_resource_is_its_own_patch_body() {
    // TMF639 declares no `Resource_MVO`, so `update_resource` takes a
    // `Resource`. This test exists to pin that down: if the signature ever
    // needs a separate type, it stops compiling.
    let server = MockTmfServer::new();
    let client = ResourceInventoryClient::new(server.base_url(), server.transport()).unwrap();

    let created = client
        .create_resource(
            &ResourceCreate::builder()
                .name("Optical port 3/1/2")
                .operational_state(ResourceOperationalState::Enabled)
                .administrative_state(ResourceAdministrativeState::Unlocked)
                .build(),
        )
        .await
        .expect("create succeeds");
    let id = created.id.clone().expect("the server assigns an id");

    let locked = client
        .update_resource(
            &id,
            &Resource::builder()
                .administrative_state(ResourceAdministrativeState::Locked)
                .alarm_status(vec![ResourceAlarmStatus::Minor])
                .build(),
        )
        .await
        .expect("patch succeeds");

    // Administrative and operational state are independent: locking the port
    // does not claim it stopped working.
    assert_eq!(
        locked.administrative_state,
        Some(ResourceAdministrativeState::Locked)
    );
    assert_eq!(
        locked.operational_state,
        Some(ResourceOperationalState::Enabled)
    );
    assert_eq!(
        locked.alarm_status.as_deref(),
        Some([ResourceAlarmStatus::Minor].as_slice())
    );
}

// --- the chain -------------------------------------------------------------

#[tokio::test]
async fn a_product_reaches_its_services_and_their_resources() {
    let server = MockTmfServer::new();
    server.seed(
        "resource",
        json!({"id": "R1", "name": "Optical port 3/1/2", "@type": "Resource"}),
    );
    server.seed(
        "service",
        json!({
            "id": "S1",
            "name": "Broadband access",
            "state": "active",
            "supportingResource": [{"id": "R1", "@type": "ResourceRef"}],
            "@type": "Service"
        }),
    );
    server.seed(
        "product",
        json!({
            "id": "P1",
            "name": "Fibre 500",
            "status": "active",
            "realizingService": [{"id": "S1", "@type": "ServiceRef"}],
            "@type": "Product"
        }),
    );

    let products = ProductInventoryClient::new(server.base_url(), server.transport()).unwrap();
    let services = ServiceInventoryClient::new(server.base_url(), server.transport()).unwrap();
    let resources = ResourceInventoryClient::new(server.base_url(), server.transport()).unwrap();

    let product = products.get_product("P1", &Query::new()).await.unwrap();
    let service_id = &product.realizing_service.as_ref().unwrap()[0].id;

    let service = services
        .get_service(service_id, &Query::new())
        .await
        .unwrap();
    let resource_id = &service.supporting_resource.as_ref().unwrap()[0].id;

    let resource = resources
        .get_resource(resource_id, &Query::new())
        .await
        .unwrap();

    assert_eq!(product.name.as_deref(), Some("Fibre 500"));
    assert_eq!(service.name.as_deref(), Some("Broadband access"));
    assert_eq!(resource.name.as_deref(), Some("Optical port 3/1/2"));
}

#[tokio::test]
async fn every_inventory_client_subscribes_the_same_way() {
    let server = MockTmfServer::new();
    let products = ProductInventoryClient::new(server.base_url(), server.transport()).unwrap();
    let services = ServiceInventoryClient::new(server.base_url(), server.transport()).unwrap();
    let resources = ResourceInventoryClient::new(server.base_url(), server.transport()).unwrap();

    for (callback, expected) in [
        (
            products
                .register_listener(&HubCreate::for_resource::<Product>(
                    "https://me/cb",
                    EventKind::StateChange,
                ))
                .await
                .unwrap(),
            "eventType=ProductStateChangeEvent",
        ),
        (
            services
                .register_listener(&HubCreate::for_resource::<Service>(
                    "https://me/cb",
                    EventKind::AttributeValueChange,
                ))
                .await
                .unwrap(),
            "eventType=ServiceAttributeValueChangeEvent",
        ),
        (
            resources
                .register_listener(&HubCreate::for_resource::<Resource>(
                    "https://me/cb",
                    EventKind::Create,
                ))
                .await
                .unwrap(),
            "eventType=ResourceCreateEvent",
        ),
    ] {
        // The event class name is derived from the resource type, so it cannot
        // be misspelled into a subscription that silently delivers nothing.
        assert_eq!(callback.query.as_deref(), Some(expected));
    }
}
