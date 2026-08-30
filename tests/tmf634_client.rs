//! End-to-end tests for TMF634 Resource Catalog Management.
//!
//! TMF634 is the catalog half of the resource domain: it publishes the
//! [`ResourceSpecification`]s that the TMF639 inventory then instantiates. The
//! last test walks that seam, because it is the reason the two APIs belong in
//! one module.

#![cfg(all(feature = "api-tmf634", feature = "api-tmf639", feature = "mock"))]

use serde_json::json;

use rutmf::api::{
    HubCreate, HubOps, Query, tmf634::ResourceCatalogClient, tmf639::ResourceInventoryClient,
};
use rutmf::core::EventKind;
use rutmf::mock::MockTmfServer;
use rutmf::resource::{
    Resource, ResourceCandidate, ResourceCatalogCreate, ResourceSpecification,
    ResourceSpecificationKind, ResourceSpecificationUpdate,
};

fn catalog(server: &MockTmfServer) -> ResourceCatalogClient {
    ResourceCatalogClient::new(server.base_url(), server.transport()).expect("a valid base URL")
}

#[tokio::test]
async fn a_catalog_round_trips_through_the_client() {
    let server = MockTmfServer::new();
    let client = catalog(&server);

    let created = client
        .create_resource_catalog(
            &ResourceCatalogCreate::builder()
                .name("Access Network Catalog")
                .lifecycle_status("Active")
                .build(),
        )
        .await
        .expect("the catalog is created");

    let id = created.id.clone().expect("the server assigns an id");
    let fetched = client
        .get_resource_catalog(&id, &Query::new())
        .await
        .expect("and can be read back");

    assert_eq!(fetched.name.as_deref(), Some("Access Network Catalog"));
    assert_eq!(fetched.type_name(), "ResourceCatalog");
}

#[tokio::test]
async fn a_specification_keeps_its_subclass_through_a_round_trip() {
    let server = MockTmfServer::new();
    server.seed(
        "resourceSpecification",
        json!({
            "id": "spec-1",
            "@type": "PhysicalResourceSpecification",
            "name": "SFP+ 10G Transceiver",
            "vendor": "Acme",
            "sku": "SFP-10G-LR",
            "lifecycleStatus": "Active",
        }),
    );

    let spec = catalog(&server)
        .get_resource_specification("spec-1", &Query::new())
        .await
        .expect("the seeded specification is there");

    assert_eq!(spec.kind(), ResourceSpecificationKind::Physical);
    assert_eq!(spec.vendor.as_deref(), Some("Acme"));
    assert_eq!(spec.sku.as_deref(), Some("SFP-10G-LR"));
    assert!(
        spec.extensions.is_empty(),
        "subclass members must be typed, not swept into extensions"
    );
}

#[tokio::test]
async fn a_candidate_carries_the_name_the_schema_omits() {
    let server = MockTmfServer::new();
    server.seed(
        "resourceCandidate",
        json!({
            "id": "cand-1",
            "@type": "ResourceCandidate",
            "name": "Virtual Storage Medium",
            "lifecycleStatus": "Active",
        }),
    );

    let candidate: ResourceCandidate = catalog(&server)
        .get_resource_candidate("cand-1", &Query::new())
        .await
        .expect("the seeded candidate is there");

    // TMF634 declares no `name` on this schema, but requires it on create and
    // sends it in every example. Typing it is what keeps it usable.
    assert_eq!(candidate.name.as_deref(), Some("Virtual Storage Medium"));
    assert!(candidate.extensions.is_empty());
}

#[tokio::test]
async fn a_patch_updates_only_what_it_names() {
    let server = MockTmfServer::new();
    server.seed(
        "resourceSpecification",
        json!({
            "id": "spec-2",
            "@type": "ResourceSpecification",
            "name": "Fibre Port",
            "description": "keep me",
            "lifecycleStatus": "Active",
        }),
    );

    let updated = catalog(&server)
        .update_resource_specification(
            "spec-2",
            &ResourceSpecificationUpdate::builder()
                .lifecycle_status("Retired")
                .build(),
        )
        .await
        .expect("the patch applies");

    assert_eq!(updated.lifecycle_status.as_deref(), Some("Retired"));
    assert_eq!(
        updated.description.as_deref(),
        Some("keep me"),
        "a merge patch must not clear members it does not name"
    );
}

#[tokio::test]
async fn subscribing_derives_the_event_name_from_the_type() {
    let server = MockTmfServer::new();
    let client = catalog(&server);

    let hub = client
        .register_listener(&HubCreate::for_resource::<ResourceSpecification>(
            "https://me/callback",
            EventKind::Create,
        ))
        .await
        .expect("the hub accepts the subscription");

    assert_eq!(
        hub.query.as_deref(),
        Some("eventType=ResourceSpecificationCreateEvent"),
        "the event class name is derived, not spelled out"
    );
}

/// The seam between the two halves of the resource domain: a catalog publishes
/// a specification, and an inventory resource points back at it.
///
/// This is what the TMF634 work bought that a marker type could not — before
/// it, `resource_specification` pointed at a placeholder that could never be
/// resolved into anything.
#[tokio::test]
async fn an_inventory_resource_resolves_its_catalog_specification() {
    let server = MockTmfServer::new();

    server.seed(
        "resourceSpecification",
        json!({
            "id": "spec-9",
            "@type": "LogicalResourceSpecification",
            "name": "VLAN",
            "lifecycleStatus": "Active",
        }),
    );
    server.seed(
        "resource",
        json!({
            "id": "res-1",
            "@type": "Resource",
            "name": "VLAN 42",
            "resourceSpecification": {"id": "spec-9", "@type": "ResourceSpecificationRef"},
        }),
    );

    let inventory = ResourceInventoryClient::new(server.base_url(), server.transport())
        .expect("a valid base URL");

    let resource: Resource = inventory
        .get_resource("res-1", &Query::new())
        .await
        .expect("the resource is there");

    let reference = resource
        .resource_specification
        .as_ref()
        .expect("it names a specification");
    assert_eq!(reference.id, "spec-9");

    // The reference is typed as the real TMF634 resource, so it resolves.
    let spec = catalog(&server)
        .get_resource_specification(&reference.id, &Query::new())
        .await
        .expect("and the catalog serves it");

    assert_eq!(spec.name.as_deref(), Some("VLAN"));
    assert_eq!(spec.kind(), ResourceSpecificationKind::Logical);
}
